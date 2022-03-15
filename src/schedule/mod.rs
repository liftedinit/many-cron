//! Task scheduling and handlers

use many::ManyError;
use many_client::ManyClient;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use std::sync::Arc;

use crate::storage::CronStorage;
use crate::tasks::ledger::LedgerSendParams;
use crate::tasks::*;

use crate::errors;

mod ledger;
use ledger::ledger_send;

/// Wait this amount of second before exiting the application when a shutdown signal is received
const SHUTDOWN_DELAY: u64 = 5;

/// Schedule tasks in the Tokio cron scheduler
///
/// You need to add new task handlers here
#[tokio::main]
pub async fn schedule_tasks(
    client: ManyClient,
    tasks: Tasks,
    storage: CronStorage,
) -> Result<(), ManyError> {
    let mut sched = JobScheduler::new();

    // Shutdown gracefully. Wait some amount of time for the task to finish
    // We want to make sure we register the response in the persistent storage
    sched.shutdown_on_ctrl_c();
    sched
        .set_shutdown_handler(Box::new(|| {
            Box::pin(async move {
                info!("Shutting down... Waiting {SHUTDOWN_DELAY} secs for tasks to finish");
                tokio::time::sleep(tokio::time::Duration::from_secs(SHUTDOWN_DELAY)).await;
                info!("Goodbye!");
                std::process::exit(0);
            })
        }))
        .map_err(|e| errors::scheduler_error(format!("{:?}", e)))?;

    let client = Arc::new(client);
    let storage = Arc::new(storage);

    info!("Adding tasks to scheduler");
    for task in tasks.into_iter() {
        let client = client.clone();
        let storage = storage.clone();
        let params = Arc::new(task.params);
        let job = Job::new_async(&task.schedule, move |_uuid, _lock| {
            let client = client.clone();
            let storage = storage.clone();
            let params = params.clone();
            Box::pin(async move {
                let result = match params.as_ref() {
                    Params::LedgerSend(params) => {
                        ledger_send(client, storage, LedgerSendParams::clone(params)).await
                        //TODO: Can we propagate this error using '?'?
                    }
                };

                if let Err(e) = result {
                    panic!("Unable to schedule job {e}")
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
