use std::{collections::HashMap, sync::Arc};

use serde_json::{Value, json};
use sqlx::Row;
use tokio::sync::RwLock;

use crate::{
    app::{GroupType, LAST_DAYS, URL, as_i64},
    models::{GroupInfo, HostInfo, HostsInfo, LoadGroup, Status, api_zbx::ZbxApiResourceType},
    repository::Repository,
    zabbix_api::{ZbxApi, request_reqwest_handle},
};

pub async fn load_group(
    repo: Repository,
    group: LoadGroup,
) -> HashMap<String, Arc<RwLock<GroupInfo>>> {
    let query = format!(
        r#"
            select gh.group_name as group,
                array_agg(distinct h.hostid) as hosts,
                coalesce(max(e.start_time), $1) as latest_start,
                coalesce(max(e.eventid), 0) as latest_eventid
            from zbx_group_host as gh 
                join zbx_hosts as h on h.host = gh.host
                left join events as e on e.host = h.host{}
                group by gh.group_name;
        "#,
        if let LoadGroup::Group(_) = &group {
            "\nwhere gh.group_name = $2\n"
        } else {
            ""
        }
    );

    let mut resp = sqlx::query(sqlx::AssertSqlSafe(query))
        .bind((time::OffsetDateTime::now_utc() - time::Duration::days(LAST_DAYS)).unix_timestamp());

    if let LoadGroup::Group(g) = group {
        resp = resp.bind(g);
    };

    let resp = resp.fetch_all(&*repo).await.unwrap();

    resp.into_iter()
        .map(|x| {
            let last_eid: i64 = x.get("latest_eventid");
            let last_start: i64 = x.get("latest_start");
            (
                x.get("group"),
                Arc::new(RwLock::new(GroupInfo::new(
                    last_start,
                    last_eid + 1,
                    {
                        let tmp: Vec<(i64, i64)> = x.get("hosts");
                        HostsInfo::new(
                            tmp.into_iter()
                                .map(|(hid, last_change)| HostInfo::new(hid, last_change))
                                .collect(),
                        )
                    },
                    time::OffsetDateTime::UNIX_EPOCH.unix_timestamp(),
                ))),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub async fn data_update(db: Repository, group: &str, mut resolved: HashMap<i64, i64>) -> bool {
    let events = db.get_unresolved_events(group).await.unwrap();
    tracing::warn!("Unresolved eventds: {}: {events:#?}", events.len());
    tracing::debug!("To update {resolved:#?}");

    let event_ids = events.into_iter().map(|x| x.eventid).collect::<Vec<_>>();

    let token = std::env::var("TOKEN").unwrap();
    let client = reqwest::Client::new();
    let d = json!({
        "jsonrpc":"2.0",
        "method":"event.get",
            "params":{
                "output":["eventid", "r_eventid"],
                "eventids": event_ids,
            },
        "id":1
    });
    let req = request_reqwest_handle::<Vec<Value>>(
        client
            .post(&*URL)
            .json(&d)
            .header("Authorization", format!("Bearer {token}")),
    )
    .await;

    match req {
        Ok(result) => {
            let mut vec_eventid = Vec::with_capacity(result.len());
            let mut vec_end_time = Vec::with_capacity(result.len());
            let mut to_get = HashMap::new();
            for i in result {
                if let Some(r_eid @ 1..) = as_i64(&i["r_eventid"])
                    && let Some(eid) = as_i64(&i["eventid"])
                {
                    if let Some(clock) = resolved.remove(&r_eid) {
                        tracing::debug!("One match: eid: {} - r_eid: {}", eid, r_eid);
                        vec_eventid.push(eid);
                        vec_end_time.push(clock);
                    } else {
                        to_get.insert(r_eid, eid);
                    }
                }
            }
            if !to_get.is_empty() {
                tracing::info!(
                    "{} not found in resolved events, so we going to obtain the r_eventid from the api zabbix",
                    to_get.len()
                );
                let d = json!({
                    "jsonrpc": "2.0",
                    "method": "event.get",
                    "params": {
                        "output": ["eventid", "clock"],
                        "eventids": to_get.keys().collect::<Vec<_>>()
                    },
                    "id": 1
                });

                tracing::debug!("query: {d:#?}");

                let req = request_reqwest_handle::<Vec<Value>>(
                    client
                        .post(&*URL)
                        .json(&d)
                        .header("Authorization", format!("Bearer {token}")),
                )
                .await;

                match req {
                    Ok(resp) => {
                        if resp.len() != to_get.len() {
                            tracing::warn!(
                                "We dont' obtain all event resolved data: events: {} - to obtain: {}",
                                resp.len(),
                                to_get.len()
                            );
                        }

                        for i in resp {
                            if let Some(r_eid) = as_i64(&i["eventid"])
                                && let Some(eid) = to_get.remove(&r_eid)
                                && let Some(end_time) = as_i64(&i["clock"])
                            {
                                vec_eventid.push(eid);
                                vec_end_time.push(end_time);
                            }
                        }
                    }
                    Err(er) => tracing::error!("{er:?}"),
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
    let from = (time::OffsetDateTime::now_utc() - time::Duration::days(LAST_DAYS)).unix_timestamp();
    let group = ZbxApi::get_group(&name).await.unwrap();
    tracing::debug!("Group info: {group:?}");
    let hosts = ZbxApi::get_hosts(group.groupid).await.unwrap();
    tracing::debug!("Hosts: {hosts:?}");
    let (names, hids): (Vec<String>, Vec<i64>) =
        hosts.into_iter().map(|x| (x.host, x.hostid)).unzip();

    let mut repo = repo.begin().await.unwrap();

    let last_change_default = time::OffsetDateTime::UNIX_EPOCH.unix_timestamp();
    let tr = async {
        sqlx::query("INSERT INTO zbx_groups (name, groupid, last_change) VALUES ($1, $2, $3)")
            .bind(group.name)
            .bind(group.groupid)
            .bind(last_change_default)
            .execute(&mut *repo)
            .await?;

        sqlx::query(
            "INSERT INTO zbx_hosts (host, hostid, last_change) SELECT *, $3 FROM UNNEST ($1::TEXT[], $2::BIGINT[])",
        )
        .bind(&names)
        .bind(&hids)
        .bind(last_change_default)
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
                groups.write().await.insert(
                    name,
                    Arc::new(RwLock::new(GroupInfo::new(
                        from,
                        0,
                        HostsInfo::new(
                            hids.into_iter()
                                .map(|x| HostInfo::new(x, last_change_default))
                                .collect(),
                        ),
                        time::OffsetDateTime::now_utc().unix_timestamp(),
                    ))),
                );
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

pub async fn update_group_meta(repo: Repository, groups: GroupType, from: i64) {
    let wr = groups.write().await;

    let mut d = json!({
        "jsonrpc": "2.0",
        "method": "auditlog.get",
        "params": {
            "output": "extend",
            "from_time": from,
            "filter": {
                "resourcetype": ZbxApiResourceType::Host,
                "resourceid": []
            },
            "sortfield": "clock",
            "sortorder": "DESC",
            "limit": 100
        },
        "id": 1
    });
    let client = reqwest::Client::new();
    let token = ";";
    for (group, g_info) in wr.iter() {
        let hids = &g_info.read().await.hosts;
        d["params"]["filter"]["resourceid"] = serde_json::to_value(hids.get_hostids()).unwrap();
    }
    todo!()
}
