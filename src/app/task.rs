use std::{collections::HashMap, sync::Arc};

use serde_json::{Value, json};
use sqlx::{PgPool, Row};

use crate::{
    app::{GroupType, URL},
    models::{EventId, Timestamp},
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
            MAX(ev.start_tine) as latest_start,
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
        group,
        (Arc::new(Timestamp::new(now)), Arc::new(EventId::new(0)), h),
    );
}

pub async fn data_update(db: Repository, mut resolved: HashMap<i64, i64>) {
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

    let resp = req
        .post(URL)
        .header("Authentication", format!("Bearer {token}"))
        .json(&body_get_goup_id)
        .send()
        .await
        .unwrap();
    let resp = &mut resp.json::<Value>().await.unwrap();
    let resp = resp.as_object_mut().unwrap();

    let gids = resp["result"][0]["groupid"].as_i64().unwrap();

    fetch_hostids(gids).await
}

pub async fn fetch_hostids_with_group_name(db: Arc<PgPool>, group: &str) -> Vec<i64> {
    let groupid: i64 = sqlx::query("SELECT groupid FROM zbx_groups WHERE group_name = $1")
        .bind(group)
        .fetch_one(&*db)
        .await
        .unwrap()
        .get("groupid");

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
    let resp = req.post(URL).json(&d).send().await.unwrap();
    let mut resp = resp.json::<Value>().await.unwrap();
    let resp = resp.as_object_mut().unwrap();

    resp.remove("result")
        .unwrap()
        .as_array()
        .unwrap()
        .into_iter()
        .map(|x| x["hostid"].as_i64().unwrap())
        .collect::<Vec<_>>()
}
