use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct LedgerSendParams {
    pub to: String,
    pub amount: u64,
    pub symbol: String,
}
