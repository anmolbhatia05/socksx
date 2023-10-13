/// This is the main entry point for the SOCKSX proxy server.
/// It is responsible for parsing CLI arguments, setting up logging, and
/// spawning the main event loop.
/// The main event loop is responsible for accepting incoming connections and
/// spawning a new task for each connection.
/// Each task is responsible for handling the SOCKS handshake and proxying
/// data between the client and the destination server.
#[macro_use]
extern crate human_panic;

use std::{convert::TryInto, sync::Arc};

use anyhow::Result;
use clap::Parser;
use dotenv::dotenv;
use itertools::Itertools;
use log::{info, LevelFilter};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio::time::Instant;

use socksx::{self, Socks5Handler, Socks6Handler, SocksHandler};

// Alias for SOCKS handler with Arc and Sync/Send trait bounds
type Handler = Arc<dyn SocksHandler + Sync + Send>;

/// CLI arguments structure
#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"))]
struct Args {
    /// Entry in the proxy chain, the order is preserved
    #[clap(short, long, env = "CHAIN")]
    chain: Vec<String>,

    /// Prints debug information
    #[clap(short, long, env = "DEBUG")]
    debug: bool,

    /// Host (IP) for the SOCKS server
    #[clap(short='H', long, env = "HOST", default_value = "0.0.0.0")]
    host: String,

    /// Concurrent connections limit (0=unlimted)
    #[clap(short, long, env = "LIMIT", default_value = "256")]
    limit: usize,

    /// Port for the SOCKS server
    #[clap(short, long, env = "PORT", default_value = "1080")]
    port: u16,

    /// SOCKS version
    #[clap(short, long, env = "SOCKS", default_value = "6")]
    socks: u8,
}

/// Main asynchronous function
#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from `.env` file
    dotenv().ok();
    let args = Args::parse();

    // Setup logger
    let mut logger = env_logger::builder();
    logger.format_module_path(false);

    if args.debug {
        logger.filter_level(LevelFilter::Debug).init();
    } else {
        logger.filter_level(LevelFilter::Info).init();
    }

    // Setup human-friendly panic messages
    setup_panic!(Metadata {
        name: "SOCKSX".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        authors: env!("CARGO_PKG_AUTHORS").replace(":", ", ").into(),
        homepage: env!("CARGO_PKG_HOMEPAGE").into(),
    });

    // TODO: validate host

    // Convert and collect chain arguments
    let chain = args.chain.iter().cloned().map(|c| c.try_into()).try_collect()?;

    // Create a semaphore for connection limiting
    let semaphore = if args.limit > 0 {
        Some(Arc::new(Semaphore::new(args.limit)))
    } else {
        None
    };

    // Bind TCP listener to the specified host and port
    let listener = TcpListener::bind(format!("{}:{}", args.host, args.port)).await?;
    // Determine the appropriate SOCKS handler based on the specified version and restricting them to 5 and 6
    let handler: Handler = match args.socks {
        5 => Arc::new(Socks5Handler::new(chain)),
        6 => Arc::new(Socks6Handler::new(chain)),
        _ => unreachable!(),
    };

    // Main event loop for accepting incoming connections
    loop {
        let (incoming, _) = listener.accept().await?;

        let handler = Arc::clone(&handler);
        let semaphore = semaphore.clone();

        tokio::spawn(process(incoming, handler, semaphore));
    }
}

/// Asynchronously processes an incoming connection
///
/// # Parameters
///
/// - `incoming`: The incoming `TcpStream`.
/// - `handler`: The SOCKS handler.
/// - `semaphore`: An optional semaphore for limiting concurrent connections.
///
/// # Returns
///
/// Returns a `Result` indicating the success or failure of the operation.
async fn process(
    incoming: TcpStream,
    handler: Handler,
    semaphore: Option<Arc<Semaphore>>,
) -> Result<()> {
    let mut incoming = incoming;
    let start_time = Instant::now();

    // Handle the incoming connection based on the availability of permits
    if let Some(semaphore) = semaphore {
        let permit = semaphore.try_acquire();
        if permit.is_ok() {
            handler.accept_request(&mut incoming).await?;
        } else {
            handler.refuse_request(&mut incoming).await?;
        }
    } else {
        handler.accept_request(&mut incoming).await?;
    }

    // Log the time taken to process the request
    info!("Request fulfilled in {}ms", Instant::now().saturating_duration_since(start_time).as_millis());

    Ok(())
}
