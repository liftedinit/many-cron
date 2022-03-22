use clap::Parser;
use many::types::identity::cose::CoseKeyIdentity;
use many::Identity;
use many_client::ManyClient;
use tracing::level_filters::LevelFilter;
use tracing::{debug, error};

use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

mod errors;
mod schedule;
mod tasks;

use schedule::schedule_tasks;
use tasks::*;

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

    /// Path to a task list
    #[clap(long)]
    tasks: PathBuf,
}

fn main() {
    let Opts {
        server,
        server_id,
        pem,
        verbose,
        quiet,
        tasks,
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

    let pem = std::fs::read_to_string(&pem).expect("Could not read PEM file.");
    let key = CoseKeyIdentity::from_pem(&pem).expect("Could not create COSE identity from PEM");

    let server_id = server_id.unwrap_or_default();

    let json_file = File::open(tasks).expect("Unable to open tasks JSON file");
    let reader = BufReader::new(json_file);
    let tasks: Tasks = serde_json::from_reader(reader).expect("Unable to read the tasks JSON file");

    let client = ManyClient::new(&server, server_id, key).expect("Unable to create MANY client");

    let res = schedule_tasks(client, tasks);
    if let Err(e) = res {
        error!("Task scheduling error {e}");
        std::process::exit(1);
    }
}
