pub mod task;
use futures::stream::{self, StreamExt};
use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{RwLock, mpsc::Receiver};
use tracing::Instrument;

use crate::{
    app::task::{data_update, load_groups, new_group},
    models::{GroupInfo, Severity, Status},
    repository::{HMessage, Repository},
    zabbix_api::ZbxApi,
};

pub const URL: &str = "http://172.30.0.153/api_jsonrpc.php";
pub const LIMIT: usize = 2000;

pub type Group = Arc<RwLock<GroupInfo>>;
pub type GroupType = Arc<RwLock<HashMap<String, Group>>>;

pub async fn data_handler(repo: Repository, mut rx: Receiver<HMessage>) {
    let groups = Arc::new(RwLock::new(load_groups(repo.clone()).await));
    tracing::debug!("Data: {groups:#?}");
    let mut interval = tokio::time::interval(Duration::from_secs(600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let groups = groups.read().await.iter().map(|(k,v)| (k.clone(), v.clone())).collect::<Vec<(String, Group)>>();
                stream::iter(groups)
                    .for_each_concurrent(5, |(name, group_info)| {
                        task(repo.clone(), name.clone(), group_info).instrument(tracing::info_span!("task", group = %name))
                    })
                    .await;
            }
            msg = rx.recv() => {
                if let Some(HMessage::Group(g)) = msg {
                    tracing::debug!("New group: {g}");
                    if !groups.read().await.contains_key(&g) {
                        tokio::spawn(new_group(repo.clone(), g, groups.clone()));
                    } else {
                        tracing::info!("Group {g} already exists");
                    }
                }
            }
        }
    }
}

pub async fn task(db: Repository, group_name: String, group: Group) {
    tracing::info!("Start");
    let mut group_meta = group.write().await;

    let mut length;
    let mut resolved = HashMap::new();
    let mut problems = VecDeque::new();
    let to = time::OffsetDateTime::now_utc().unix_timestamp();

    let mut ev = ZbxApi::get_events::<Value>();

    ev.hostids(&group_meta.hosts);
    ev.until(to);
    ev.limit(LIMIT);

    loop {
        let this_eid;
        let this_from;

        ev.from(&group_meta.last_start);
        ev.eventid(&group_meta.last_event);

        match ev.clone().get().await {
            Ok(result) => {
                length = result.len();
                this_eid = result
                    .last()
                    .and_then(|x| as_i64(&x["eventid"]).map(|x| x + 1));
                this_from = result.last().and_then(|x| as_i64(&x["clock"]));
                tracing::info!(
                    "Number of Events: {length}; last eventid: {this_eid:?}; last_from: {this_from:?}"
                );

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
            Err(er) => {
                tracing::error!("Parse error: {er:?}");
                break;
            }
        };

        if !problems.is_empty() {
            let len = problems.len();
            let mut count = 0;
            let mut eventids = Vec::with_capacity(len);
            let mut host = Vec::with_capacity(len);
            let mut severities = Vec::with_capacity(len);
            let mut triggers = Vec::with_capacity(len);
            let mut start_times = Vec::with_capacity(len);
            let mut opdatas = Vec::with_capacity(len);
            let mut end_times = Vec::with_capacity(len);
            let mut statuses = Vec::with_capacity(len);

            while let Some(ev) = problems.pop_front()
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
                host.push(ev["hosts"][0]["host"].as_str().unwrap().to_string());
                severities.push(Severity::from_number(
                    ev["severity"]
                        .as_str()
                        .and_then(|x| x.parse::<i32>().ok())
                        .unwrap(),
                ));
                triggers.push(ev["name"].as_str().unwrap().to_string());
                start_times.push(as_i64(&ev["clock"]).unwrap());
                opdatas.push(ev["opdata"].as_str().unwrap().to_string());
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
                            (eventid, host, severity, trigger_name, start_time, opdata, end_time, status)
                        SELECT * FROM UNNEST(
                            $1::bigint[],
                            $2::text[],
                            $3::severity_level[],
                            $4::text[],
                            $5::bigint[],
                            $6::text[],
                            $7::bigint[],
                            $8::event_status[]
                        )
                        "#,
            )
            .bind(&eventids)
            .bind(&host)
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
            tracing::info!("There are events as resolved: {resolved:#?}");
            data_update(db.clone(), &group_name, resolved.drain().collect()).await;
        }

        if let Some(t) = this_eid {
            group_meta.last_event = t;
        }

        if let Some(t) = this_from {
            group_meta.last_start = t;
        }

        if LIMIT != length {
            break;
        }
    }
}

pub fn as_i64(value: &Value) -> Option<i64> {
    value.as_str().and_then(|x| x.parse().ok())
}
