use serde::Deserialize;

use self::ledger::LedgerSend;

pub mod ledger;

#[derive(Deserialize)]
#[serde(tag = "endpoint")]
pub enum Task {
    #[serde(alias = "ledger.send")]
    LedgerSend(LedgerSend),
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
