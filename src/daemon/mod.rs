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
        let mut buf = vec![0u8; 8192];

        loop {
            let n = stream.read(&mut buf).await?;
            if n == 0 {
                break;
            }

            let command: IPCCommand = bincode::deserialize(&buf[..n])?;
            let response = player.handle_command(command).await;
            let data = bincode::serialize(&response)?;

            stream.write_all(&data).await?;
        }

        Ok(())
    }
}
