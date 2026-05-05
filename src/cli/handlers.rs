use std::sync::Arc;

use crate::birthdays::application::commands::BirthdayCommandService;
use crate::birthdays::application::export_service::ExportService;
use crate::birthdays::application::queries::BirthdayQueryService;
use crate::birthdays::domain::repository::BirthdayRepository;
use crate::infrastructure::database::DatabasePool;
use crate::reminders::application::reminder_job::ReminderJobService;
use crate::users::application::commands::UserCommandService;
use crate::users::domain::repository::UserRepository;
use crate::users::domain::user::Role;

use super::commands::Commands;

pub async fn handle_command(
    cmd: Commands,
    db: &DatabasePool,
    _user_repo: &Arc<dyn UserRepository>,
    birthday_repo: &Arc<dyn BirthdayRepository>,
    user_cmd_svc: &UserCommandService,
    birthday_cmd_svc: &BirthdayCommandService,
    birthday_query_svc: &BirthdayQueryService,
    reminder_svc: &Arc<ReminderJobService>,
    user_repo: &Arc<dyn UserRepository>,
) -> anyhow::Result<()> {
    match cmd {
        Commands::Serve { .. } => {
            // Handled in main.rs
            unreachable!()
        }
        Commands::CreateUser {
            username,
            email,
            password,
            admin,
        } => {
            let role = if admin { Role::Admin } else { Role::User };
            let user = user_cmd_svc
                .create_user(&username, &email, &password, role)
                .await?;
            println!("Created user: {} ({})", user.username, user.role.as_str());
            Ok(())
        }
        Commands::Add {
            name,
            date,
            email,
            phone_number,
            address,
            postal_code,
            city,
            country,
            notes,
            token,
        } => {
            let user_id = user_cmd_svc.resolve_api_token(&token, db).await?;
            let birth_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
            let birthday = birthday_cmd_svc
                .add(
                    &user_id,
                    &name,
                    birth_date,
                    email,
                    phone_number,
                    address,
                    postal_code,
                    city,
                    country,
                    notes,
                )
                .await?;
            println!(
                "Added birthday: {} ({})",
                birthday.name, birthday.birth_date
            );
            Ok(())
        }
        Commands::List { token } => {
            let user_id = user_cmd_svc.resolve_api_token(&token, db).await?;
            let birthdays = birthday_query_svc.list_all(&user_id).await?;

            if birthdays.is_empty() {
                println!("No birthdays found.");
                return Ok(());
            }

            println!(
                "{:<36} {:<20} {:<12} {:<5} {:<10} {:<15}",
                "ID", "Name", "Birth Date", "Age", "Days Until", "Phone"
            );
            println!("{}", "-".repeat(99));

            let today = chrono::Local::now().date_naive();
            for b in &birthdays {
                println!(
                    "{:<36} {:<20} {:<12} {:<5} {:<10} {:<15}",
                    b.id.0,
                    b.name,
                    b.birth_date.format("%Y-%m-%d"),
                    b.age_on(today),
                    b.days_until_next_from(today),
                    b.phone_number.as_deref().unwrap_or("-"),
                );
            }
            Ok(())
        }
        Commands::Upcoming { days, token } => {
            let user_id = user_cmd_svc.resolve_api_token(&token, db).await?;
            let birthdays = birthday_query_svc.get_upcoming(&user_id, days).await?;

            if birthdays.is_empty() {
                println!("No upcoming birthdays in the next {} days.", days);
                return Ok(());
            }

            println!(
                "{:<20} {:<12} {:<12} {:<10}",
                "Name", "Birth Date", "Turning", "Days Until"
            );
            println!("{}", "-".repeat(54));

            let today = chrono::Local::now().date_naive();
            for b in &birthdays {
                println!(
                    "{:<20} {:<12} {:<12} {:<10}",
                    b.name,
                    b.birth_date.format("%Y-%m-%d"),
                    b.turning_age_on(today),
                    b.days_until_next_from(today),
                );
            }
            Ok(())
        }
        Commands::Remove { id, token } => {
            let user_id = user_cmd_svc.resolve_api_token(&token, db).await?;
            let uuid = uuid::Uuid::parse_str(&id)?;
            birthday_cmd_svc.delete(uuid, &user_id).await?;
            println!("Deleted birthday {}", id);
            Ok(())
        }
        Commands::Export {
            admin,
            token,
            r#type,
            output,
        } => {
            handle_export_command(
                admin,
                token,
                r#type,
                output,
                db,
                user_cmd_svc,
                user_repo,
                birthday_repo,
            )
            .await
        }
        Commands::CheckReminders => {
            println!("Running reminder check for all users...");
            reminder_svc.run_for_all_users().await?;
            println!("Done.");
            Ok(())
        }
    }
}

