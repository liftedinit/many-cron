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

#[cfg(test)]
mod tests {
    use super::{Params, Tasks};

    #[test]
    fn deserialize_ledger_send() {
        let json = r#"
        {
            "tasks": [
                {
                    "schedule": "1/5 * * * * *",
                    "endpoint": "ledger.send",
                    "params": {
                        "to": "oaa",
                        "amount": 10,
                        "symbol": "FBT"
                    }
                }
            ]
        }
        "#;

        let tasks: Tasks = serde_json::from_str(json).unwrap();
        let task = tasks.into_iter().next().unwrap();

        assert_eq!(task.schedule, "1/5 * * * * *");

        match task.params {
            Params::LedgerSend(p) => {
                assert_eq!(p.to, "oaa".to_string());
                assert_eq!(p.amount, 10);
                assert_eq!(p.symbol, "FBT".to_string());
            }
        }
    }
}
