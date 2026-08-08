pub mod task;
use serde_json::{Value, from_value, json};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};
use tokio::sync::{RwLock, mpsc::Receiver};
use tracing::Instrument;

use crate::{
    app::task::{data_update, load_groups, new_group},
    models::{EventId, Severity, Status, Timestamp},
    repository::{HMessage, Repository},
};

pub const URL: &str = "http://172.30.0.153/api_jsonrpc.php";
pub const LIMIT: usize = 2000;

pub type GroupType = Arc<RwLock<HashMap<String, (Arc<Timestamp>, Arc<EventId>, Vec<i64>)>>>;

pub async fn data_handler(client: Repository, mut rx: Receiver<HMessage>) {
    let mut groups = Arc::new(RwLock::new(load_groups(client.clone()).await));
    let mut interval = tokio::time::interval(Duration::from_secs(600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                task(client.clone(), &mut groups).await
            }
            msg = rx.recv() => {
                if let Some(HMessage::Group(g)) = msg {
                    tracing::debug!("New group: {g}");
                    if !groups.read().await.contains_key(&g) {
                        tokio::spawn(new_group(g, groups.clone()));
                    }
                }
            }
        }
    }
}

pub async fn task(db: Repository, groups: &mut GroupType) {
    let token = std::env::var("TOKEN").unwrap();
    let groups = {
        let tmp = groups.read().await;
        tmp.values()
            .map(|(t, e, hids)| (t.clone(), e.clone(), hids.clone()))
            .collect::<Vec<_>>()
        /* que hacer si obtengo nuevos hostids desde la funcion fetch_hostids */
    };

    for (from, eventid, hids) in groups {
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
                    if as_i64(&ev["value"]).unwrap() == 1 {
                        problems.push_back(ev);
                    } else {
                        resolved.insert(
                            as_i64(&ev["eventid"]).unwrap(),
                            as_i64(&ev["clock"]).unwrap(),
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
                let r_res = as_i64(&ev["r_eventid"]).unwrap();
                let end_time = resolved.remove(&r_res);

                let eid = as_i64(&ev["eventid"]).unwrap();
                let clock = as_i64(&ev["clock"]).unwrap();

                if eid > eventid.load(Ordering::Relaxed) {
                    eventid.store(eid, Ordering::Relaxed);
                }

                from.store(end_time.unwrap_or(clock), Ordering::Relaxed);

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

pub fn as_i64(value: &Value) -> Option<i64> {
    value.as_str().and_then(|x| x.parse().ok())
}
