use crate::app::data_handler;
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::sync::Arc;
use tokio::sync::mpsc::{Sender, channel};

pub struct Repository {
    client: Arc<PgPool>,
    tx: Sender<HMessage>,
}

impl Repository {
    pub async fn new(url: &str) -> Self {
        let client = PgPoolOptions::default()
            .max_connections(5)
            .connect(url)
            .await
            .unwrap();
        let client = Arc::new(client);
        let (tx, rx) = channel(64);

        tokio::spawn(data_handler(client.clone(), rx));

        Self { client, tx }
    }

    pub async fn new_group(self, group: String) {
        for i in group.split(",") {
            if let Err(er) = self.tx.send(HMessage::Group(i.to_string())).await {
                tracing::error!("{er}");
            }
        }
    }
}

impl std::ops::Deref for Repository {
    type Target = PgPool;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

pub enum HMessage {
    Group(String),
}
