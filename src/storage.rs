//! The persistent storage where to log the task response

use many::message::ResponseMessage;
use many::ManyError;
use tokio::sync::Mutex;
use tracing::info;

use std::path::Path;

use crate::errors;

/// The persistent key-value store where to log the task responses
pub struct CronStorage {
    persistent_store: Mutex<fmerk::Merk>,
}

impl CronStorage {
    /// Create a new persistent key-value storage instance
    pub fn new<P: AsRef<Path>>(persistent_path: P) -> Result<Self, String> {
        let persistent_store = fmerk::Merk::open(persistent_path).map_err(|e| e.to_string())?;

        Ok(Self {
            persistent_store: Mutex::new(persistent_store),
        })
    }

    /// Generate the key where to store the response in the key-value store
    pub(super) fn get_key(&self, r: &ResponseMessage) -> Vec<u8> {
        format!("/cron/{:?}", r.to_bytes()).into_bytes()
    }

    /// Store a new entry in the database and display the entry after storage
    pub async fn push(&self, response: ResponseMessage) -> Result<(), ManyError> {
        let mut lock = self.persistent_store.lock().await;
        lock.apply(&[(
            self.get_key(&response),
            fmerk::Op::Put(minicbor::to_vec(&response).unwrap()), // Caution: Encoding the response will modify it's content!
        )])
        .map_err(|e| errors::storage_error(e.to_string()))?;

        let result = lock
            .get(&self.get_key(&response))
            .map_err(|e| errors::storage_mutex_error(e.to_string()))?;
        if result.is_some() {
            info!(
                "{:?}",
                ResponseMessage::from_bytes(&result.unwrap())
                    .map_err(errors::response_deserialization_error)?
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CronStorage, ManyError, ResponseMessage};
    use many::Identity;
    use tempdir::TempDir;

    impl CronStorage {
        /// Retrieve a key from the database.
        /// Only used for testing purpose
        async fn get(self, k: Vec<u8>) -> Result<ResponseMessage, String> {
            let lock = self.persistent_store.lock().await;
            let result = lock.get(&k).map_err(|e| e.to_string())?;
            ResponseMessage::from_bytes(&result.unwrap())
        }
    }

    /// Push a `ResponseMessage` to the database, retrieve it from the database and compare both
    #[tokio::test]
    async fn storage() -> Result<(), ManyError> {
        let dir = TempDir::new("MANY_").unwrap();
        let file_path = dir.path().join("cron.db");
        let storage = CronStorage::new(file_path).unwrap();

        let rm = ResponseMessage::default();
        let key = storage.get_key(&rm);
        storage.push(rm).await?;

        let result = storage.get(key).await.unwrap();
        assert!(result.version.is_none());
        assert_eq!(result.from, Identity::anonymous());
        assert!(result.to.is_none());
        assert!(result.data.unwrap().is_empty());
        assert!(result.timestamp.is_some());
        assert!(result.id.is_none());
        assert_eq!(result.attributes, Default::default());
        Ok(())
    }
}
