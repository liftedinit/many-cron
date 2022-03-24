//! Task parameters for the various `Ledger` enpoints

use cbor_diag;
use many::{
    message::ResponseMessage,
    server::module::ledger::{InfoReturns, SendArgs},
    types::ledger::Symbol,
    ManyError,
};
use many_client::ManyClient;
use minicbor::decode;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer};
use tracing::info;

use std::{collections::BTreeMap, sync::Arc};

use crate::errors;

/// Mapping from Symbol to local name
type SymbolMap = BTreeMap<Symbol, String>;

/// Local symbol names. This will get initialized only once
static SYMBOLS: OnceCell<SymbolMap> = OnceCell::new();

/// Deserialize CBOR string from JSON
/// TODO: Make this generic?
pub(crate) fn from_cbor<'de, D>(deserializer: D) -> Result<SendArgs, D::Error>
where
    D: Deserializer<'de>,
{
    let p = cbor_diag::parse_diag(&String::deserialize(deserializer)?).unwrap();
    decode(&p.to_bytes()).map_err(serde::de::Error::custom)
}

/// Handles the `ledger.send` task to transfer some amount from one account to another
pub(crate) async fn send(
    client: Arc<ManyClient>,
    SendArgs {
        from,
        to,
        amount,
        symbol,
    }: SendArgs,
) -> Result<ResponseMessage, ManyError> {
    let response = tokio::task::spawn_blocking(move || {
        let local_symbol = SYMBOLS
            .get_or_init(|| local_symbol_name(&client).unwrap())
            .get(&symbol)
            .ok_or_else(|| ManyError::unknown(format!("Could not resolve symbol {}", symbol)))?;
        info!("Transfering {} {} to {}", amount, local_symbol, to);
        client.call(
            "ledger.send",
            SendArgs {
                from,
                to,
                amount,
                symbol,
            },
        )
    })
    .await
    .map_err(|e| errors::job_error(format!("{:?}", e)))?;

    response
}

/// Fetch the local symbol names from the ledger
fn local_symbol_name(client: &ManyClient) -> Result<SymbolMap, ManyError> {
    Ok(
        minicbor::decode::<InfoReturns>(&client.call_("ledger.info", ())?)
            .map_err(|e| ManyError::deserialization_error(e.to_string()))?
            .local_names,
    )
}
