use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use log::info;

use crate::{Socks6Client, SocksHandler};
use crate::addresses::ProxyAddress;
use crate::socks6::{self, Socks6Reply};

/// Implements a SOCKS6 handler.
#[derive(Clone)]
pub struct Socks6Handler {
    static_links: Vec<ProxyAddress>,
}

impl Default for Socks6Handler {
    /// Default constructor for `Socks6Handler`.
    fn default() -> Self {
        Self::new(vec![])
    }
}

impl Socks6Handler {
    /// Constructs a new `Socks6Handler`.
    ///
    /// # Parameters
    /// - `static_links`: A list of static proxy addresses.
    ///
    /// # Returns
    /// A new `Socks6Handler`.
    pub fn new(static_links: Vec<ProxyAddress>) -> Self {
        Socks6Handler { static_links }
    }
}

#[async_trait]
impl SocksHandler for Socks6Handler {
    /// Accepts a request from the source and sets up a tunnel to the destination.
    ///
    /// # Parameters
    /// - `source`: A mutable reference to the source TCP stream.
    ///
    /// # Returns
    /// An `Ok(())` if the tunnel is successfully set up, otherwise an error.
    async fn accept_request(
        &self,
        source: &mut TcpStream,
    ) -> Result<()> {
        let mut destination = self.setup(source).await?;

        // Start bidirectional copy, after this the connection closes.
        tokio::io::copy_bidirectional(source, &mut destination).await?;

        Ok(())
    }

    /// Refuses a request from the source.
    ///
    /// # Parameters
    /// - `source`: A mutable reference to the source TCP stream.
    ///
    /// # Returns
    /// An `Ok(())` if the source is successfully notified of the refusal, otherwise an error.
    async fn refuse_request(
        &self,
        source: &mut TcpStream,
    ) -> Result<()> {
        // Notify source that the connection is refused.
        socks6::write_reply(source, Socks6Reply::ConnectionRefused).await?;

        Ok(())
    }

    /// Sets up the connection to the destination.
    ///
    /// # Parameters
    /// - `source`: A mutable reference to the source TCP stream.
    ///
    /// # Returns
    /// A `Result` containing the destination `TcpStream` if successful, otherwise an error.
    async fn setup(
        &self,
        source: &mut TcpStream,
    ) -> Result<TcpStream> {
        // Receive SOCKS request, and allow unauthenticated access.
        let request = socks6::read_request(source).await?;
        socks6::write_no_authentication(source).await?;

        let destination = request.destination.to_string();
        info!("Connecting to destination - {}", destination);
        let chain = request.chain(&self.static_links)?;

        let mut destination = if let Some(mut chain) = chain {
            if let Some(next) = chain.next_link() {
                let next = next.clone();

                let proxy_addr = format!("{}:{}", next.host, next.port);
                let client = Socks6Client::new(proxy_addr, next.credentials).await?;

                let (outgoing, _) = client.connect(destination, None, Some(chain.as_options())).await?;
                outgoing
            } else {
                TcpStream::connect(destination).await?
            }
        } else {
            TcpStream::connect(destination).await?
        };

        // Send initial data
        if request.initial_data_length > 0 {
            let mut initial_data = vec![0; request.initial_data_length as usize];
            source.read_exact(&mut initial_data).await?;
            destination.write(&initial_data).await?;
        }

        // Notify source that the connection has been set up.
        socks6::write_reply(source, Socks6Reply::Success).await?;
        source.flush().await?;

        Ok(destination)
    }
}
