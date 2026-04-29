use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use crate::application::reminder_job::ReminderJobService;

pub async fn start_scheduler(
    schedule: &str,
    reminder_service: Arc<ReminderJobService>,
) -> anyhow::Result<JobScheduler> {
    let scheduler = JobScheduler::new().await?;

    let svc = reminder_service.clone();
    let job = Job::new_async(schedule, move |_uuid, _lock| {
        let svc = svc.clone();
        Box::pin(async move {
            info!("Running scheduled reminder check");
            if let Err(e) = svc.run_for_all_users().await {
                error!("Reminder job failed: {}", e);
            }
        })
    })?;

    scheduler.add(job).await?;
    scheduler.start().await?;

    info!("Scheduler started with schedule: {}", schedule);
    Ok(scheduler)
}
