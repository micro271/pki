use crate::{app::data_handler, repository::Repository};
use tokio::{
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
    todo!()
}
