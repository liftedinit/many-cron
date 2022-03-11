use clap::Parser;
use many::types::identity::cose::CoseKeyIdentity;
use many::Identity;
use many_client::ManyClient;
use tracing::debug;
use tracing::level_filters::LevelFilter;

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

mod lib;
mod schedule;
mod storage;
mod tasks;

use schedule::schedule_tasks;
use tasks::*;

use crate::storage::CronStorage;

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
    persistent: PathBuf,

    /// Path to a task list
    #[clap(long)]
    tasks: PathBuf,

    /// Delete the persistent storage to start from a clean state.
    /// If this is not specified the initial state will not be used.
    #[clap(long, short)]
    clean: bool,
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
        match std::fs::remove_dir_all(persistent.as_path()) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                panic!("Error: {}", e)
            }
        }
    }

    // Load the PEM key file
    let pem = std::fs::read_to_string(&pem).expect("Could not read PEM file.");

    // Create the COSE identity from the PEM key
    let key = CoseKeyIdentity::from_pem(&pem).expect("Could not create COSE identity from PEM");

    // Server ID is either something of None
    let server_id = server_id.unwrap_or_default();

    let json_file = File::open(tasks).expect("Unable to open tasks JSON file");
    let reader = BufReader::new(json_file);
    let tasks: Tasks = serde_json::from_reader(reader).expect("Unable to read the tasks JSON file");

    // Connect to the MANY server
    let client = ManyClient::new(&server, server_id, key).expect("Unable to create MANY client");

    let storage = CronStorage::new(persistent).expect("Unable to create permanent storage");

    schedule_tasks(client, tasks, storage);
}
