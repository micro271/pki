use core::panic;
use serde_json::{Value, from_value, json};
use sqlx::{
    Row,
    postgres::{PgPool, PgPoolOptions},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::models::{EventId, Severity, Status, Timestamp};

const URL: &str = "http://172.30.0.153/api_jsonrpc.php";
const LIMIT: usize = 2000;

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

        tokio::spawn(_data_handler(client.clone(), rx));

        Self { client, tx }
    }
}

async fn _data_handler(client: Arc<PgPool>, mut rx: Receiver<HMessage>) {
    let mut groups: HashMap<String, (Timestamp, EventId, Vec<i32>)> = HashMap::new();
    loop {
        tokio::select! {
            () = tokio::time::sleep(Duration::from_secs(180)) => {
                task(client.clone(), &mut groups).await
            }
            msg = rx.recv() => {
                if let Some(HMessage::Group(g)) = msg {
                    groups.insert(g, (Timestamp::new((time::OffsetDateTime::now_utc() - std::time::Duration::from_hours(720)).unix_timestamp()), EventId::new(0), Vec::default()));
                }
            }
        }
    }
}

async fn task(db: Arc<PgPool>, groups: &mut HashMap<String, (Timestamp, EventId, Vec<i32>)>) {
    let token = std::env::var("TOKEN").unwrap();
    for (from, eventid, hids) in groups.values_mut() {
        let mut length = 2000;
        let mut resolved = HashMap::new();
        let mut problems = VecDeque::new();
        let to = time::OffsetDateTime::now_utc().unix_timestamp();
        while LIMIT == length {
            let d = json!({
                "jsonrpc":"2.0",
                "method":"event.get",
                "params":{
                        "output":"extend",
                        "source":0,
                        "object":0,
                        "hostids": hids,
                        "time_from": from,
                        "time_till": to,
                        "selectHosts": ["hostid","host"],
                        "selectRelatedObject": ["triggerid","description","priority"],
                        "sortfield": ["clock", "eventid"],
                        "sortorder":"ASC",
                        "eventid_from": eventid,
                        "limit": LIMIT
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

            let status_code = resp.status();
            if status_code == 200 {
                let mut events: Value = resp.json().await.unwrap();
                let events = match events.as_object_mut().unwrap().remove("result").unwrap() {
                    Value::Array(val) => val,
                    _ => panic!(""),
                };

                length = events.len();

                for ev in events {
                    if ev["value"].as_i64().unwrap() == 1 {
                        problems.push_back(ev);
                    } else {
                        resolved.insert(
                            ev["eventid"].as_i64().unwrap(),
                            ev["clock"].as_i64().unwrap(),
                        );
                    }
                }
            } else {
                tracing::error!("error: {}", resp.json::<String>().await.unwrap_or_default());
                length = 0;
            }

            if problems.is_empty() {
                continue;
            }

            let len = problems.len();
            let mut count = 0;
            let mut eventids = Vec::with_capacity(len);
            let mut nodos = Vec::with_capacity(len);
            let mut severities = Vec::with_capacity(len);
            let mut triggers = Vec::with_capacity(len);
            let mut start_times = Vec::with_capacity(len);
            let mut opdatas = Vec::with_capacity(len);
            let mut end_times = Vec::with_capacity(len);
            let mut statuses = Vec::with_capacity(len);

            while let Some(mut ev) = problems.pop_front()
                && count < len
            {
                count += 1;
                let r_res = ev["r_eventid"].as_i64().unwrap();
                let end_time = resolved.remove(&r_res);

                let eid = ev["eventid"].as_i64().unwrap();
                let clock = ev["clock"].as_i64().unwrap();

                if eid > eventid.get() {
                    eventid.set(eid);
                }

                from.set(end_time.unwrap_or(clock));

                if end_time.is_none() && status_code == 200 && length == LIMIT {
                    problems.push_back(ev);
                    continue;
                }

                eventids.push(eid);
                nodos.push(ev["host"].to_string());
                severities.push(from_value::<Severity>(ev["severity"].take()).unwrap());
                triggers.push(ev["trigger"].to_string());
                start_times.push(clock);
                opdatas.push(ev["opdata"].to_string());
                end_times.push(end_time);
                statuses.push(if end_time.is_some() {
                    Status::Resolved
                } else {
                    Status::Ongoing
                });
            }

            sqlx::query(
                r#"
                    INSERT INTO eventos
                        (eventid, nodo, severity, trigger, start_time, opdata, end_time, status)
                    SELECT * FROM UNNEST(
                        $1::bigint[],
                        $2::text[],
                        $3::smallint[],
                        $4::text[],
                        $5::bigint[],
                        $6::text[],
                        $7::bigint[],
                        $8::text[]
                    )
                    "#,
            )
            .bind(&eventids)
            .bind(&nodos)
            .bind(&severities)
            .bind(&triggers)
            .bind(&start_times)
            .bind(&opdatas)
            .bind(&end_times)
            .bind(&statuses)
            .execute(&*db)
            .await
            .unwrap();
        }

        if !resolved.is_empty() {
            data_update(db.clone(), resolved).await;
        }
    }
}

async fn data_update(db: Arc<PgPool>, mut resolved: HashMap<i64, i64>) {
    let events_id = sqlx::query("SELECT eventid FROM events WHERE end_time IS NULL")
        .fetch_all(&*db)
        .await
        .unwrap()
        .into_iter()
        .map(|x| x.get("eventid"))
        .collect::<Vec<i64>>();

    let d = json!({
        "jsonrpc":"2.0",
        "method":"event.get",
            "params":{
            "output":"extend",
            "source":0,
            "object":0,
            "eventids": events_id,
            "selectHosts": ["hostid","host"],
            "selectRelatedObject": ["triggerid","description","priority"],
            "sortfield": ["clock", "eventid"],
            "sortorder":"ASC"
        },
        "id":1
    });
    let token = std::env::var("TOKEN").unwrap();
    let req = reqwest::Client::new();
    let mut res = req
        .post(URL)
        .json(&d)
        .header("Authentication", format!("Bearer {token}"))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();
    let res = res.as_object_mut().unwrap();
    match res.remove("result").unwrap() {
        Value::Array(vec) => {
            let mut vec_eventid = Vec::with_capacity(vec.len());
            let mut vec_end_time = Vec::with_capacity(vec.len());

            for i in vec {
                let r_eventid = i["r_eventid"].as_i64().unwrap();
                vec_eventid.push(r_eventid);
                vec_end_time.push(resolved.remove(&r_eventid).unwrap());
            }
            sqlx::query(
                r"
                UPDATE user u
                SET 
                    status = 'resolved'::event_statis,
                    end_time = tmp.end_time,
                FROM (
                    SELECT * 
                    FROM UNNEST (
                        $1::bigint[],
                        $2::bigint[],
                        $3::text[]
                    ) as tmp(eventid, end_time, status)
                )
                WHERE u.eventid = tmp.eventid
            ",
            )
            .bind(&vec_eventid)
            .bind(&vec_end_time)
            .execute(&*db)
            .await
            .unwrap();
        }
        _ => panic!(),
    }
}

pub enum HMessage {
    Group(String),
}
