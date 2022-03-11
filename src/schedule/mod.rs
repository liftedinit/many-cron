use many_client::ManyClient;
use tokio_cron_scheduler::JobScheduler;
use tracing::{error, info};

use std::sync::Arc;

use crate::storage::CronStorage;
use crate::tasks::*;

mod ledger;
use ledger::schedule_ledger_send;

#[tokio::main]
pub async fn schedule_tasks(client: ManyClient, tasks: Tasks, storage: CronStorage) {
    let mut sched = JobScheduler::new();
    let client = Arc::new(client);
    let storage = Arc::new(storage);

    for task in tasks.into_iter() {
        match task {
            Task::LedgerSend(ls) => {
                let result = schedule_ledger_send(
                    client.clone(),
                    &mut sched,
                    storage.clone(),
                    ls.schedule,
                    ls.params,
                );
                if let Err(e) = result {
                    error!("Scheduling error {:?}", e);
                }
            }
        }
    }

    info!("Starting cron scheduler");
    // 500ms tick
    let results = sched.start().await;
    if let Err(e) = results {
        error!("Async join error {}", e);
        std::process::exit(1);
    }
}
