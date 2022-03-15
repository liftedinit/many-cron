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
    fn get_key(&self, r: &ResponseMessage) -> Vec<u8> {
        format!("/cron/{:?}", r.to_bytes()).into_bytes()
    }

    /// Store a new entry in the database and display the entry after storage
    pub async fn push(&self, response: ResponseMessage) -> Result<(), ManyError> {
        let mut lock = self.persistent_store.lock().await;
        lock.apply(&[(
            self.get_key(&response),
            fmerk::Op::Put(minicbor::to_vec(&response).unwrap()),
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