async fn handle_export_command(
    admin: bool,
    token: Option<String>,
    export_type: Option<String>,
    output: Option<String>,
    db: &DatabasePool,
    user_cmd_svc: &UserCommandService,
    user_repo: &Arc<dyn UserRepository>,
    birthday_repo: &Arc<dyn BirthdayRepository>,
) -> anyhow::Result<()> {
    // Validate auth flags
    if admin && token.is_some() {
        anyhow::bail!("Cannot use --admin and --token together");
    }

    let export_svc = ExportService::new(birthday_repo.clone(), user_repo.clone());

    let user_id = if admin {
        None
    } else {
        let token_value =
            token.ok_or_else(|| anyhow::anyhow!("Either --admin or --token must be provided"))?;
        Some(user_cmd_svc.resolve_api_token(&token_value, db).await?)
    };

    let export_type_str = export_type.as_deref().unwrap_or("all");

    match export_type_str {
        "birthdays" => {
            let csv = if let Some(ref uid) = user_id {
                export_svc.export_birthdays_for_user(uid).await?
            } else {
                export_svc.export_all_birthdays().await?
            };

            if let Some(ref output_path) = output {
                export_svc.write_csv_file(&csv, output_path)?;
            } else {
                print!("{}", csv);
            }
            Ok(())
        }
        "users" => {
            if user_id.is_some() {
                anyhow::bail!("Only admins can export users");
            }
            let csv = export_svc.export_all_users().await?;
            if let Some(ref output_path) = output {
                export_svc.write_csv_file(&csv, output_path)?;
            } else {
                print!("{}", csv);
            }
            Ok(())
        }
        "api_tokens" => {
            let uid = user_id
                .ok_or_else(|| anyhow::anyhow!("API tokens export requires authentication"))?;
            let csv = export_svc.export_api_tokens(&uid, db).await?;
            if let Some(ref output_path) = output {
                export_svc.write_csv_file(&csv, output_path)?;
            } else {
                print!("{}", csv);
            }
            Ok(())
        }
        "notifications" => {
            let uid = user_id
                .ok_or_else(|| anyhow::anyhow!("Notifications export requires authentication"))?;
            let csv = export_svc.export_notifications(&uid, db).await?;
            if let Some(ref output_path) = output {
                export_svc.write_csv_file(&csv, output_path)?;
            } else {
                print!("{}", csv);
            }
            Ok(())
        }
        "reminders" => {
            let uid = user_id
                .ok_or_else(|| anyhow::anyhow!("Reminders export requires authentication"))?;
            let csv = export_svc.export_reminder_settings(&uid, db).await?;
            if let Some(ref output_path) = output {
                export_svc.write_csv_file(&csv, output_path)?;
            } else {
                print!("{}", csv);
            }
            Ok(())
        }
        "all" => {
            let mut exports = Vec::new();

            // Birthdays
            if let Some(ref uid) = user_id {
                let csv = export_svc.export_birthdays_for_user(uid).await?;
                exports.push(("birthdays.csv", csv));
            } else {
                let csv = export_svc.export_all_birthdays().await?;
                exports.push(("birthdays.csv", csv));
            }

            // API tokens
            if let Some(ref uid) = user_id {
                if let Ok(csv) = export_svc.export_api_tokens(uid, db).await {
                    exports.push(("api_tokens.csv", csv));
                }
            }

            // Notifications
            if let Some(ref uid) = user_id {
                if let Ok(csv) = export_svc.export_notifications(uid, db).await {
                    exports.push(("notifications.csv", csv));
                }
            }

            // Reminder settings
            if let Some(ref uid) = user_id {
                if let Ok(csv) = export_svc.export_reminder_settings(uid, db).await {
                    exports.push(("reminders.csv", csv));
                }
            }

            // Users (admin only)
            if admin {
                if let Ok(csv) = export_svc.export_all_users().await {
                    exports.push(("users.csv", csv));
                }
            }

            if let Some(ref output_dir) = output {
                export_svc.write_csv_files(exports, output_dir)?;
            } else {
                for (filename, content) in exports {
                    println!("=== {} ===", filename);
                    println!("{}", content);
                    println!();
                }
            }
            Ok(())
        }
        other => {
            anyhow::bail!(
                "Unknown export type: {}. Use: birthdays, users, api_tokens, notifications, reminders, or all",
                other
            )
        }
    }
}
