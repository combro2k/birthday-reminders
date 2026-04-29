use std::sync::Arc;

use chrono::{Datelike, Local};
use tracing::{error, info, warn};

use crate::domain::reminder::ReminderPolicy;
use crate::domain::repository::{BirthdayRepository, NotificationChannelRepository};
use crate::domain::services::compute_due_reminders;
use crate::domain::user::UserId;
use crate::domain::user_repository::UserRepository;
use crate::infrastructure::notifications::dispatcher;

pub struct ReminderJobService {
    user_repo: Arc<dyn UserRepository>,
    birthday_repo: Arc<dyn BirthdayRepository>,
    notification_repo: Arc<dyn NotificationChannelRepository>,
    default_days_before: Vec<u32>,
}

impl ReminderJobService {
    pub fn new(
        user_repo: Arc<dyn UserRepository>,
        birthday_repo: Arc<dyn BirthdayRepository>,
        notification_repo: Arc<dyn NotificationChannelRepository>,
        default_days_before: Vec<u32>,
    ) -> Self {
        Self {
            user_repo,
            birthday_repo,
            notification_repo,
            default_days_before,
        }
    }

    /// Run reminder check for all users
    pub async fn run_for_all_users(&self) -> anyhow::Result<()> {
        let users = self.user_repo.find_all().await.map_err(|e| {
            anyhow::anyhow!("Failed to fetch users: {}", e)
        })?;

        for user in &users {
            if let Err(e) = self.run_for_user(&user.id).await {
                error!("Reminder job failed for user {}: {}", user.username, e);
            }
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
        let birthdays = self.birthday_repo.find_all_for_user(user_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to fetch birthdays: {}", e)
        })?;

        if birthdays.is_empty() {
            return Ok(());
        }

        let today = Local::now().date_naive();
        let due = compute_due_reminders(&birthdays, &policy, today);

        if due.is_empty() {
            return Ok(());
        }

        // Get enabled notification channels
        let channels = self.notification_repo.find_enabled_for_user(user_id).await.map_err(|e| {
            anyhow::anyhow!("Failed to fetch channels: {}", e)
        })?;

        if channels.is_empty() {
            info!("User has no enabled notification channels, skipping");
            return Ok(());
        }

        let year = today.year();

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

                // Build sender and send
                match dispatcher::build_sender(channel) {
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
                        error!(
                            "Failed to build {} sender: {}",
                            channel.channel_type, e
                        );
                    }
                }
            }
        }

        Ok(())
    }
}
