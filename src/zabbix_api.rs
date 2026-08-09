use crate::{
    app::URL,
    models::api_zbx::{DataErrorApiZbx, ZbxError, ZbxGroup, ZbxHost, ZbxResponse},
};
use reqwest::RequestBuilder;
use serde::Deserialize;
use serde_json::json;

pub struct ZbxApi;

impl ZbxApi {
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
            req.post(URL)
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
            req.post(URL)
                .header("Authorization", format!("Bearer {token}"))
                .json(&body_get_goup_id),
        )
        .await
        {
            Ok(mut resp) => Ok(resp.pop().unwrap()),
            Err(er) => Err(er),
        }
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
