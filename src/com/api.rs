use bytes::Buf;
use reqwest::r#async::Chunk;
use serde::de::{DeserializeOwned};
use std::collections::HashMap;

//向节点提交
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitNonceRequest<'a> {
    pub method: &'a str,
    pub id: u32,
    pub params:(u64,u64,u64,u64,String)//pid,nonce,height,deadline,gin
}

//更新矿机 mac,pid,mnemonic,cpu,memory,disk string
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMinerRequest<'a> {
    pub method: &'a str,
    pub id: u32,
    pub params:(String,String,String,String,String,String)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMiningInfoRequest<'a> {
    pub method: &'a str,
    pub id: u32
}

//获取矿工预期算力
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMinerExPowerRequest<'a> {
    pub method: &'a str,
    pub id: u32,
}

//更新矿工实际算力
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMinerPowerRequest<'a> {
    pub method: &'a str,
    pub id: u32,
    pub params:(String,String) //1,mac; 2,power
}

//获取矿工算力响应
#[derive(Clone, Debug, Serialize,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinerExPowerResponse {
    pub jsonrpc: String,
    pub id: u32,
    pub result: HashMap<String,HashMap<String,String>>
}

//更新矿工算力响应
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MinerPowerResponse {
    pub jsonrpc: String,
    pub id: u32,
    pub result: HashMap<String,String>
}


//节点返回
///
/// {
//     "jsonrpc": "2.0",
//     "id": 37021,
//     "result": {
//         "accountID": 564987979,
//         "blockHeight": 3,
//         "deadline": 679279588,
//         "generationSignature": "e1639bb2a8f9626a45689fddb7158559c4f1fd81c3e3ba7ad9b3a37611726f78",
//         "nonce": 0
//     }
// }
#[derive(Debug,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitNonceResponse {
    pub jsonrpc: String,
    pub id: u32,
    pub result: ResultsSubmit
}


#[derive(Debug,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultsSubmit{
    pub accountID: u64,
    pub blockHeight: u64,
    pub deadline: u64,
    pub generationSignature: String,
    pub nonce: u64
}

//注册矿工响应
#[derive(Debug,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultsUpdateMiner{
    pub jsonrpc: String,
    pub id: u32,
    pub result: HashMap<String,Miner>
}

#[derive(Debug,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Miner{
    pub pid: u64,
    pub mac: String,
    pub ip: String,
    pub cpu: String,
    pub memory: String,
    pub disk: String,
    pub mnemonic: String,
    pub timestamp: String
}

#[derive(Debug,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultsMining{
    pub baseTarget: u64,
    pub blockHeight: u64,
    pub generationSignature: String,
}


#[derive(Debug,Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiningInfoResponse {
    pub jsonrpc: String,
    pub id: u32,
    pub result: ResultsMining
}

// fn default_target_deadline() -> u64 {
//     std::u64::MAX
// }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PoolErrorWrapper {
    error: PoolError,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug)]
pub enum FetchError {
    Http(reqwest::Error),
    Pool(PoolError),
}

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> FetchError {
        FetchError::Http(err)
    }
}

impl From<PoolError> for FetchError {
    fn from(err: PoolError) -> FetchError {
        FetchError::Pool(err)
    }
}

// MOTHERFUCKING pool
// fn from_str_or_int<'de, D>(deserializer: D) -> Result<u64, D::Error>
// where
//     D: de::Deserializer<'de>,
// {
//     struct StringOrIntVisitor;
//
//     impl<'de> de::Visitor<'de> for StringOrIntVisitor {
//         type Value = u64;
//
//         fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
//             formatter.write_str("string or int")
//         }
//
//         fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
//             v.parse::<u64>().map_err(de::Error::custom)
//         }
//
//         fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
//             Ok(v)
//         }
//     }
//
//     deserializer.deserialize_any(StringOrIntVisitor)
// }

pub fn parse_json_result<T: DeserializeOwned>(body: &Chunk) -> Result<T, PoolError> {
    match serde_json::from_slice(body.bytes()) {
        Ok(x) => Ok(x),
        _ => match serde_json::from_slice::<PoolErrorWrapper>(body.bytes()) {
            Ok(x) => Err(x.error),
            _ => {
                let v = body.to_vec();
                Err(PoolError {
                    code: 0,
                    message: String::from_utf8_lossy(&v).to_string(),
                })
            }
        },
    }
}
