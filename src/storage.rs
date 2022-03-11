use many::message::ResponseMessage;
use tokio::sync::Mutex;
use tracing::{error, info};

use std::path::Path;

pub struct CronStorage {
    persistent_store: Mutex<fmerk::Merk>,
}

impl CronStorage {
    pub fn new<P: AsRef<Path>>(persistent_path: P) -> Result<Self, String> {
        let persistent_store = fmerk::Merk::open(persistent_path).map_err(|e| e.to_string())?;

        Ok(Self {
            persistent_store: Mutex::new(persistent_store),
        })
    }

    fn get_key(&self, r: &ResponseMessage) -> Vec<u8> {
        format!("/cron/{:?}", r.to_bytes()).into_bytes()
    }

    pub async fn push(&self, response: ResponseMessage) {
        let mut lock = self.persistent_store.lock().await;
        lock.apply(&[(
            self.get_key(&response),
            fmerk::Op::Put(minicbor::to_vec(&response).unwrap()),
        )])
        .unwrap();

        let result = lock.get(&self.get_key(&response));
        if let Err(e) = result {
            error!("Unable to get key from database {}", e);
        } else {
            let o = result.unwrap();
            if o.is_some() {
                // TODO: Handle error
                info!("{:?}", ResponseMessage::from_bytes(&o.unwrap()).unwrap());
            }
        }
    }
}
