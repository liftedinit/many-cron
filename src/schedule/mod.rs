//! Task scheduling and handlers

use many::ManyError;
use many_client::ManyClient;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info};

use std::sync::Arc;

use crate::errors;
use crate::tasks::Tasks;

/// Wait this amount of second before exiting the application when a shutdown signal is received
const SHUTDOWN_DELAY: u64 = 5;

/// Schedule tasks in the Tokio cron scheduler
///
/// You need to add new task handlers here
#[tokio::main]
pub async fn schedule_tasks(client: ManyClient, tasks: Tasks) -> Result<(), ManyError> {
    let mut sched = JobScheduler::new();

    // Shutdown gracefully. Wait some amount of time for the task to finish
    sched.shutdown_on_ctrl_c();
    sched
        .set_shutdown_handler(Box::new(|| {
            Box::pin(async move {
                info!("Shutting down... Waiting {SHUTDOWN_DELAY} secs for tasks to finish");
                tokio::time::sleep(tokio::time::Duration::from_secs(SHUTDOWN_DELAY)).await;
                info!("Goodbye!");
                // TODO: Use channels/broadcast here?
                // See https://github.com/mvniekerk/tokio-cron-scheduler/issues/14
                std::process::exit(0); // This is required for the application to stop...
            })
        }))
        .map_err(|e| errors::scheduler_error(format!("{:?}", e)))?;

    let client = Arc::new(client);

    info!("Adding tasks to scheduler");
    for task in tasks.into_iter() {
        let client = client.clone();
        let job = Job::new_async(&task.schedule.clone(), move |_uuid, _lock| {
            let client = client.clone();
            let task = task.clone();
            Box::pin(async move {
                // TODO: Can I propage the error using '?'?
                let result = task.execute(client).await;
                if let Err(e) = result {
                    error!("Task execution error {e}");
                } else {
                    info!("{:?}", result.unwrap());
                }
            })
        })
        .map_err(|e| errors::job_error(e.to_string()))?;

        sched
            .add(job)
            .map_err(|e| errors::job_scheduling_error(format!("{:?}", e)))?;
    }

    info!("Starting cron scheduler");
    // 500ms tick
    sched
        .start()
        .await
        .map_err(|e| errors::scheduler_error(format!("{:?}", e)))?;
    Ok(())
}
