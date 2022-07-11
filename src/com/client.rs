use crate::com::api::*;
use futures::stream::Stream;
use futures::Future;
use reqwest::r#async::{Client as InnerClient, ClientBuilder, Decoder};
use std::cmp::Ordering;
use std::mem;
use std::time::Duration;
use url::Url;
// use async_std::task;
// use crate::future::prio_retry::Error;

extern crate log;

extern crate ureq;


/// A client for communicating with Pool/Proxy/Wallet.
#[derive(Clone, Debug)]
pub struct Client {
    inner: InnerClient,
    base_uri: Url,
    total_size_gb: usize,
    mac: String,
}

/// Parameters ussed for nonce submission.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmissionParameters {
    pub account_id: u64,
    pub nonce: u64,
    pub height: u64,
    pub block: u64,
    pub deadline_unadjusted: u64,
    pub deadline: u64,
    pub gen_sig: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdateMinerPar {
    pub mac: String,
    pub account_id: String,
    pub mnemonic: String,
    pub cpu: String,
    pub memory: String,
    pub disk: String
}

/// Usefull for deciding which submission parameters are the newest and best.
/// We always cache the currently best submission parameters and on fail
/// resend them with an exponential backoff. In the meantime if we get better
/// parameters the old ones need to be replaced.
impl Ord for SubmissionParameters {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.block < other.block {
            Ordering::Less
        } else if self.block > other.block {
            Ordering::Greater
        } else if self.gen_sig == other.gen_sig {
            // on the same chain, best deadline wins
            if self.deadline <= other.deadline {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        } else {
            // switched to a new chain
            Ordering::Less
        }
    }
}

impl PartialOrd for SubmissionParameters {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Whether to send additional data for Proxies.
// #[derive(Clone, PartialEq, Debug)]
// pub enum ProxyDetails {
//     /// Send additional data like capacity, miner name, ...
//     Enabled,
//     /// Don't send any additional data:
//     Disabled,
// }



impl Client {
    // fn ua() -> String {
    //     "Scavenger/".to_owned() + crate_version!()
    // }


    /// Create   a new client communicating with Pool/Proxy/Wallet.
    pub fn new(
        base_uri: Url,
        timeout: u64,
        total_size_gb: usize,
        mac: String,
    ) -> Self {

        let client = ClientBuilder::new()
            .timeout(Duration::from_millis(timeout))
            .build()
            .unwrap();

        Self {
            inner: client,
            base_uri,
            total_size_gb,
            mac: mac,
        }
    }

    /// Get current mining info.  Future<Item = MiningInfoResponse, Error = FetchError>
    pub fn get_mining_info(&self) -> impl Future<Item = MiningInfoResponse, Error = FetchError> {
        self.inner
            .post(self.uri_for(""))
            .json(&GetMiningInfoRequest {
                method: &String::from("eth_getMinerInfo"),
                id:5
            })
            .send()
            .and_then(|mut res| {
                let body = mem::replace(res.body_mut(), Decoder::empty());
                body.concat2()
            })
            .from_err::<FetchError>()
            .and_then(|body| match parse_json_result(&body) {
                Ok(x) => Ok(x),
                Err(e) => Err(e.into()),
            })
    }

    pub fn uri_for(&self, path: &str) -> Url {
        let mut url = self.base_uri.clone();
        url.path_segments_mut()
            .map_err(|_| "cannot be base")
            .unwrap()
            .pop_if_empty()
            .push(path);
        url
    }

    /// Submit nonce to the pool and get the corresponding deadline.
    pub fn submit_nonce(
        &self,
        submission_data: &SubmissionParameters,
    ) -> impl Future<Item = SubmitNonceResponse, Error = FetchError> {

        self.inner
            .post(self.uri_for(""))
            .json(&SubmitNonceRequest {
                method: &String::from("eth_addNonce"),
                id:  666,
                params: (submission_data.account_id,submission_data.nonce,submission_data.deadline,
                         submission_data.height,hex::encode(submission_data.gen_sig))//pid,nonce,height,deadline,gensig
            })
            .send()
            .and_then(|mut res| {
                let body = mem::replace(res.body_mut(), Decoder::empty());
                body.concat2()
            })
            .from_err::<FetchError>()
            .and_then(|body| match parse_json_result(&body) {
                Ok(x) => Ok(x),
                Err(e) => Err(e.into()),
            })
    }

