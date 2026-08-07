use serde::Deserialize;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct Repository {
    client: PgPool,
    tx: Sender<HMessage>,
}

impl Repository {
    pub async fn new(url: &str, tx: Sender<HMessage>) -> Self {
        let client = PgPoolOptions::default()
            .max_connections(5)
            .connect(url)
            .await
            .unwrap();
        let client = client;

        Self { client, tx }
    }

    pub async fn new_group(&self, group: String) {
        for i in group.split(",") {
            if let Err(er) = self.tx.send(HMessage::Group(i.to_string())).await {
                tracing::error!("{er}");
            }
        }
    }

    pub async fn get_db(&self) -> PgPool {
        self.client.clone()
    }
}

impl std::ops::Deref for Repository {
    type Target = PgPool;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[derive(Debug, Deserialize)]
pub enum HMessage {
    Group(String),
}
