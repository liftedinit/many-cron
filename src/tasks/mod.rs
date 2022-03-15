//! Define new task types and how to deserialize them

use serde::Deserialize;

use self::ledger::LedgerSendParams;

pub mod ledger;

/// Task parameters for different task types
#[derive(Deserialize)]
#[serde(tag = "endpoint", content = "params")]
pub enum Params {
    #[serde(alias = "ledger.send")]
    LedgerSend(LedgerSendParams),
}

/// The base Task type
#[derive(Deserialize)]
pub struct Task {
    pub schedule: String,
    #[serde(flatten)]
    pub params: Params,
}

/// A simple task collection
#[derive(Deserialize)]
pub struct Tasks {
    tasks: Vec<Task>,
}

/// Allow iterating on our task collection
impl IntoIterator for Tasks {
    type Item = Task;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tasks.into_iter()
    }
}
