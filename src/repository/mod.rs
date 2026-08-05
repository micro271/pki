use influxdb3_client::{Client, ClientConfig, Point, Precision};
use serde::{Deserialize, Serialize, de::IntoDeserializer};
use serde_json::{Value, from_value, json};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use tracing::event;

use crate::models::{Event, Severity, Status};

const URL: &str = "http://172.30.0.153/api_jsonrpc.php";

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
    let mut groups: HashMap<String, (i64, i64, Vec<i32>)> = HashMap::new();
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(180)) => {
                _data(&client, &mut groups).await
            }
            msg = rx.recv() => {
                if let Some(HMessage::Group(g)) = msg {
                    groups.insert(g, ((time::OffsetDateTime::now_utc() - std::time::Duration::from_secs(30*24*3600)).unix_timestamp(), 0, Vec::default()));
                }
            }
        }
    }
}

async fn _data(db: &Arc<Client>, groups: &mut HashMap<String, (i64, i64, Vec<i32>)>) {
    let token = std::env::var("TOKEN").unwrap();
    for (_, (from, eventid, hids)) in groups {
        let limit = 2000;
        let mut length = 2000;

        while limit == length {
            let d = json!({
                "jsonrpc":"2.0",
                "method":"event.get",
                "params":{
                        "output":"extend",
                        "source":0,
                        "object":0,
                        "hostids": hids,
                        "time_from": from,
                        "time_till": time::OffsetDateTime::now_utc().unix_timestamp(),
                        "selectHosts": ["hostid","host"],
                        "selectRelatedObject": ["triggerid","description","priority"],
                        "sortfield": ["clock", "eventid"],
                        "sortorder":"ASC",
                        "eventid_from": eventid,
                        "limit": 2000
                },
                "id":1
            });
            let req = reqwest::Client::default();
            let resp = req
                .post(URL)
                .header("Authentication", format!("Bearer {token}"))
                .json(&d)
                .send()
                .await
                .unwrap();

            if resp.status() == 200 {
                let mut events: Value = resp.json().await.unwrap();
                let events = match events.as_object_mut().unwrap().remove("result").unwrap() {
                    Value::Array(val) => val,
                    _ => panic!(""),
                };

                length = events.len();
                let mut resolved = HashMap::new();
                let mut problems = Vec::new();
                for ev in events {
                    if ev["value"].as_i64().unwrap() == 1 {
                        problems.push(ev);
                    } else {
                        resolved.insert(ev["eventid"].as_i64().unwrap(), ev);
                    }
                }

                let events = problems
                    .into_iter()
                    .map(|mut ev| {
                        let r_res = ev["r_eventid"].as_i64().unwrap();
                        let end_time = resolved
                            .remove(&r_res)
                            .map(|x| x["clock"].as_i64().unwrap());

                        let resp = Event {
                            eventid: ev["eventid"].as_i64().unwrap(),
                            nodo: ev["host"].to_string(),
                            severity: from_value(ev["severity"].take()).unwrap(),
                            trigger: ev["trigger"].to_string(),
                            start_time: ev["clock"].as_i64().unwrap(),
                            opdata: ev["opdata"].to_string(),
                            end_time: end_time,
                            status: end_time
                                .is_some()
                                .then_some(Status::Resolved)
                                .unwrap_or(Status::Ongoing),
                        };
                        if resp.eventid > *eventid {
                            *eventid = resp.eventid;
                        }

                        *from = end_time.unwrap_or(resp.start_time);

                        resp.into()
                    })
                    .collect::<Vec<Point>>();

                db.write(events).await.unwrap();
            } else {
                tracing::error!("{}", resp.status());
                break;
            }
        }
    }
}

pub enum HMessage {
    Group(String),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EventResponse {
    result: Vec<Event>,
}

impl From<Event> for Point {
    fn from(value: Event) -> Self {
        Point::new("events")
            .tag("nodo", value.nodo)
            .tag("severity", value.severity)
            .field("status", value.status)
            .field("trigger", value.trigger)
            .field("opdata", value.opdata)
            .field("end_time", value.end_time.unwrap())
            .timestamp_nanos(
                std::time::Duration::from_secs(value.start_time as u64).as_nanos() as i64,
            )
    }
}
