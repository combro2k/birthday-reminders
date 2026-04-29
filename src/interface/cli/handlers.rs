use std::sync::Arc;

use crate::application::birthday_commands::BirthdayCommandService;
use crate::application::birthday_queries::BirthdayQueryService;
use crate::application::reminder_job::ReminderJobService;
use crate::application::user_commands::UserCommandService;
use crate::domain::repository::BirthdayRepository;
use crate::domain::user::Role;
use crate::domain::user_repository::UserRepository;
use crate::infrastructure::database::DatabasePool;

use super::commands::Commands;

pub async fn handle_command(
    cmd: Commands,
    db: &DatabasePool,
    _user_repo: &Arc<dyn UserRepository>,
    _birthday_repo: &Arc<dyn BirthdayRepository>,
    user_cmd_svc: &UserCommandService,
    birthday_cmd_svc: &BirthdayCommandService,
    birthday_query_svc: &BirthdayQueryService,
    reminder_svc: &Arc<ReminderJobService>,
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
            notes,
            token,
        } => {
            let user_id = user_cmd_svc.resolve_api_token(&token, db).await?;
            let birth_date = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")?;
            let birthday = birthday_cmd_svc
                .add(&user_id, &name, birth_date, notes)
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
                "{:<36} {:<20} {:<12} {:<5} {:<10}",
                "ID", "Name", "Birth Date", "Age", "Days Until"
            );
            println!("{}", "-".repeat(83));

            let today = chrono::Local::now().date_naive();
            for b in &birthdays {
                println!(
                    "{:<36} {:<20} {:<12} {:<5} {:<10}",
                    b.id.0,
                    b.name,
                    b.birth_date.format("%Y-%m-%d"),
                    b.age_on(today),
                    b.days_until_next_from(today),
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
        Commands::CheckReminders => {
            println!("Running reminder check for all users...");
            reminder_svc.run_for_all_users().await?;
            println!("Done.");
            Ok(())
        }
    }
}
