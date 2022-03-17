//! Task handlers and utilities for the `Ledger` MANY component

use many::message::ResponseMessage;
use many::server::module::ledger::{InfoReturns, SendArgs};
use many::types::ledger::TokenAmount;
use many::{Identity, ManyError};
use many_client::ManyClient;
use num_bigint::BigUint;
use tracing::info;

use std::str::FromStr;
use std::sync::Arc;

use crate::errors;
use crate::storage::CronStorage;
use crate::tasks::ledger::LedgerSendParams;

/// Handles the `ledger.send` task to transfer some amount from one account to another
pub async fn ledger_send(
    client: Arc<ManyClient>,
    storage: Arc<CronStorage>,
    params: LedgerSendParams,
) -> Result<(), ManyError> {
    // Execute the transaction in a thread allowed to block, since the HTTP transport is blocking
    // The maximum number of blocking thread that Tokio can spawn is 512 by default
    let response = tokio::task::spawn_blocking(move || {
        info!(
            "Transfering {}{} to {}",
            params.amount, params.symbol, params.to
        );
        send(
            &client,
            params.to,
            BigUint::from(params.amount),
            params.symbol.clone(),
        )
    })
    .await
    .map_err(|e| errors::job_error(format!("{:?}", e)))?
    .map_err(|e| errors::ledger_send_error(e.to_string()))?;

    storage.push_response(response).await?;

    Ok(())
}

/// Performs the actual `ledger.send` transaction
///
/// Inspired from many-framework/src/ledger/main.rs
/// TODO: DRY
pub fn send(
    client: &ManyClient,
    to: Identity,
    amount: BigUint,
    symbol: String,
) -> Result<ResponseMessage, ManyError> {
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
        let response = client.call("ledger.send", arguments)?;
        Ok(response)
    }
}

/// Request the list of supported symbols from the server and make sure the symbol we're using is valid
///
/// Taken from many-framework/src/ledger/main.rs
/// TODO: DRY
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
