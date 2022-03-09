use clap::Parser;
use many::message::ResponseMessage;
use many::protocol::attributes::response::AsyncAttribute;
use many::server::module::ledger::{InfoReturns, SendArgs};
use many::types::identity::cose::CoseKeyIdentity;
use many::types::ledger::TokenAmount;
use many::{Identity, ManyError};
use many_client::ManyClient;
use minicbor::data::Tag;
use minicbor::encode::{Error, Write};
use minicbor::{Decoder, Encoder};
use num_bigint::BigUint;
use serde::Deserialize;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::level_filters::LevelFilter;
use tracing::{debug, error, info};

use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::BufReader;
use std::iter::IntoIterator;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
#[derive(Clone, Debug, Deserialize)]
#[repr(transparent)]
struct Amount(pub BigUint);

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
impl Display for Amount {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
impl minicbor::Encode for Amount {
    fn encode<W: Write>(&self, e: &mut Encoder<W>) -> Result<(), Error<W::Error>> {
        e.tag(Tag::PosBignum)?.bytes(&self.0.to_bytes_be())?;
        Ok(())
    }
}

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
impl<'b> minicbor::Decode<'b> for Amount {
    fn decode(d: &mut Decoder<'b>) -> Result<Self, minicbor::decode::Error> {
        let t = d.tag()?;
        if t != Tag::PosBignum {
            Err(minicbor::decode::Error::Message("Invalid tag."))
        } else {
            Ok(Amount(BigUint::from_bytes_be(d.bytes()?)))
        }
    }
}

#[derive(Parser, Debug)]
struct Opts {
    /// Many server URL to connect to.
    #[clap(default_value = "http://localhost:8000")]
    server: String,

    /// The identity of the server (an identity string), or anonymous if you don't know it.
    server_id: Option<Identity>,

    /// A PEM file for the identity. If not specified, anonymous will be used.
    #[clap(long)]
    pem: PathBuf,

    /// Increase output logging verbosity to DEBUG level.
    #[clap(short, long, parse(from_occurrences))]
    verbose: i8,

    /// Suppress all output logging. Can be used multiple times to suppress more.
    #[clap(short, long, parse(from_occurrences))]
    quiet: i8,

    /// Path to a persistent store database (rocksdb).
    #[clap(long)]
    persistent: Option<PathBuf>,

    /// Path to a task list
    #[clap(long)]
    tasks: PathBuf,

    /// Delete the persistent storage to start from a clean state.
    /// If this is not specified the initial state will not be used.
    #[clap(long, short)]
    clean: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct Task {
    to: String,
    amount: u64,
    symbol: String,
    schedule: String,
}

#[derive(Deserialize)]
struct Tasks(Vec<Task>);

// and we'll implement IntoIterator
impl IntoIterator for Tasks {
    type Item = Task;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

fn main() {
    let Opts {
        server,
        server_id,
        pem,
        verbose,
        quiet,
        persistent,
        tasks,
        clean,
    } = Opts::parse();

    let verbose_level = 2 + verbose - quiet;
    let log_level = match verbose_level {
        x if x > 3 => LevelFilter::TRACE,
        3 => LevelFilter::DEBUG,
        2 => LevelFilter::INFO,
        1 => LevelFilter::WARN,
        0 => LevelFilter::ERROR,
        x if x < 0 => LevelFilter::OFF,
        _ => unreachable!(),
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    debug!("{:?}", Opts::parse());

    if clean {
        // Delete the persistent storage
        if let Some(persistent_path) = persistent {
            match std::fs::remove_dir_all(persistent_path) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    panic!("Error: {}", e)
                }
            }
        } else {
            panic!("Unable to execute --clean without --persistent")
        }
    }

    // Load the PEM key file
    let pem = std::fs::read_to_string(&pem).expect("Could not read PEM file.");

    // Create the COSE identity from the PEM key
    let key = CoseKeyIdentity::from_pem(&pem).expect("Could not create COSE identity from PEM");

    // Server ID is either something of None
    let server_id = server_id.unwrap_or_default();

    // TODO: Load tasks from JSON here
    let json_file = File::open(tasks).expect("Unable to open tasks JSON file");
    let reader = BufReader::new(json_file);
    let tasks: Tasks = serde_json::from_reader(reader).expect("Unable to read the tasks JSON file");

