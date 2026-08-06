use crate::repository::Repository;
use tokio::net::{UnixListener, UnixStream};

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

    tokio::spawn(socket_handler(stm));

    let url = format!("postgres://{userdb}:{passwddb}@{host}:{port}/{db}");

    /* postgres:///mydb?host=/var/run/postgresql */

    Ok(())
}

async fn socket_handler(stm: UnixStream) {
    todo!()
}
