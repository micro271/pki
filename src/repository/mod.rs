use serde::Deserialize;
use sqlx::{
    FromRow, Row,
    postgres::{PgPool, PgPoolOptions},
};
use tokio::sync::mpsc::Sender;

use crate::models::Event;

#[derive(Debug, Clone)]
pub struct Repository {
    client: PgPool,
    tx: Sender<HMessage>,
}

impl Repository {
    pub async fn new(url: &str, tx: Sender<HMessage>) -> Self {
        let client = PgPoolOptions::default()
            .max_connections(15)
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

    pub async fn get_unresolved_events(&self) -> Option<Vec<Event>> {
        sqlx::query("SELECT eventid FROM events WHERE end_time IS NULL")
            .fetch_all(&self.client)
            .await
            .ok()
            .map(|x| {
                x.into_iter()
                    .filter_map(|x| Event::from_row(&x).ok())
                    .collect()
            })
    }

    pub async fn get_groupid_by_name(&self, name: &str) -> Option<i64> {
        sqlx::query("SELECT groupid FROM zbx_groups WHERE group_name = $1")
            .bind(name)
            .fetch_one(&self.client)
            .await
            .ok()
            .map(|x| x.get("groupid"))
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
