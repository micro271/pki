use std::{collections::HashMap, sync::Arc};

use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;

use crate::{
    app::{GroupType, URL, as_i64},
    models::{
        EventId, Timestamp,
        api_zbx::{ApiZbxResponse, DataErrorApiZbx, ErrorApiZbxResponse, ZbxError},
    },
    repository::Repository,
};

pub async fn load_groups(
    client: Repository,
) -> HashMap<String, (Arc<Timestamp>, Arc<EventId>, Vec<i64>)> {
    let resp = sqlx::query(
        r#"
        SELECT 
            g.group_name as group,
            array_agg(h.host_id) as hosts,
            MAX(ev.start_time) as latest_start,
            MAX(ev.end_time) as latest_end,
            MAX(ev.eventid) as latest_eventid
        FROM zbx_groups g
        JOIN zbx_group_host gh ON (g.group_name = gh.group_name)
        JOIN zbx_hosts h ON (gh.host = h.host)
        JOIN events ev ON (h.host = ev.host)
        GROUP BY g.group_name
    "#,
    )
    .fetch_all(&*client)
    .await
    .unwrap();

    resp.into_iter()
        .map(|x| {
            let et: i64 = x.get("latest_end");
            let st: i64 = x.get("latest_start");
            let tm = Arc::new(Timestamp::new(et.max(st)));
            (
                x.get("group"),
                (
                    tm,
                    Arc::new(EventId::new(x.get("latest_eventid"))),
                    x.get("hosts"),
                ),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub async fn new_group(group: String, groups: GroupType) {
    let now = (time::OffsetDateTime::now_utc() - time::Duration::days(30)).unix_timestamp();
    let h = fetch_hostids_from_zbx_api(&group).await;
    groups.write().await.insert(
        {
            tracing::info!("New group added: {group}");
            group
        },
        (Arc::new(Timestamp::new(now)), Arc::new(EventId::new(0)), h),
    );
}

pub async fn data_update(db: Repository, mut resolved: HashMap<i64, i64>) {
    let events_id = db.get_eventids().await.unwrap();

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
    match request_reqwest_handle::<Vec<Value>>(
        req.post(URL)
            .json(&d)
            .header("Authentication", format!("Bearer {token}")),
    )
    .await
    {
        Ok(result) => {
            let mut vec_eventid = Vec::with_capacity(result.len());
            let mut vec_end_time = Vec::with_capacity(result.len());

            for i in result {
                let r_eventid = as_i64(&i["r_eventid"]).unwrap();
                vec_eventid.push(r_eventid);
                vec_end_time.push(resolved.remove(&r_eventid).unwrap());
            }
            sqlx::query(
                r"
                UPDATE events e
                SET 
                    status = 'resolved'::event_statis,
                    end_time = tmp.end_time,
                FROM (
                    SELECT * 
                    FROM UNNEST (
                        $1::bigint[],
                        $2::bigint[]
                    ) as tmp(eventid, end_time)
                )
                WHERE e.eventid = tmp.eventid
            ",
            )
            .bind(&vec_eventid)
            .bind(&vec_end_time)
            .execute(&*db)
            .await
            .unwrap();
        }
        Err(er) => {
            tracing::error!("{er}");
        }
    }
}

pub async fn fetch_hostids_from_zbx_api(group: &str) -> Vec<i64> {
    let token = std::env::var("TOKEN").unwrap();
    let body_get_goup_id = json!({
        "jsonrpc":"2.0",
        "method":"hostgroup.get",
        "params":{
                "output":["groupid","name"],
                "filter":{
                    "name": group
                }
        },
        "id":1
    });

    let req = reqwest::Client::new();

    match request_reqwest_handle::<Vec<Value>>(
        req.post(URL)
            .header("Authentication", format!("Bearer {token}"))
            .json(&body_get_goup_id),
    )
    .await
    {
        Ok(resp) => fetch_hostids(as_i64(&resp[0]["groupid"]).unwrap()).await,
        Err(er) => {
            tracing::error!("{er}");
            Vec::new()
        }
    }
}

pub async fn fetch_hostids_with_group_name(db: Repository, group: &str) -> Vec<i64> {
    let groupid: i64 = db.get_groupid_by_name(group).await.unwrap();

    fetch_hostids(groupid).await
}

pub async fn fetch_hostids(groupid: i64) -> Vec<i64> {
    let d = json!({
        "jsonrpc": "2.0",
        "method": "host.get",
        "params": {
            "output": ["hostid", "host", "name"],
            "groupids": groupid,
            "selectHostGroups": ["groupid", "name"]
        },
        "id": 1
    });

    let req = reqwest::Client::new();

    match request_reqwest_handle::<Vec<Value>>(req.post(URL).json(&d)).await {
        Ok(result) => result
            .into_iter()
            .map(|x| as_i64(&x["hostid"]).unwrap())
            .collect::<Vec<i64>>(),
        Err(er) => {
            tracing::error!("{er}");
            Vec::new()
        }
    }
}

async fn request_reqwest_handle<O: for<'de> Deserialize<'de>>(
    req: RequestBuilder,
) -> Result<O, ZbxError> {
    let res = req.send().await?;

    if res.status().is_success() {
        let ApiZbxResponse { result, .. } = res.json::<ApiZbxResponse<O>>().await?;
        Ok(result)
    } else {
        let ErrorApiZbxResponse {
            error:
                DataErrorApiZbx {
                    code,
                    message,
                    data,
                },
            ..
        } = res.json::<ErrorApiZbxResponse>().await?;
        Err(ZbxError::Api {
            kind: code.into(),
            data,
            message,
        })
    }
}
