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

#[tokio::main]
pub async fn schedule_tasks(
    client: ManyClient,
    tasks: Tasks,
    storage: CronStorage,
) -> Result<(), ManyError> {
    let mut sched = JobScheduler::new();
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
