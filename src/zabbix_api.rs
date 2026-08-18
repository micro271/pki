use std::marker::PhantomData;

use crate::{
    app::URL,
    models::api_zbx::{DataErrorApiZbx, ZbxAuditLog, ZbxError, ZbxGroup, ZbxHost, ZbxResponse},
};
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub struct ZbxApi;

impl ZbxApi {
    pub fn get_events<T: for<'de> Deserialize<'de>>() -> ZbxApiEvents<T> {
        ZbxApiEvents {
            hostids: None,
            from: None,
            eventid: None,
            until: None,
            asc: true,
            limit: 1000,
            _priv: PhantomData,
        }
    }

    pub async fn get_hosts(groupid: i64) -> Result<Vec<ZbxHost>, ZbxError> {
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
        match request_reqwest_handle::<Vec<ZbxHost>>(
            req.post(&*URL)
                .header("Authorization", format!("Bearer {token}"))
                .json(&d),
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(er) => Err(er),
        }
    }

    pub async fn get_group(name: &str) -> Result<ZbxGroup, ZbxError> {
        let token = std::env::var("TOKEN").unwrap();
        let body_get_goup_id = json!({
            "jsonrpc":"2.0",
            "method":"hostgroup.get",
            "params":{
                    "output":["groupid","name"],
                    "filter":{
                        "name": name
                    }
            },
            "id":1
        });

        let req = reqwest::Client::new();

        match request_reqwest_handle::<Vec<ZbxGroup>>(
            req.post(&*URL)
                .header("Authorization", format!("Bearer {token}"))
                .json(&body_get_goup_id),
        )
        .await
        {
            Ok(mut resp) => Ok(resp.pop().unwrap()),
            Err(er) => Err(er),
        }
    }

    pub async fn get_auditlog() -> Result<ZbxAuditLog, ZbxError> {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct ZbxApiEvents<T> {
    hostids: Option<Value>,
    from: Option<Value>,
    eventid: Option<Value>,
    until: Option<Value>,
    asc: bool,
    limit: usize,
    _priv: PhantomData<T>,
}

impl<D> ZbxApiEvents<D>
where
    D: for<'de> Deserialize<'de>,
{
    pub fn hostids<T: Serialize>(&mut self, hids: T) {
        self.hostids = serde_json::to_value(hids).ok();
    }

    pub fn until<T: Serialize>(&mut self, until: T) {
        self.until = serde_json::to_value(until).ok();
    }

    pub fn from<T: Serialize>(&mut self, from: T) {
        self.from = serde_json::to_value(from).ok();
    }

    pub fn eventid<T: Serialize>(&mut self, eventid: T) {
        self.eventid = serde_json::to_value(eventid).ok();
    }

    pub fn limit(&mut self, limit: usize) {
        self.limit = limit;
    }

    pub fn asc(&mut self) {
        self.asc = true;
    }

    pub fn desc(&mut self) {
        self.asc = false;
    }

    pub async fn get(self) -> Result<Vec<D>, ZbxError> {
        let token = std::env::var("TOKEN").unwrap();
        let sorted = if self.asc { "ASC" } else { "DESC" };
        let until = self.until.unwrap_or_else(|| {
            serde_json::to_value(time::OffsetDateTime::now_utc().unix_timestamp()).unwrap()
        });
        let d = json!({
            "jsonrpc":"2.0",
            "method":"event.get",
            "params":{
                    "output":"extend",
                    "source":0,
                    "object":0,
                    "hostids": self.hostids,
                    "time_from": self.from,
                    "time_till": until,
                    "selectHosts": ["hostid","host"],
                    "selectRelatedObject": ["triggerid","description","priority"],
                    "sortfield": ["clock", "eventid"],
                    "sortorder": sorted,
                    "eventid_from": self.eventid,
                    "limit": self.limit
            },
            "id":1
        });

        let client = reqwest::Client::new();

        request_reqwest_handle::<Vec<D>>(
            client
                .post(&*URL)
                .header("Authorization", format!("Bearer {token}"))
                .json(&d),
        )
        .await
    }
}

pub async fn request_reqwest_handle<O: for<'de> Deserialize<'de>>(
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
