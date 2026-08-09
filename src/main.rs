use crate::{
    app::data_handler,
    repository::{HMessage, Repository},
};
use tokio::{io::AsyncReadExt, net::UnixListener, sync::mpsc::channel};
use tracing::Instrument;
use tracing_subscriber::EnvFilter;

pub mod app;
pub mod models;
pub mod repository;
pub mod zabbix_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_level(true)
        .init();
    dotenv::dotenv().ok();
    let socket_path = std::env::var("FILE_SOCKET").unwrap_or("/tmp/rust-app.socket".to_string());

    if let Err(er) = std::fs::remove_file(&socket_path) {
        tracing::error!("Delete file ({socket_path}) failed: {er}");
    }

    let lst = UnixListener::bind(socket_path)?;
    let database = std::env::var("DATABASE")?;
    let (tx, rx) = channel(64);

    let repo = Repository::new(&database, tx).await;
    let span = tracing::info_span!("{SOCKET}");
    tokio::spawn(socket_handler(lst, repo.clone()).instrument(span));

    let span = tracing::info_span!("{MAIN_TASK}");
    tokio::spawn(data_handler(repo, rx).instrument(span)).await?;

    Ok(())
}

async fn socket_handler(lst: UnixListener, repo: Repository) {
    tracing::info!("Listen: {:?}", lst.local_addr());
    loop {
        match lst.accept().await {
            Ok((stream, socket)) => {
                tracing::info!("New connection, socket: {socket:?}");
                let (mut rx, _) = stream.into_split();
                let repo = repo.clone();
                tokio::spawn(async move {
                    loop {
                        let mut len_buf = [0u8; 4];

                        if let Err(er) = rx.read_exact(&mut len_buf).await {
                            tracing::error!("{er}");
                        }

                        let len = u32::from_be_bytes(len_buf) as usize;
                        let mut data_buf = vec![0u8; len];
                        if let Err(er) = rx.read_exact(&mut data_buf).await {
                            tracing::error!("{er}");
                            continue;
                        }
                        let data = serde_json::from_slice::<HMessage>(&data_buf);
                        tracing::info!("New message from {socket:?}: {data:?}");
                        let data = match data.unwrap() {
                            HMessage::Group(e) => e,
                        };

                        repo.new_group(data).await;
                    }
                });
            }
            Err(er) => tracing::error!("Connection error: {er}"),
        }
    }
}
