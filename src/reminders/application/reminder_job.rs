use std::sync::Arc;
use std::{collections::HashMap, time::Duration};

use chrono::{Datelike, Local};
use tokio::time::{Instant, sleep};
use tracing::{error, info, warn};

use crate::auth::infrastructure::crypto;
use crate::birthdays::domain::repository::BirthdayRepository;
use crate::channels::domain::repository::{
    NotificationChannelRecord, NotificationChannelRepository,
};
use crate::channels::infrastructure::dispatcher;
use crate::channels::infrastructure::signal::SignalRuntimeConfig;
use crate::reminders::domain::reminder::ReminderPolicy;
use crate::reminders::domain::services::compute_due_reminders;
use crate::users::domain::repository::UserRepository;
use crate::users::domain::user::UserId;

pub struct ReminderJobService {
    user_repo: Arc<dyn UserRepository>,
    birthday_repo: Arc<dyn BirthdayRepository>,
    notification_repo: Arc<dyn NotificationChannelRepository>,
    default_days_before: Vec<u32>,
    encryption_key: String,
    signal_runtime: SignalRuntimeConfig,
}

impl ReminderJobService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        birthday_repo: Arc<dyn BirthdayRepository>,
        notification_repo: Arc<dyn NotificationChannelRepository>,
        default_days_before: Vec<u32>,
        encryption_key: String,
        signal_runtime: SignalRuntimeConfig,
    ) -> Self {
        Self {
            user_repo,
            birthday_repo,
            notification_repo,
            default_days_before,
            encryption_key,
            signal_runtime,
        }
    }

    /// Decrypt a channel record's config
    fn decrypt_record(
        &self,
        mut record: NotificationChannelRecord,
    ) -> Option<NotificationChannelRecord> {
        if let Some(encrypted) = record.config.get("_encrypted").and_then(|v| v.as_str()) {
            match crypto::decrypt(encrypted, &self.encryption_key) {
                Ok(json_str) => match serde_json::from_str(&json_str) {
                    Ok(config) => {
                        record.config = config;
                        Some(record)
                    }
                    Err(e) => {
                        error!("Failed to parse decrypted config: {}", e);
                        None
                    }
                },
                Err(e) => {
                    error!("Failed to decrypt channel config: {}", e);
                    None
                }
            }
        } else {
            // Not encrypted (legacy data), return as-is
            Some(record)
        }
    }

    /// Run reminder check for all users
    pub async fn run_for_all_users(&self) -> anyhow::Result<()> {
        let users = self
            .user_repo
            .find_all()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch users: {}", e))?;

        for user in &users {
            if let Err(e) = self.run_for_user(&user.id).await {
                error!("Reminder job failed for user {}: {}", user.username, e);
            }
        }

        // Clean up reminder log entries older than 400 days
        match self.birthday_repo.cleanup_old_reminders(400).await {
            Ok(count) if count > 0 => info!("Cleaned up {} old reminder log entries", count),
            Err(e) => warn!("Failed to clean up old reminder logs: {}", e),
            _ => {}
        }

        Ok(())
    }

    /// Run reminder check for a single user
    pub async fn run_for_user(&self, user_id: &UserId) -> anyhow::Result<()> {
        // Get user's custom reminder days or fall back to default
        let days_before = match self.user_repo.get_reminder_days(user_id).await? {
            Some(days) => days.into_iter().map(|d| d as u32).collect(),
            None => self.default_days_before.clone(),
        };
        let policy = ReminderPolicy::new(days_before);

        // Get all user's birthdays
        let birthdays = self
            .birthday_repo
            .find_all_for_user(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch birthdays: {}", e))?;

        if birthdays.is_empty() {
            return Ok(());
        }

        let today = Local::now().date_naive();
        let due = compute_due_reminders(&birthdays, &policy, today);

        if due.is_empty() {
            return Ok(());
        }

        // Get enabled notification channels and decrypt their configs
        let raw_channels = self
            .notification_repo
            .find_enabled_for_user(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch channels: {}", e))?;

        let channels: Vec<NotificationChannelRecord> = raw_channels
            .into_iter()
            .filter_map(|r| self.decrypt_record(r))
            .collect();

        if channels.is_empty() {
            info!("User has no enabled notification channels, skipping");
            return Ok(());
        }

        let year = today.year();
        let mut last_send_by_user_channel: HashMap<String, Instant> = HashMap::new();
        let whatsapp_min_interval = Duration::from_millis(500);

        for reminder in &due {
            for channel in &channels {
                // Check if already reminded
                let already = match self
                    .birthday_repo
                    .has_been_reminded(
                        &reminder.birthday.id,
                        &channel.channel_type,
                        reminder.days_before,
                        year,
                    )
                    .await
                {
                    Ok(val) => val,
                    Err(e) => {
                        error!(
                            "Failed to check reminder status for {} on {}: {}",
                            reminder.birthday.name, channel.channel_type, e
                        );
                        continue;
                    }
                };

                if already {
                    continue;
                }

                if channel.channel_type == "whatsapp" {
                    let limiter_key = format!("{}:{}", user_id.0, channel.channel_type);
                    if let Some(last_send) = last_send_by_user_channel.get(&limiter_key) {
                        let elapsed = last_send.elapsed();
                        if elapsed < whatsapp_min_interval {
                            sleep(whatsapp_min_interval - elapsed).await;
                        }
                    }
                    last_send_by_user_channel.insert(limiter_key, Instant::now());
                }

                // Build sender and send
                match dispatcher::build_sender(channel, &self.signal_runtime) {
                    Ok(sender) => match sender.send(reminder).await {
                        Ok(()) => {
                            info!(
                                "Sent {} reminder for {} ({} days before)",
                                channel.channel_type, reminder.birthday.name, reminder.days_before
                            );
                            // Log the reminder
                            if let Err(e) = self
                                .birthday_repo
                                .log_reminder(
                                    &reminder.birthday.id,
                                    &channel.channel_type,
                                    reminder.days_before,
                                    year,
                                )
                                .await
                            {
                                warn!("Failed to log reminder: {}", e);
                            }
                        }
                        Err(e) => {
                            error!(
                                "Failed to send {} for {}: {}",
                                channel.channel_type, reminder.birthday.name, e
                            );
                        }
                    },
                    Err(e) => {
                        error!("Failed to build {} sender: {}", channel.channel_type, e);
                    }
                }
            }
        }

        Ok(())
    }
}
