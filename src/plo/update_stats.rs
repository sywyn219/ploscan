// use std::collections::HashMap;
// use std::mem;
//
// use reqwest::r#async::{Client as InnerClient, ClientBuilder, Decoder};
// use std::time::Duration;
// use futures::{Stream, Future};
// use crate::com::api::FetchError;
// use crate::com::api;
//
//
// extern crate rustc_serialize;
// // 引入rustc_serialize模块
// use rustc_serialize::json;
//
//
// #[derive(RustcDecodable, RustcEncodable,Debug,Clone,Serialize)]
// pub struct stats {
//     pub hardware: HardWare,
//     pub plots: HashMap<String, plot>
// }
//
// #[derive(RustcDecodable, RustcEncodable,Debug,Clone,Serialize)]
// pub struct plot{
//     pub path: String,
//     pub start_nonce: u64,
//     pub nonces: u64,
//     pub size: u64,//mb
//     pub schedule: String,
// }
//
// #[derive(RustcDecodable, RustcEncodable,Debug,Clone,Serialize)]
// pub struct HardWare{
//     pub mac: String,
//     pub pid: u64,
//     pub cpu: String,
//     pub cpu_thread: String,
//     pub mem: String,
//     pub disk: String,
// }
//
// //向管理端提交状态
// #[derive(Clone, Debug, Serialize)]
// #[serde(rename_all = "camelCase")]
// pub struct SubmitStatsRequest<'a> {
//     pub method: &'a str,
//     pub id: u32,
//     pub params:(String)//pid,nonce,height,deadline,gin
// }
//
// impl stats {
//     pub fn new() -> stats {
//         Self {
//             hardware: HardWare{
//                 mac:"".to_owned(),
//                 pid: 0,
//                 cpu: "".to_owned(),
//                 cpu_thread: "".to_owned(),
//                 mem: "".to_owned(),
//                 disk: "".to_owned(),
//             },
//             plots: HashMap::new()
//         }
//     }
//
//     pub fn update_send(&mut self, mut hd: &HardWare){
//         self.hardware = hd.clone();
//     }
//
//     pub fn update_plot_send(&mut self, mut plo: &plot){
//         self.plots.insert(plo.clone().path,plo.clone());
//     }
//
//     pub fn send(&mut self,url: String) {
//         let st = self.clone();
//         let encoded = json::encode(&st).unwrap();
//         // println!("{}",encoded);
//
//         let client = ClientBuilder::new()
//             .timeout(Duration::from_secs(10))
//             .build()
//             .unwrap();
//
//         for _ in 0..3  {
//             println!("------------");
//
//             let response= client.get(&url)
//                 .json(&SubmitStatsRequest {
//                     method: &String::from("update.msg"),
//                     id: 66,
//                     params: (encoded.clone()), })
//                 .send()
//                  .map(|resp| {
//                      println!("status: {}", resp.status());
//                      println!("resp: {:?}", resp)
//                  });
//
//             let mut rt = tokio::runtime::current_thread::Runtime::new().expect("new rt");
//             // tokio::spawn(response);
//              rt.block_on(response);
//             println!("************")
//         }
//     }
// }
//
// // .send() {
// // Ok(t) => {
// //     println!("get from url={} resp->{:?}",url,t);
// //     return;
// // },
// //Insert.Message
// // Err(e) => println!("get from url={} err->{:?}",url,e),
// //
// // _ => println!("get from url={} is err",url),
// // };
// #[cfg(test)]
// mod test {
//     use crate::plo::update_stats;
//     #[test]
//     fn test_update(){
//         // let mut up=update_stats::stats::new();
//         // up.send("http://192.168.1.13:8080".to_owned());
//     }
// }


