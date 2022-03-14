use serde::Deserialize;

use self::ledger::LedgerSendParams;

pub mod ledger;

#[derive(Deserialize)]
#[serde(tag = "endpoint", content = "params")]
pub enum Params {
    #[serde(alias = "ledger.send")]
    LedgerSend(LedgerSendParams),
}
#[derive(Deserialize)]
pub struct Task {
    pub schedule: String,
    #[serde(flatten)]
    pub params: Params,
}
#[derive(Deserialize)]
pub struct Tasks {
    tasks: Vec<Task>,
}

// and we'll implement IntoIterator
impl IntoIterator for Tasks {
    type Item = Task;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tasks.into_iter()
    }
}