    //提交更新矿工信息,不存在则注册
    pub fn update_miner(&self,um: &UpdateMinerPar) -> Result<ResultsUpdateMiner,serde_json::Error>{
        let url = self.base_uri.clone();

        let resp=ureq::post(url.as_str()).timeout(Duration::from_secs(5))
            .send_json(serde_json::json!(&UpdateMinerRequest {
                method: &String::from("eth_updateMiner"),
                id:  555,
                params: (um.mac.to_owned(),um.account_id.to_owned(),um.mnemonic.to_owned(),
                         um.cpu.to_owned(),um.memory.to_owned(),um.disk.to_owned())
            })).into_string().unwrap();
        println!("Update_miner-----> {}",resp);

        serde_json::from_str(&resp)
    }

    //获取矿工预期算力
    pub fn get_expected_power(&self) -> (f64,bool){

        let url = self.base_uri.clone();
        let mac = self.mac.clone();
        let resp=ureq::post(url.as_str()).timeout(Duration::from_secs(5))
            .send_json(serde_json::json!(&GetMinerExPowerRequest {
                    method: &String::from("eth_getPower"),
                    id: 222,
                })).into_string().unwrap();

      match result_power(resp) {
          Ok(ex) => {
              match ex.result.get(&mac) {
                  Some(power) => {
                      match power.get("ExpectPower") {
                          Some(x) => {
                                if x != "" {
                                    match x.parse() {
                                        Ok(f) => {
                                            (f,true)
                                        }
                                        Err(e) => {
                                            println!("String to f64 is error={}",e);
                                            (0f64,true)
                                        }
                                    }
                                }else {
                                    (0f64,true)
                                }
                          }
                          None => {
                              println!("ExpectPower is not");
                              (0f64,true)
                          }
                      }
                  }
                  None => {
                      println!("the mac is not in here mac= {}",mac);
                      (0f64,false)
                  }
              }
          }
          err=> {
              error!("requet error {:?}",err);
              (0f64,false)
          }
      }
    }

    //更新矿工算力
    pub fn update_actual_power(&self, power: f64){

        let url = self.base_uri.clone();
        let mac = self.mac.clone();
        let p = power.to_string();

        let resp=ureq::post(url.as_str()).timeout(Duration::from_secs(5))
            .send_json(serde_json::json!(&UpdateMinerPowerRequest {
                    method: &String::from("eth_updateExistPower"),
                    id: 222,
                    params: (mac,p)
                })).into_string().unwrap();

        println!("Update the actual computing power to the server and return the result eth_updateExistPower---> {}",resp);
    }
}

fn result_power(resp: String) -> Result<MinerExPowerResponse,serde_json::Error> {
    serde_json::from_str(&resp)
}


#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use crate::com::api::MinerExPowerResponse;
    use futures::future::Err;
    use tokio::io::Error;


    extern crate ureq;
    extern crate serde_json;

    static BASE_URL: &str = "http://192.168.1.193:8545";

    #[test]
    fn test_Update_miner() {
        let client = Client::new(
            BASE_URL.parse().unwrap(),
            5000,
            12,
            "pppp".to_owned()
        );
        let up = &UpdateMinerPar{
                mac: "pppp".to_owned(),
                account_id: "0x290".to_owned(),
                mnemonic: "are you ok".to_owned(),
                cpu: "x5".to_owned(),
                memory: "16".to_owned(),
                disk: "3.7".to_owned()
        };
        match client.update_miner(up) {
            Ok(r) => {
                println!("true")
            }
            Err(e) => {
                println!("e--> {}",e)
            }
        }
    }

    #[test]
    fn test_json(){
        let str =serde_json::json!(&UpdateMinerPowerRequest {
                    method: &String::from("eth_updateExistPower"),
                    id: 222,
                    params: ("sdafda".to_string(),85.856.to_string())
                });
        println!("{}",str);
    }

