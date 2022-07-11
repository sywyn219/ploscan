
extern crate easy_hasher;

// use crate::scan::miner;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::scan::plot;
use std::fs::read_dir;
use filetime::FileTime;
use crate::plo::plotter;
// use bip39::{Bip39, KeyType, Language};
// use crypto::sha2::Sha256;
// use crypto::digest::Digest;
// use easy_hasher::easy_hasher::*;
use crate::scan::utils::{get_device_id};
use crate::scan::config;
use crate::com;
use std::time::Duration;
use crate::com::client::Client;
use crate::com::client;
use crate::scan::scanbool::scanbool;

/**
    实时修改 算力
*/
//pid===96656851439868 ph===era tomorrow slender bid another push climb clump bind tunnel pottery until
// pub const pid:u64=96656851439868;

pub struct MinerPower {
     cfg: config::Cfg,
     plotter_ing: bool,
     client: com::client::Client
}

impl MinerPower {
    pub fn new(cfg: config::Cfg,mac: String)-> MinerPower{

        let url = cfg.url.clone();
        let timeout = cfg.timeout.clone();

        let client = Client::new(
            url,
            timeout,
            0 as usize,
            mac,
        );

        MinerPower {
            cfg: cfg,
            plotter_ing: false,
            client:client
        }
    }

    //实时P盘修改    //测试边P盘，边扫描文件，是否冲突
    pub fn update_power(&mut self,mac: String,account_id: String,mnemonic: String,cpu: String,memory: String,
                        disk: String,scan: Arc<Mutex<scanbool>>){
        let ump = client::UpdateMinerPar{
            mac,
            account_id,
            mnemonic,
            cpu,
            memory,
            disk,
        };
        loop {
            //获取真实算力
            let (_,total) =
                scan_plots(&self.cfg.plot_dirs,self.cfg.hdd_use_direct_io,self.cfg.benchmark_cpu());

            //本地配置算力，或者从服务器获取
            // let (power,is) = if self.cfg.power_location {
            //     (self.cfg.power as f64,false )
            // }else {
            //     self.client.get_expected_power()
            // };
            let (power,is) = self.client.get_expected_power();

            scan.lock().unwrap().updatePowerEx(( power * 1024f64 * 1024f64 * 1024f64 / 256f64 ) as u64);
            scan.lock().unwrap().update(true);

            if !is {
                //向服务器注册矿工
                match self.client.update_miner(&ump) {
                    Ok(r) => {
                        println!("Registered miner successfully --->{:?}",r)
                    }
                    Err(e) => {
                        println!("Failed to register miner--> {:?}",e)
                    }
                }
            }else {
                let tg = total as f64 * 256f64 / 1024f64 / 1024f64 / 1024f64;
                //向服务器更新算力  单位TB
                self.client.update_actual_power(tg);
            }

            //未在P盘中，则进行P盘检测
            if !self.plotter_ing {
              let ex_power =   ( power * 1024f64 * 1024f64 * 1024f64 / 256f64 ) as u64;
                info!("ex_power_total------> {}  ac_power_total---> {}",ex_power,total);
                if ex_power > total {
                    if ex_power - total > 1000 {
                        let path = self.cfg.plot_dir.clone();
                        let mem = self.cfg.mem.clone();

                        let threads = self.cfg.threads.clone();
                        let pid = self.cfg.pid.clone();

                        //P盘
                        self.plotter_power(total,ex_power-total,path,mem,threads,pid);

                        //获取真实算力
                        let (_,total) =
                            scan_plots(&self.cfg.plot_dirs,self.cfg.hdd_use_direct_io,self.cfg.benchmark_cpu());
                        info!("Scan to real computing power {}",total);

                        // let total = total as f64 * 256f64 / 1024f64 / 1024f64 / 1024f64;
                        // //向服务器更新算力
                        // self.client.update_actual_power(total);
                    }
                }
            }

            std::thread::sleep(Duration::from_secs(100))
        }
    }

    pub fn location_power(&mut self,mac: String,account_id: String,mnemonic: String,cpu: String,memory: String,
                          disk: String){
        let ump = client::UpdateMinerPar{
            mac,
            account_id,
            mnemonic,
            cpu,
            memory,
            disk,
        };
        //向服务器注册矿工
        match self.client.update_miner(&ump) {
            Ok(r) => {
                println!("Registered miner successfully --->{:?}",r)
            }
            Err(e) => {
                println!("Failed to register miner--> {:?}",e)
            }
        }
    }