    // Connect to the MANY server
    let client = Arc::new(ManyClient::new(&server, server_id, key).expect("Unable to create MANY client"));

    // We should use an external thread because a Tokio runtime already exists
    // many-rs : http.rs
    std::thread::spawn(|| {
        schedule(client, tasks);
    })
    .join()
    .expect("Scheduling thread error");
}

// We can't use #[tokio::main(worker_thread=...)] because we need to lookup the number of cpus at runtime
fn schedule(client: Arc<ManyClient>, tasks: Tasks) {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(num_cpus::get() - 1) // There's already a Tokio runtime started by many-rs. We don't want to overschedule
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut sched = JobScheduler::new();

            for task in tasks.into_iter() {
                let schedule = &task.schedule.clone();
                let task = Arc::new(task);
                let client = client.clone();
                let result = sched.add(
                    Job::new_async(schedule, move |_uuid, _l| {
                        let task = task.clone();
                        let client = client.clone();
                        Box::pin(async move {
                            info!("Transfering {}{} to {}", task.amount, task.symbol, task.to);


                            let identity =
                            if let Ok(data) = hex::decode(&task.to) {
                                match Identity::try_from(data.as_slice()) {
                                    Ok(mut i) => {
                                        // if let Some(subid) = task.to.subid {
                                        //     i = i.with_subresource_id(subid);
                                        // }
                                        println!("First block: {}", i);
                                        i
                                    }
                                    Err(e) => {
                                        eprintln!("Identity did not parse: {:?}", e.to_string());
                                        std::process::exit(1);
                                    }
                                }
                            } else if let Ok(mut i) = Identity::try_from(task.to.clone()) {
                                // if let Some(subid) = o.subid {
                                //     i = i.with_subresource_id(subid);
                                // }
                                println!("Second block: {}", hex::encode(&i.to_vec()));
                                i
                            } else {
                                error!("Invalid identity {}", task.to);
                                panic!("Invalid identity {}", task.to)
                            };         

                            // Execute the transaction
                            let result =
                                send(client, identity, BigUint::from(task.amount), task.symbol.clone());

                            // Exit if the server returned an error
                            if let Err(e) = result {
                                error!("Error returned by server {}", e);
                                std::process::exit(1);
                            }
                        })
                    })
                    .unwrap(),
                );

                if let Err(e) = result {
                    error!("Job scheduler error {:?}", e);
                    std::process::exit(1);
                }
            }

            info!("Starting cron scheduler");
            // 500ms tick
            let results = sched.start().await;
            if let Err(e) = results {
                error!("Async join error {}", e);
                std::process::exit(1);
            }
        });
}

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
fn send(
    client: Arc<ManyClient>,
    to: Identity,
    amount: BigUint,
    symbol: String,
) -> Result<(), ManyError> {
    let symbol = resolve_symbol(&client, symbol)?;

    if client.id.identity.is_anonymous() {
        Err(ManyError::invalid_identity())
    } else {
        let arguments = SendArgs {
            from: None,
            to,
            symbol,
            amount: TokenAmount::from(amount),
        };
        let ResponseMessage {
            data, attributes, ..
        } = client.call("ledger.send", arguments)?;

        let payload = data?;
        if payload.is_empty() {
            let attr = attributes.get::<AsyncAttribute>()?;
            debug!("Async token: {}", hex::encode(&attr.token));
            Ok(())
        } else {
            minicbor::display(&payload);
            Ok(())
        }
    }
}

// Taken from omni-ledger/src/ledger/main.rs
// TODO: DRY
fn resolve_symbol(client: &ManyClient, symbol: String) -> Result<Identity, ManyError> {
    if let Ok(symbol) = Identity::from_str(&symbol) {
        Ok(symbol)
    } else {
        // Get info.
        let info: InfoReturns = minicbor::decode(&client.call_("ledger.info", ())?).unwrap();
        info.local_names
            .into_iter()
            .find(|(_, y)| y == &symbol)
            .map(|(x, _)| x)
            .ok_or_else(|| ManyError::unknown(format!("Could not resolve symbol '{}'", &symbol)))
    }
}
