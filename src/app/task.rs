use std::{collections::HashMap, sync::Arc};

use serde_json::{Value, json};
use sqlx::Row;
use tokio::sync::RwLock;

use crate::{
    app::{GroupType, URL, as_i64},
    models::{GroupInfo, Status},
    repository::Repository,
    zabbix_api::{ZbxApi, request_reqwest_handle},
};

pub async fn load_groups(repo: Repository) -> HashMap<String, Arc<RwLock<GroupInfo>>> {
    let resp = sqlx::query(
        r#"
        select gh.group_name as group,
            array_agg(distinct h.hostid) as hosts,
            max(e.start_time) as latest_start,
            max(e.eventid) as latest_eventid
        from events as e 
            join zbx_group_host as gh on gh.host = e.host 
            join zbx_hosts as h on h.host = e.host
            group by gh.group_name;
        "#,
    )
    .fetch_all(&*repo)
    .await
    .unwrap();

    resp.into_iter()
        .map(|x| {
            let last_eid: i64 = x.get("latest_eventid");
            let last_start: i64 = x.get("latest_start");
            (
                x.get("group"),
                Arc::new(RwLock::new(GroupInfo::new(
                    last_start,
                    last_eid + 1,
                    x.get("hosts"),
                ))),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub async fn data_update(db: Repository, mut resolved: HashMap<i64, i64>) -> bool {
    let events = db.get_unresolved_events().await.unwrap();
    tracing::warn!("Unresolved eventds: {}: {events:#?}", events.len());
    tracing::debug!("events unresolved from now: {events:#?}");
    tracing::debug!("To update {resolved:#?}");

    let event_ids = events.into_iter().map(|x| x.eventid).collect::<Vec<_>>();

    let d = json!({
        "jsonrpc":"2.0",
        "method":"event.get",
            "params":{
                "output":"extend",
                "source":0,
                "object":0,
                "eventids": event_ids,
                "selectHosts": ["hostid","host"],
                "selectRelatedObject": ["triggerid","description","priority"],
                "sortfield": ["clock", "eventid"],
                "sortorder":"ASC"
            },
        "id":1
    });
    let token = std::env::var("TOKEN").unwrap();
    let req = reqwest::Client::new();
    match request_reqwest_handle::<Vec<Value>>(
        req.post(URL)
            .json(&d)
            .header("Authorization", format!("Bearer {token}")),
    )
    .await
    {
        Ok(result) => {
            let mut vec_eventid = Vec::with_capacity(result.len());
            let mut vec_end_time = Vec::with_capacity(result.len());

            for i in result {
                let r_eid = as_i64(&i["r_eventid"]).unwrap();
                if let Some(eid) = resolved.remove(&r_eid) {
                    tracing::debug!("One match: eid: {} - r_eid: {}", eid, r_eid);
                    vec_eventid.push(r_eid);
                    vec_end_time.push(eid);
                }
            }
            tracing::info!("It was updated {} events", vec_end_time.len());
            tracing::warn!(
                "Pending to update: {}",
                event_ids.len() - vec_end_time.len()
            );
            sqlx::query(
                r"
                UPDATE events e
                SET 
                    status = tmp.status,
                    end_time = tmp.end_time
                FROM (
                    SELECT $1::event_status, eid, st
                    FROM UNNEST (
                        $2::bigint[],
                        $3::bigint[]
                    ) AS u(eid, st)
                ) AS tmp(status, eventid, end_time)
                WHERE e.eventid = tmp.eventid
            ",
            )
            .bind(Status::Resolved)
            .bind(&vec_eventid)
            .bind(&vec_end_time)
            .execute(&*db)
            .await
            .unwrap();
            true
        }
        Err(er) => {
            tracing::error!("{er}");
            false
        }
    }
}

pub async fn new_group(repo: Repository, name: String, groups: GroupType) {
    let from = (time::OffsetDateTime::now_utc() - time::Duration::days(30)).unix_timestamp();
    let group = ZbxApi::get_group(&name).await.unwrap();
    tracing::debug!("Group info: {group:?}");
    let hosts = ZbxApi::get_hosts(group.groupid).await.unwrap();
    tracing::debug!("Hosts: {hosts:?}");
    let (names, hids): (Vec<String>, Vec<i64>) =
        hosts.into_iter().map(|x| (x.host, x.hostid)).unzip();

    let mut repo = repo.begin().await.unwrap();

    let tr = async {
        sqlx::query("INSERT INTO zbx_groups (name, groupid) VALUES ($1, $2)")
            .bind(group.name)
            .bind(group.groupid)
            .execute(&mut *repo)
            .await?;

        sqlx::query(
            "INSERT INTO zbx_hosts (host, hostid) SELECT * FROM UNNEST ($1::TEXT[], $2::BIGINT[])",
        )
        .bind(&names)
        .bind(&hids)
        .execute(&mut *repo)
        .await?;

        sqlx::query(
            "INSERT INTO zbx_group_host (group_name, host) SELECT $1, h FROM UNNEST ($2::TEXT[]) AS h",
        )
        .bind(&name)
        .bind(names)
        .execute(&mut *repo)
        .await?;

        Result::<(), sqlx::Error>::Ok(())
    };

    match tr.await {
        Ok(()) => match repo.commit().await {
            Ok(()) => {
                groups
                    .write()
                    .await
                    .insert(name, Arc::new(RwLock::new(GroupInfo::new(from, 0, hids))));
            }
            Err(er) => tracing::error!("{er:?}"),
        },
        Err(er) => {
            tracing::error!("{er:?}");
            if let Err(er) = repo.rollback().await {
                tracing::error!("{:?}", er);
            }
        }
    }
}
