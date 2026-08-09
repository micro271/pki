pub mod task;
use futures::stream::{self, StreamExt};
use serde_json::{Value, from_value, json};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{RwLock, mpsc::Receiver};
use tracing::Instrument;

use crate::{
    app::task::{data_update, fetch_hostids_from_zbx_api, load_groups},
    models::{GroupInfo, Severity, Status, api_zbx::ZbxResponse},
    repository::{HMessage, Repository},
};

pub const URL: &str = "http://172.30.0.153/api_jsonrpc.php";
pub const LIMIT: usize = 2000;

pub type Group = Arc<RwLock<GroupInfo>>;
pub type GroupType = Arc<RwLock<HashMap<String, Group>>>;

pub async fn data_handler(repo: Repository, mut rx: Receiver<HMessage>) {
    let groups = Arc::new(RwLock::new(load_groups(repo.clone()).await));
    let mut interval = tokio::time::interval(Duration::from_secs(600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let groups = groups.read().await.iter().map(|(k,v)| (k.clone(), v.clone())).collect::<Vec<(String, Group)>>();
                stream::iter(groups)
                    .for_each_concurrent(5, |(name, group_info)| {
                        task(repo.clone(), group_info).instrument(tracing::info_span!("task", group = %name))
                    })
                    .await;
            }
            msg = rx.recv() => {
                if let Some(HMessage::Group(g)) = msg {
                    tracing::debug!("New group: {g}");
                    if !groups.read().await.contains_key(&g) {
                        let from = (time::OffsetDateTime::now_utc() - time::Duration::days(30)).unix_timestamp();
                        let hostids = fetch_hostids_from_zbx_api(&g).await;
                        groups.write().await.insert(g, Arc::new(RwLock::new(GroupInfo::new(from, 0, hostids))));
                    } else {
                        tracing::info!("Group {g} already exists");
                    }
                }
            }
        }
    }
}

pub async fn task(db: Repository, groups: Group) {
    let token = std::env::var("TOKEN").unwrap();
    let mut group_meta = groups.write().await;

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
                    "hostids": &group_meta.hosts,
                    "time_from": &group_meta.last_start,
                    "time_till": to,
                    "selectHosts": ["hostid","host"],
                    "selectRelatedObject": ["triggerid","description","priority"],
                    "sortfield": ["clock", "eventid"],
                    "sortorder":"ASC",
                    "eventid_from": &group_meta.last_event,
                    "limit": LIMIT
            },
            "id":1
        });
        let req = reqwest::Client::default();
        let resp = req
            .post(URL)
            .header("Authorization", format!("Bearer {token}"))
            .json(&d)
            .send()
            .await
            .unwrap();

        let mut this_eid = None;
        let mut this_from = None;
        match resp.json::<ZbxResponse<Vec<Value>>>().await {
            Ok(ZbxResponse::Ok { result, .. }) => {
                length = result.len();
                this_eid = result
                    .last()
                    .and_then(|x| as_i64(&x["eventid"]).map(|x| x + 1));
                this_from = result.last().and_then(|x| as_i64(&x["clock"]));
                for ev in result {
                    if as_i64(&ev["value"]).unwrap() == 1 {
                        problems.push_back(ev);
                    } else {
                        resolved.insert(
                            as_i64(&ev["eventid"]).unwrap(),
                            as_i64(&ev["clock"]).unwrap(),
                        );
                    }
                }
            }
            Ok(ZbxResponse::Err { error, .. }) => {
                tracing::error!("{error:?}");
                length = 0;
            }
            Err(er) => {
                tracing::error!("Parse error: {er:?}");
                length = 0;
            }
        };

        if !problems.is_empty() {
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

                if end_time.is_none() && length == LIMIT {
                    problems.push_back(ev);
                    continue;
                }

                eventids.push(as_i64(&ev["eventid"]).unwrap());
                nodos.push(ev["host"].to_string());
                severities.push(from_value::<Severity>(ev["severity"].take()).unwrap());
                triggers.push(ev["trigger"].to_string());
                start_times.push(as_i64(&ev["clock"]).unwrap());
                opdatas.push(ev["opdata"].to_string());
                end_times.push(end_time);
                statuses.push(if end_time.is_some() {
                    Status::Resolved
                } else {
                    Status::Ongoing
                });
            }

            let resp = sqlx::query(
                r#"
                        INSERT INTO events
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
            .await;

            match resp {
                Ok(resp) => {
                    tracing::info!(
                        "Insert Result: {resp:?}, New latest eventid: {this_eid:?}, new latest start: {this_from:?}"
                    );
                }
                Err(er) => {
                    tracing::error!("Insert error: {er:?}");
                    continue;
                }
            }
        }

        if !resolved.is_empty() {
            data_update(db.clone(), resolved.drain().collect()).await;
        }

        group_meta.last_event = this_eid.unwrap();
        group_meta.last_start = this_from.unwrap();
    }
}

pub fn as_i64(value: &Value) -> Option<i64> {
    value.as_str().and_then(|x| x.parse().ok())
}
