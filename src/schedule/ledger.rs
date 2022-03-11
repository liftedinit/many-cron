use many::{Identity, ManyError};
use many::message::ResponseMessage;
use many::protocol::attributes::response::AsyncAttribute;
use many::server::module::ledger::{InfoReturns, SendArgs};
use many::types::ledger::TokenAmount;
use many_client::ManyClient;
use num_bigint::BigUint;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{debug, error, info};

use std::str::FromStr;

use std::sync::Arc;

use crate::lib::decode_identity;
use crate::tasks::ledger::LedgerParams;

pub fn schedule_ledger_send(
    client: Arc<ManyClient>,
    sched: &mut JobScheduler,
    schedule: String,
    params: LedgerParams,
) -> Result<(), JobSchedulerError>
{
    let params = Arc::new(params);
    sched.add(
        Job::new_async(&schedule, move |_uuid, _lock| {
            let params = params.clone();
            let client = client.clone();

            Box::pin(async move {
                info!(
                    "Transfering {}{} to {}",
                    params.amount, params.symbol, params.to
                );

                let id =
                    decode_identity(params.to.clone());

                if let Err(e) = id.clone() {
                    error!("{}", e.to_string());
                }
                // Execute the transaction in a thread allowed to block, since the HTTP transport is blocking
                // The maximum number of blocking thread that Tokio can spawn is 512 by default
                let result = tokio::task::spawn_blocking(move || {
                    let result = send(
                        &client,
                        id.unwrap(),
                        BigUint::from(params.amount),
                        params.symbol.clone(),
                    );

                    if let Err(e) = result {
                        error!("{}", e.to_string());
                    }
                })
                .await;

                if let Err(e) = result {
                    error!("{}", e.to_string());
                }
            })
        })
        .unwrap()
    )?;
    Ok(())
}

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
pub fn send(
    client: &ManyClient,
    to: Identity,
    amount: BigUint,
    symbol: String,
) -> Result<(), ManyError> {
    let symbol = resolve_symbol(client, symbol)?;

    if client.id.identity.is_anonymous() {
        Err(ManyError::invalid_identity())
    } else {
        let arguments = SendArgs {
            from: None,
            to,
            symbol,
            amount: TokenAmount::from(amount),
        };
        let ResponseMessage {
            data, attributes, ..
        } = client.call("ledger.send", arguments)?;

        let payload = data?;
        if payload.is_empty() {
            let attr = attributes.get::<AsyncAttribute>()?;
            debug!("Async token: {}", hex::encode(&attr.token));
            Ok(())
        } else {
            minicbor::display(&payload);
            Ok(())
        }
    }
}

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
fn resolve_symbol(client: &ManyClient, symbol: String) -> Result<Identity, ManyError> {
    if let Ok(symbol) = Identity::from_str(&symbol) {
        Ok(symbol)
    } else {
        // Get info.
        let info: InfoReturns = minicbor::decode(&client.call_("ledger.info", ())?).unwrap();
        info.local_names
            .into_iter()
            .find(|(_, y)| y == &symbol)
            .map(|(x, _)| x)
            .ok_or_else(|| ManyError::unknown(format!("Could not resolve symbol '{}'", &symbol)))
    }
}