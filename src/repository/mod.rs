use influxdb3_client::{Client, ClientConfig, Point, Precision};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub struct Repository {
    client: Arc<Client>,
    tx: Sender<HMessage>,
}

impl Repository {
    pub async fn new(host: &str, token: &str, database: &str) -> Self {
        let conf = ClientConfig::builder()
            .host(host)
            .token(token)
            .database(database)
            .build()
            .unwrap();
        let client = Arc::new(Client::new(conf).await.unwrap());
        let (tx, rx) = channel(64);
        let cl = client.clone();
        tokio::spawn(_data_handler(cl, rx));

        Self { client, tx }
    }
}

async fn _data_handler(client: Arc<Client>, mut rx: Receiver<HMessage>) {
    let mut groups: HashMap<String, Vec<i32>> = HashMap::new();
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(180)) => {
                _data(&client, &mut groups).await
            }
            msg = rx.recv() => {
                if let Some(HMessage::Group(g)) = msg {
                    groups.insert(g, Vec::default());
                }
            }
        }
    }
}

async fn _data(db: &Arc<Client>, groups: &mut HashMap<String, Vec<i32>>) {
    for hids in groups.values() {}
}

pub enum HMessage {
    Group(String),
}
