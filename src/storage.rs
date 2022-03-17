//! The persistent storage where to log the task response

use many::message::ResponseMessage;
use many::{Identity, ManyError};
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

    /// Generate the key where to store the error in the key-value store
    pub(super) fn get_error_key(&self, from: Option<Identity>) -> Vec<u8> {
        match from {
            Some(id) => format!("/cron_err/{id}").into_bytes(),
            None => b"/cron_err/".to_vec(),
        }
    }

    /// Generate the key where to store the response in the key-value store
    pub(super) fn get_response_key(&self, r: &ResponseMessage) -> Vec<u8> {
        format!("/cron/{:?}/{:?}", r.from, r.to).into_bytes()
    }

    /// Store a new error in the database and display the entry after storage
    pub async fn push_error(
        &self,
        error: ManyError,
        from: Option<Identity>,
    ) -> Result<(), ManyError> {
        let mut lock = self.persistent_store.lock().await;
        let key = self.get_error_key(from);
        lock.apply(&[(
            key.clone(),
            fmerk::Op::Put(minicbor::to_vec(&error).unwrap()),
        )])
        .map_err(|e| errors::storage_error(e.to_string()))?;

        let result = lock
            .get(&key)
            .map_err(|e| errors::storage_mutex_error(e.to_string()))?;
        if result.is_some() {
            info!(
                "{:?}",
                ManyError::from_bytes(&result.unwrap())
                    .map_err(errors::response_deserialization_error)?
            );
        }
        Ok(())
    }

    /// Store a new response message in the database and display the entry after storage
    pub async fn push_response(&self, response: ResponseMessage) -> Result<(), ManyError> {
        let mut lock = self.persistent_store.lock().await;
        lock.apply(&[(
            self.get_response_key(&response),
            fmerk::Op::Put(minicbor::to_vec(&response).unwrap()), // Caution: Encoding the response will modify it's content!
        )])
        .map_err(|e| errors::storage_error(e.to_string()))?;

        let result = lock
            .get(&self.get_response_key(&response))
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
        /// Retrieve a response message from the database.
        /// Only used for testing purpose
        /// TODO: Make a single generic method?
        async fn get_rm(self, k: Vec<u8>) -> Result<ResponseMessage, String> {
            let lock = self.persistent_store.lock().await;
            let result = lock.get(&k).map_err(|e| e.to_string())?;
            ResponseMessage::from_bytes(&result.unwrap())
        }

        /// Retrieve a response message from the database.
        /// Only used for testing purpose
        /// TODO: Make a single generic method?
        async fn get_err(self, k: Vec<u8>) -> Result<ManyError, String> {
            let lock = self.persistent_store.lock().await;
            let result = lock.get(&k).map_err(|e| e.to_string())?;
            ManyError::from_bytes(&result.unwrap())
        }
    }

    fn setup_tmp_storage() -> CronStorage {
        let dir = TempDir::new("MANY_").unwrap();
        let file_path = dir.path().join("cron.db");
        CronStorage::new(file_path).unwrap()
    }

    /// Push a `ResponseMessage` to the database, retrieve it from the database and compare both
    #[tokio::test]
    async fn store_response_message() -> Result<(), ManyError> {
        let storage = setup_tmp_storage();

        let rm = ResponseMessage::default();
        let key = storage.get_response_key(&rm);
        storage.push_response(rm).await?;

        let result = storage.get_rm(key).await.unwrap();
        assert!(result.version.is_none());
        assert_eq!(result.from, Identity::anonymous());
        assert!(result.to.is_none());
        assert!(result.data.unwrap().is_empty());
        assert!(result.timestamp.is_some());
        assert!(result.id.is_none());
        assert_eq!(result.attributes, Default::default());
        Ok(())
    }

    #[tokio::test]
    async fn store_error() -> Result<(), ManyError> {
        let storage = setup_tmp_storage();

        let err = ManyError::default();
        let id = Identity::anonymous();
        let key = storage.get_error_key(Some(id));

        storage.push_error(err.clone(), Some(id)).await?;

        let result = storage.get_err(key).await.unwrap();

        assert_eq!(result, err);

        Ok(())
    }
}
