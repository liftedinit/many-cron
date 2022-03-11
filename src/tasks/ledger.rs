use serde::Deserialize;

#[derive(Deserialize)]
pub struct LedgerParams {
    pub to: String,
    pub amount: u64,
    pub symbol: String,
}


#[derive(Deserialize)]
pub struct LedgerSend {
        pub schedule: String,
        pub params: LedgerParams,
}