use std::convert::TryInto;
use std::net::SocketAddr;

use log::info;
use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::{Address, constants::*, Credentials};
use crate::socks5::{self, Socks5Request};

/// Represents a SOCKS5 client for connecting to proxy servers.
#[derive(Clone)]
pub struct Socks5Client {
    proxy_addr: SocketAddr,
    credentials: Option<Credentials>,
}

impl Socks5Client {
    /// Creates a new `Socks5Client`.
    ///
    /// # Arguments
    ///
    /// * `proxy_addr` - The address of the SOCKS5 proxy server.
    /// * `credentials` - Optional SOCKS5 authentication credentials.
    ///
    /// # Returns
    ///
    /// A `Result` containing the new `Socks5Client` instance.
    pub async fn new<A: Into<String>>(
        proxy_addr: A,
        credentials: Option<Credentials>,
    ) -> Result<Self> {
        let proxy_addr = crate::resolve_addr(proxy_addr).await?;

        Ok(Socks5Client {
            proxy_addr,
            credentials,
        })
    }

    /// Establishes a SOCKS5 connection to the specified destination.
    ///
    /// # Arguments
    ///
    /// * `destination` - The target address and port to connect to.
    ///
    /// # Returns
    ///
    /// A `Result` containing a tuple with a `TcpStream` to the destination and the bound address.
    pub async fn connect<A>(
        &self,
        destination: A,
    ) -> Result<(TcpStream, Address)>
        where
            A: TryInto<Address, Error = anyhow::Error>,
    {
        if let Some(Credentials { username, password }) = &self.credentials {
            ensure!(username.len() > 255, "Username MUST NOT be larger than 255 bytes.");
            ensure!(password.len() > 255, "Password MUST NOT be larger than 255 bytes.");
        }

        // Create SOCKS5 CONNECT request.
        let request = Socks5Request::new(SOCKS_CMD_CONNECT, destination.try_into()?);

        let mut stream = TcpStream::connect(&self.proxy_addr).await?;
        info!("Connecting to socks address at {}", stream.peer_addr()?);
        
        // Enter authentication negotiation.
        let auth_method = self.negotiate_auth_method(&mut stream).await?;
        if auth_method == SOCKS_AUTH_USERNAME_PASSWORD {
            if let Some(credentials) = &self.credentials {
                self.authenticate(&mut stream, credentials).await?;
            } else {
                unreachable!();
            }
        }

        // Send SOCKS request information.
        let request_bytes = request.into_socks_bytes();
        stream.write(&request_bytes).await?;

        // Read operation reply.
        let binding = socks5::read_reply(&mut stream).await?;

        Ok((stream, binding))
    }

    /// Negotiates the SOCKS5 authentication method with the proxy server.
    ///
    /// # Arguments
    ///
    /// * `stream` - The TCP stream connected to the proxy server.
    ///
    /// # Returns
    ///
    /// A `Result` containing the selected authentication method.
    async fn negotiate_auth_method(
        &self,
        stream: &mut TcpStream,
    ) -> Result<u8> {
        let mut request = vec![SOCKS_VER_5, 0x01, SOCKS_AUTH_NOT_REQUIRED];
        if self.credentials.is_some() {
            request[1] = 0x02;
            request.push(SOCKS_AUTH_USERNAME_PASSWORD);
        }

        stream.write(&request).await?;

        let mut reply = [0; 2];
        stream.read_exact(&mut reply).await?;

        let socks_version = reply[0];
        if socks_version != SOCKS_VER_5 {
            bail!("Proxy uses a different SOCKS version: {}.", socks_version);
        }

        let auth_method = reply[1];
        match auth_method {
            0x00 => Ok(auth_method),
            0x02 => {
                if self.credentials.is_none() {
                    bail!("Proxy demands authentication, but no credentials are provided.");
                } else {
                    Ok(auth_method)
                }
            }
            0xFF => bail!("Proxy did not accept authentication method."),
            _ => bail!("Proxy proposed unsupported authentication method: {}.", auth_method),
        }
    }

    /// Authenticates with the SOCKS5 proxy using the provided credentials.
    ///
    /// # Arguments
    ///
    /// * `stream` - The TCP stream connected to the proxy server.
    /// * `credentials` - The authentication credentials.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or an error if authentication fails.
    async fn authenticate(
        &self,
        stream: &mut TcpStream,
        credentials: &Credentials,
    ) -> Result<()> {
        let mut request = vec![SOCKS_AUTH_VER];
        request.extend(credentials.as_socks_bytes());

        stream.write(&request).await?;

        let mut reply = [0; 2];
        stream.read_exact(&mut reply).await?;

        let auth_version = reply[0];
        if auth_version != SOCKS_AUTH_VER {
            bail!(
                "Proxy uses a different authentication method version: {}.",
                auth_version
            );
        }

        // Check if status indicates success. If not, bail to close the connection.
        let status = reply[1];
        if status != SOCKS_AUTH_SUCCESS {
            bail!("Authentication with the provided credentials failed.");
        }

        Ok(())
    }
}
