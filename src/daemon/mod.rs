mod player;
pub mod state;

pub use player::PlayerManager;
pub use state::IPCCommand;

use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

pub struct Daemon {
    addr: SocketAddr,
    player: PlayerManager,
}

impl Daemon {
    pub async fn new(port: u16) -> Result<Self> {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let player = PlayerManager::new().await?;

        info!("Daemon initialized on {}", addr);
        Ok(Self { addr, player })
    }

    pub async fn run(&mut self) -> Result<()> {
        let listener = TcpListener::bind(self.addr).await?;
        info!("Daemon listening on {}", self.addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    info!("Connection from {}", peer);
                    let player = self.player.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_client(stream, player).await {
                            error!("Client error: {}", e);
                        }
                    });
                }
                Err(e) => error!("Accept error: {}", e),
            }
        }
    }

    async fn handle_client(mut stream: TcpStream, player: PlayerManager) -> Result<()> {
        loop {
            let mut len_bytes = [0u8; 4];
            match stream.read_exact(&mut len_bytes).await {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            let len = u32::from_be_bytes(len_bytes) as usize;
            if len > 10 * 1024 * 1024 {
                return Err(anyhow::anyhow!("Payload too large: {} bytes", len));
            }

            let mut buf = vec![0u8; len];
            stream.read_exact(&mut buf).await?;

            let command: IPCCommand = bincode::deserialize(&buf)?;
            let response = player.handle_command(command).await;
            let data = bincode::serialize(&response)?;

            let resp_len = (data.len() as u32).to_be_bytes();
            stream.write_all(&resp_len).await?;
            stream.write_all(&data).await?;
        }

        Ok(())
    }
}