    //实时扫描P盘结果,用于nonces扫描
    pub fn get_plots_scan(&mut self) -> HashMap<String, Arc<Vec<Mutex<plot::Plot>>>>{
        let (actual_power,total) =
            scan_plots(&self.cfg.plot_dirs,self.cfg.hdd_use_direct_io,self.cfg.benchmark_cpu());
        actual_power
    }

    //p盘
    pub fn plotter_power(&mut self,start: u64,nonces: u64,path: String, mem: String, threads: u8,pid: u64){

        self.plotter_ing = true;


        let p = plotter::Plotter::new();
        p.run(plotter::PlotterTask {
            numeric_id: pid,
            start_nonce: start,
            nonces: nonces,
            output_path: path,
            mem: mem,
            cpu_threads: threads,
            direct_io: true,
            async_io: true,
            quiet: false,
            benchmark: false,
            zcb: false,
        });

        self.plotter_ing = false;
    }

}

pub fn scan_plots(plot_dirs: &[PathBuf],
                  use_direct_io: bool,
                  dummy: bool,) -> (HashMap<String, Arc<Vec<Mutex<plot::Plot>>>>, u64) {
    let mut drive_id_to_plots: HashMap<String, Vec<Mutex<plot::Plot>>> = HashMap::new();
    let mut global_capacity: u64 = 0;

    for plot_dir in plot_dirs {
        let mut num_plots = 0;
        let mut local_capacity: u64 = 0;
        for file in read_dir(plot_dir).unwrap() {
            let file = &file.unwrap().path();

            if let Ok(p) = plot::Plot::new(file, use_direct_io, dummy) {
                let drive_id = get_device_id(&file.to_str().unwrap().to_string());
                let plots = drive_id_to_plots.entry(drive_id).or_insert(Vec::new());

                local_capacity += p.meta.nonces as u64;
                plots.push(Mutex::new(p));
                num_plots += 1;
            }
        }

        // info!(
        //     "path={}, files={}, size={:.4} TiB",
        //     plot_dir.to_str().unwrap(),
        //     num_plots,
        //     local_capacity as f64 / 4.0 / 1024.0 / 1024.0
        // );

        global_capacity += local_capacity;
        if num_plots == 0 {
            warn!("no plots in {}", plot_dir.to_str().unwrap());
        }
    }

    // sort plots by filetime and get them into an arc
    let drive_id_to_plots: HashMap<String, Arc<Vec<Mutex<plot::Plot>>>> = drive_id_to_plots
        .drain()
        .map(|(drive_id, mut plots)| {
            plots.sort_by_key(|p| {
                let m = p.lock().unwrap().fh.metadata().unwrap();
                -FileTime::from_last_modification_time(&m).unix_seconds()
            });
            (drive_id, Arc::new(plots))
        })
        .collect();

    // info!(
    //     "plot files loaded: total drives={}, total capacity={:.4} TiB",
    //     drive_id_to_plots.len(),
    //     global_capacity as f64 / 4.0 / 1024.0 / 1024.0
    // );

    (drive_id_to_plots, global_capacity)
}

//生成pid
// fn get_pid()->(u64,String){
//     let kt = KeyType::for_word_length(12).unwrap();
//     let bip39 = match Bip39::new(&kt, Language::English, "") {
//         Ok(b) => b,
//         Err(e) => {
//             println!("e: {}", e);
//             return (0,"".to_owned())
//         }
//     };
//     let phrase = &bip39.mnemonic;
//     // let seed = &bip39.seed;
//
//     let hash = sha256(phrase);
//     // let string_hash = hash.to_hex_string();
//     let data = hash.to_vec();
//     let pu:Vec<u8> = vec![data[5],data[4],data[3],data[2],data[1],data[0]];
//     let x = hex::encode(pu);
//     let pd:u64 =u64::from_str_radix(&x, 16).unwrap();
//
//     return (pd,phrase.to_owned());
// }

#[cfg(test)]
mod tests {
    // use crate::scan::miner_power::get_pid;
    //
    // #[test]
    // fn pid(){
    //     for _ in 0..100 {
    //         let (p,ph) = get_pid();
    //         println!("pid==={} ph==={}",p,ph)
    //     }
    //
    // }
}



