use std::{collections::HashMap, sync::Arc};

use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::Row;
use tokio::sync::RwLock;

use crate::{
    app::{URL, as_i64},
    models::{
        GroupInfo,
        api_zbx::{DataErrorApiZbx, ZbxError, ZbxResponse},
    },
    repository::Repository,
};

pub async fn load_groups(repo: Repository) -> HashMap<String, Arc<RwLock<GroupInfo>>> {
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
    .fetch_all(&*repo)
    .await
    .unwrap();

    resp.into_iter()
        .map(|x| {
            let et: i64 = x.get("latest_end");
            let st: i64 = x.get("latest_start");
            (
                x.get("group"),
                Arc::new(RwLock::new(GroupInfo::new(st, et, x.get("hosts")))),
            )
        })
        .collect::<HashMap<_, _>>()
}

pub async fn data_update(db: Repository, mut resolved: HashMap<i64, i64>) -> bool {
    let events = db.get_unresolved_events().await.unwrap();

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
                let r_eventid = as_i64(&i["r_eventid"]).unwrap();
                vec_eventid.push(r_eventid);
                vec_end_time.push(resolved.remove(&r_eventid).unwrap());
            }
            sqlx::query(
                r"
                UPDATE events e
                SET 
                    status = 'resolved'::event_statis,
                    end_time = tmp.end_time
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
            true
        }
        Err(er) => {
            tracing::error!("{er}");
            false
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
            .header("Authorization", format!("Bearer {token}"))
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
    let token = std::env::var("TOKEN").unwrap();
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

    match request_reqwest_handle::<Vec<Value>>(
        req.post(URL)
            .header("Authorization", format!("Bearer {token}"))
            .json(&d),
    )
    .await
    {
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

    match res.json::<ZbxResponse<O>>().await? {
        ZbxResponse::Ok { result, .. } => Ok(result),
        ZbxResponse::Err {
            error:
                DataErrorApiZbx {
                    code,
                    message,
                    data,
                },
            ..
        } => Err(ZbxError::Api {
            kind: code.into(),
            data,
            message,
        }),
    }
}