    #[test]
    fn test_get_expected_power() {
        info!("{}",56546);
        let client = Client::new(
            BASE_URL.parse().unwrap(),
            5000,
            12,
            "sssss".to_owned()
        );

       let (power,_)= client.get_expected_power();
        println!("{}",power);

        client.update_actual_power(59.06f64);
    }

    #[test]
    fn test_get_power() {

        let resp = ureq::post("http://192.168.1.193:8545")
            .send_json(  serde_json::json!(&GetMinerExPowerRequest {
                    method: &String::from("eth_getPower"),
                    id: 222,
                }));

        if resp.ok() {
            match resp.into_string() {
                Ok(str) => {
                    println!("{}", str);

                    let results = || -> Result<MinerExPowerResponse,serde_json::Error>{
                        serde_json::from_str(&str)
                    }();

                    match results {
                        Ok(ex) => {
                            println!("{:?}",ex.id)
                        }
                        Error=> {
                            println!("{:?}",Error)
                        }
                    }
                }
                str => {
                    println!("{:?}", str)
                }
            }
        }
    }

    #[test]
    fn test_submit_params_cmp() {
        let submit_params_1 = SubmissionParameters {
            account_id: 1337,
            nonce: 12,
            height: 112,
            block: 0,
            deadline_unadjusted: 7123,
            deadline: 1193,
            gen_sig: [0; 32],
        };

        let mut submit_params_2 = submit_params_1.clone();
        submit_params_2.block += 1;
        assert!(submit_params_1 < submit_params_2);

        let mut submit_params_2 = submit_params_1.clone();
        submit_params_2.deadline -= 1;
        assert!(submit_params_1 < submit_params_2);

        let mut submit_params_2 = submit_params_1.clone();
        submit_params_2.gen_sig[0] = 1;
        submit_params_2.deadline += 1;
        assert!(submit_params_1 < submit_params_2);

        let mut submit_params_2 = submit_params_1.clone();
        submit_params_2.deadline += 1;
        assert!(submit_params_1 > submit_params_2);
    }

    #[test]
    fn test_my_requests(){
        let mut rt = tokio::runtime::Runtime::new().expect("can't create runtime");

        let client = Client::new(
            BASE_URL.parse().unwrap(),
            5000,
            12,
            "aafd".to_owned()
        );
        println!("{:?}",client);
    }

    #[test]
    fn test_myreqminin(){

        let mut rt = tokio::runtime::Runtime::new().expect("can't create runtime");

        let client = Client::new(
            BASE_URL.parse().unwrap(),
            5000,
            12,
            "aafd".to_owned()
        );

        match rt.block_on(client.get_mining_info()) {
            Err(e) => panic!(format!("can't get mining info: {:?}", e)),
            Ok(mining_info) => println!("*****>{:?}",mining_info.result)
        };

    }

    fn log_submission_failed(account_id: u64, nonce: u64, deadline: u64, err: &str) {
        warn!(
            "{: <80}",
            format!(
                "submission failed, retrying: account={}, nonce={}, deadline={}, description={}",
                account_id, nonce, deadline, err
            )
        );
    }

    #[test]
    fn test_requestssubmission() {

        let mut rt = tokio::runtime::Runtime::new().expect("can't create runtime");

        let client = Client::new(
            BASE_URL.parse().unwrap(),
            5000,
            12,
            "aafd".to_owned()
        );

        // this fails if pinocchio switches to a new block height in the meantime
        let nonce_submission_response = rt.block_on(client.submit_nonce(&SubmissionParameters {
            account_id: 1337,
            nonce: 12,
            height: 56,
            block: 1,
            deadline_unadjusted: 7123,
            deadline: 1193,
            gen_sig: [0; 32],
        }));

        println!("nonce_submission_response----->{:?}",nonce_submission_response);

        if let Err(e) = nonce_submission_response {
            assert!(false, format!("can't submit nonce: {:?}", e));
        }
    }

    #[test]
    fn test_str(){
        let ua = || {
            "Scavenger/".to_owned()+crate_version!()
        };
        let uu = ua();
        println!("uu-----> {}",uu.to_owned());
        println!("{}",ua());
    }
}
