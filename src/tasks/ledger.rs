//! Task parameters for the various `Ledger` enpoints

use cbor_diag;
use many::{types::ledger::TokenAmount, Identity};
use minicbor::{decode, Decode};
use serde::{Deserialize, Deserializer};
/// Define the task parameter for the `ledger.send` endpoint
#[derive(Clone, Decode, Debug)]
#[cbor(map)]
pub struct LedgerSendParams {
    #[n(1)]
    pub to: Identity,

    #[n(2)]
    pub amount: TokenAmount,

    #[n(3)]
    pub symbol: String,
}

/// Deserialize CBOR string from JSON
/// TODO: Make this generic?
pub(crate) fn from_cbor<'de, D>(deserializer: D) -> Result<LedgerSendParams, D::Error>
where
    D: Deserializer<'de>,
{
    let p = cbor_diag::parse_diag(&String::deserialize(deserializer)?).unwrap();
    decode(&p.to_bytes()).map_err(serde::de::Error::custom)
}
