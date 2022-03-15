//! Task parameters for the various `Ledger` enpoints

use serde::Deserialize;

/// Define the task parameter for the `ledger.send` endpoint
#[derive(Clone, Deserialize, Debug)]
pub struct LedgerSendParams {
    pub to: String,
    pub amount: u64,
    pub symbol: String,
}
