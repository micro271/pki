use crate::{
    app::data_handler,
    repository::{HMessage, Repository},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::mpsc::channel,
};

pub mod app;
pub mod models;
pub mod repository;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    let db = std::env::var("DATABASE").unwrap();
    let userdb = std::env::var("USER_DB").unwrap();
    let passwddb = std::env::var("PASS_DB").unwrap();
    let host = std::env::var("HOST_DB").unwrap();
    let port = std::env::var("PORT_DB").unwrap();
    let socket_path = std::env::var("FILE_SOCKET").unwrap_or("/tmp/rust-app.socket".to_string());

    if let Err(er) = std::fs::remove_file(&socket_path) {
        tracing::error!("Delete file ({socket_path}) failed: {er}");
    }

    let (stm, socket) = UnixListener::bind(socket_path)?.accept().await?;

    tracing::info!("Listening {socket:?}");

    let url = format!("postgres://{userdb}:{passwddb}@{host}:{port}/{db}");

    let (tx, rx) = channel(64);

    let repo = Repository::new(&url, tx).await;

    tokio::spawn(socket_handler(stm, repo.clone()));
    tokio::spawn(data_handler(repo, rx)).await?;

    /* postgres:///mydb?host=/var/run/postgresql */

    Ok(())
}

async fn socket_handler(stream: UnixStream, repo: Repository) {
    let (mut rx, _tx) = stream.into_split();

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
        let data = match serde_json::from_slice::<HMessage>(&data_buf).unwrap() {
            HMessage::Group(e) => e,
        };

        repo.new_group(data).await;
    }
}
