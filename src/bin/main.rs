#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;

use clap::{App, Arg};
use clap::AppSettings::{ArgRequiredElseHelp, DeriveDisplayOrder, VersionlessSubcommands};
use cmd_lib;
use bip39::{Bip39, KeyType, Language};
use easy_hasher::easy_hasher::*;
use std::str;
use ploscan::scan::config;
use ploscan::scan::config::load_cfg;
use ploscan::scan::logger;
use ploscan::scan::miner_power;
use futures::Future;
use tokio::runtime::Builder;
use ploscan::scan::miner;
use std::thread;
use std::sync::{Arc, Mutex};
use hex;
use ploscan::scan::scanbool::scanbool;
use ploscan::com::client;


fn main() {

    let arg = App::new("ploscan")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(ArgRequiredElseHelp)
        .setting(DeriveDisplayOrder)
        .setting(VersionlessSubcommands)
    .arg(
        Arg::with_name("pid")
        .short("p")
        .long("pid_create")
        .value_name("pid_create")
        .help("create the pid ")
        .default_value("false")
    ).arg(
        Arg::with_name("start")
        .short("s")
        .long("plotter_scan")
        .value_name("plotter_scan")
        .help("plotter disk and scan ")
        .default_value("false")
    ).arg(
        Arg::with_name("plotter")
            .short("o")
            .long("plotter_create")
            .value_name("plotter_create")
            .help("create plotter file")
            .default_value("false")
    ).arg(
        Arg::with_name("registered")
            .short("r")
            .long("register")
            .value_name("register_miner")
            .help("register miner to service")
            .default_value("false")
    ).arg(
        Arg::with_name("tstart")
            .short("t")
            .long("tstart")
            .value_name("tstart")
            .help("non to ex")
            .default_value("false")
    ).arg(
        Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Location of the config file")
            .takes_value(true)
            .default_value("config.yaml"),
    );

    let matches = &arg.get_matches();
    let config = matches.value_of("config").unwrap();

    let cfg_loaded = load_cfg(config.clone());

    logger::init_logger(&cfg_loaded);

    debug!("{:?}",cfg_loaded);
    let ispid = value_t!(matches, "pid", bool).unwrap_or_else(|e| e.exit());

    //生成pid 打印
    if ispid {
        let (p,m) = get_pid();
        println!("--------------- p={} m={}",p,m);
        return //https:app.slack.com/client/TEHTVS1L6/C019UFEACBT?cdn_fallback=2
    }

    //检测配置文件
    if cfg_loaded.pid == 0 || cfg_loaded.mnemonic == "" || cfg_loaded.mac == ""{
        error!("pid,mnemonic,mac is 0 ,need create");
        return
    }

    //是否P盘
    let plo = value_t!(matches, "plotter", bool).unwrap_or_else(|e| e.exit());
    if plo {
        plotter_file(config.clone());
        return;
    }

    //是否向矿池注册矿工
    let reg  = value_t!(matches,"registered",bool).unwrap_or_else(|e| e.exit());
    if reg {
        regist(config.clone())
    }

    //无预期算力
    let t_start = value_t!(matches, "tstart", bool).unwrap_or_else(|e| e.exit());
    if t_start {
        non_start(config.clone())
    }

    //是否启动扫盘
    let start = value_t!(matches, "start", bool).unwrap_or_else(|e| e.exit());
    if start {
        start_online(config.clone())
    }

    println!("ploscan v.{}", crate_version!());
}

fn plotter_file(config: &str){
    println!("start plotter file...");
    let cfg_loaded= load_cfg(config.clone());
    let cfg= load_cfg(config.clone());
    let macp = cfg_loaded.mac.clone();
    let mut  mp = miner_power::MinerPower::new(cfg,macp);
    let (_,total) = miner_power::scan_plots(&cfg_loaded.plot_dirs.clone(),
                                            cfg_loaded.hdd_use_direct_io.clone(),
                                            cfg_loaded.benchmark_io().clone());
    if cfg_loaded.fileSize < 1f64 {
        println!("fileSize is Less than 1");
        return;
    }

    let nonces = (cfg_loaded.fileSize.clone() * 1024f64 / 256f64) as u64;
    println!("nonces--->{}",nonces.clone());
    mp.plotter_power(total,nonces,cfg_loaded.plot_dir.clone(),
                     cfg_loaded.mem.clone(),cfg_loaded.threads.clone(),cfg_loaded.pid.clone());
}

fn regist(config: &str) {
    println!("register to service");

    let cfg= load_cfg(config.clone());
    let (disk,_,cpuid,_,mem)
        = get_hardware_info(&cfg.plot_dir.clone());
    let pid = format!("0x{:x}",cfg.pid.clone());
    let ump = client::UpdateMinerPar{
        mac:cfg.mac.clone(),
        account_id:pid,
        mnemonic:cfg.mnemonic.clone(),
        cpu:cpuid,
        memory:mem,
        disk:disk,
    };

    let url = cfg.url.clone();
    let timeout = cfg.timeout.clone();

    let client = client::Client::new(
        url,
        timeout,
        0 as usize,
        cfg.mac.clone(),
    );

    //向服务器注册矿工
    match client.update_miner(&ump) {
        Ok(r) => {
            println!("Registered miner successfully --->{:?}",r)
        }
        Err(e) => {
            println!("Failed to register miner--> {:?}",e)
        }
    }
}


fn non_start(config: &str){
    let cfg = load_cfg(config.clone());
    let cfg_loaded = load_cfg(config.clone());
    let mac = cfg.mac.clone();

    let mut scan = Arc::new(Mutex::new(scanbool::new()));


    let  mp = miner_power::MinerPower::new(cfg_loaded,mac.clone());
    let  miner_power = Arc::new(Mutex::new(mp));

    let rt = Builder::new().core_threads(1).build().unwrap();
    let m = miner::Miner::new(cfg, rt.executor(),mac.clone(),miner_power,scan);
    m.run();
    rt.shutdown_on_idle().wait().unwrap();
}

fn start_online(config: &str) {
    info!("start scan and plotter ...");

    let cfg = load_cfg(config.clone());
    let cfgs = load_cfg(config.clone());
    let cfg_loaded= load_cfg(config.clone());

    let pid = cfg_loaded.pid.clone();
    let mnemonic = cfg_loaded.mnemonic.clone();


    let (disk,_,cpuid,_,mem)
        = get_hardware_info(&cfg_loaded.plot_dir);

    let mac = cfg_loaded.mac.clone();
    let macp = cfg_loaded.mac.clone();
    let macs = cfg_loaded.mac.clone();
    let macu = cfg_loaded.mac.clone();

    // fmt::LowerHex
    let  mp = miner_power::MinerPower::new(cfg_loaded,macp);
    let  mps = miner_power::MinerPower::new(cfgs,macs);

    let  miner_power = Arc::new(Mutex::new(mp));
    let  mpm = Arc::new(Mutex::new(mps));

    let mut scan = Arc::new(Mutex::new(scanbool::new()));
    let mut rscan = scan.clone();

    thread::spawn( move ||{
        let pid = format!("0x{:x}",pid);
        mpm.lock().unwrap().update_power(macu,pid,mnemonic,cpuid,mem,disk,scan)
    });

    let rt = Builder::new().core_threads(1).build().unwrap();
    let m = miner::Miner::new(cfg, rt.executor(),mac,miner_power,rscan);
    m.run();
    rt.shutdown_on_idle().wait().unwrap();
}

//生成pid
fn get_pid()->(u64,String){
     let kt = KeyType::for_word_length(12).unwrap();
     let bip39 = match Bip39::new(&kt, Language::English, "") {
         Ok(b) => b,
         Err(e) => {
             println!("e: {}", e);
            return (0,"".to_owned())
         }
     };
    let phrase = &bip39.mnemonic;
     // let seed = &bip39.seed;

    let hash = sha256(phrase);
    let data = hash.to_vec();
    let pu:Vec<u8> = vec![data[5],data[4],data[3],data[2],data[1],data[0]];
    let x = hex::encode(pu);
    let pid:u64 =u64::from_str_radix(&x, 16).unwrap();

    return (pid,phrase.to_owned());
}

//return 所属目录硬盘容量 硬盘可用空间KB cpuid cpu线程 内存GB 网卡
fn get_hardware_info(path:&str)->(String,u64,String,String,String){
    let space=match cmd_lib::run_fun!("df {}",path){
        Ok(s) => s.to_owned(),
        Err(e) => {
            println!("Error getting disk information {:?}",e);
            "1".to_owned()
        }
    };

    if space == "1"{
        std::process::exit(0x0100);
    }

    let  split: Vec<&str>=space.split("on").collect();
    let mut  disk: Vec<&str> = split[1].trim().split(" ").collect();
    disk.retain(|&x| x!="");
    let  total = disk[1].parse::<i64>().unwrap() as f64;
    let t = ((total/1024f64/1024f64/1024f64)*1000.0).round()/1000.0;

    let  free = disk[3].parse::<i64>().unwrap() as u64;

    let  cpu_id = match cmd_lib::run_fun!("cat /proc/cpuinfo | grep name | cut -f2 -d: | uniq -c"){
        Ok(ref s) => {
            if s.trim()==""{
                "arm".to_owned()
            } else {
                s.trim().to_owned()
            }
        }
        Err(_) => "arm".to_owned()
    };

    let cpu_threads = match cmd_lib::run_fun!("cat /proc/cpuinfo | grep \"processor\" | sort | uniq | wc -l"){
        Ok(ref n) => n.to_owned(),
        Err(_) => "1".to_owned()
    };


    let memory = match cmd_lib::run_fun!("cat /proc/meminfo | grep MemTotal"){
        Ok(ref m) => {
            let  split: Vec<&str>=m.split(":").collect();
            let  sp: Vec<&str> = split[1].trim().split(" ").collect();
            let me=sp[0].parse::<f64>().unwrap();
            let ro=((me / 1024.0 / 1024.0)*100.0).round()/100.0;
            ro.to_string()
        },
        Err(e) =>{
            println!("Error getting memory information,{:?}",e);
            "1".to_owned()}
    };

    if memory == "1" {
        println!("not getting memory");
        std::process::exit(0x0100);
    }

    // let  mac = get_mac();
    return (t.to_string(),free,cpu_id,cpu_threads,memory)
}

fn get_mac()->String{
    let gmc = |et:&str,sh:&str| ->String{
        match cmd_lib::run_fun!("{}",sh){
            Ok(ref m) => {
                let  split:Vec<&str>=m.split(" ").collect();
                for i in 0..split.len(){
                    if split[i]==et{
                        return split[i+1].to_owned()
                    }
                }
            }
            Err(e) => {
                println!("Err getting mac information {:?}",e)
            }
        };
        return "".to_owned()
    };
    let mut mac= gmc("HWaddr","ifconfig eth0 |grep HWaddr");
    if mac == "" {
        mac = gmc("ether","ifconfig enp2s0 |grep ether");
    }
    if mac == "" {
        mac = gmc("ether","ifconfig enp3s0 |grep ether");
    }

    if mac == "" {
        println!("mac is not geting");
        std::process::exit(0x0100);
    }
   return mac
}



#[cfg(test)]
mod tests{
    use std::sync::{mpsc};
    use std::sync::mpsc::channel;
    use std::thread;
    use std::sync::{Arc, Mutex};

    use std::time;

    extern crate rustc_serialize;
    // 引入rustc_serialize模块
    use rustc_serialize::json;

    extern crate rand;
    use rand::Rng;


    #[test]
    fn test_hard(){
        let path = String::from("/nda");
        let space=match cmd_lib::run_fun!("df {}",path){
            Ok(s) => s.to_owned(),
            Err(e) => {
                println!("Error getting disk information {:?}",e);
                "1".to_owned()
            }
        };

        if space == "1"{
            std::process::exit(0x0100);
        }

        let  split: Vec<&str>=space.split("on").collect();
        let mut  disk: Vec<&str> = split[1].trim().split(" ").collect();
        disk.retain(|&x| x!="");
        let  total = disk[1].parse::<i64>().unwrap() as f64;
        let t = ((total/1024f64/1024f64/1024f64)*1000.0).round()/1000.0;

        let  free = disk[3].parse::<i64>().unwrap() as u64;

        let  cpu_id = match cmd_lib::run_fun!("cat /proc/cpuinfo | grep name | cut -f2 -d: | uniq -c"){
            Ok(ref s) => {
                if s.trim()==""{
                    "arm".to_owned()
                } else {
                    s.trim().to_owned()
                }
            }
            Err(_) => "arm".to_owned()
        };

        let cpu_threads = match cmd_lib::run_fun!("cat /proc/cpuinfo | grep \"processor\" | sort | uniq | wc -l"){
            Ok(ref n) => n.to_owned(),
            Err(_) => "1".to_owned()
        };


        let memory = match cmd_lib::run_fun!("cat /proc/meminfo | grep MemTotal"){
            Ok(ref m) => {
                let  split: Vec<&str>=m.split(":").collect();
                let  sp: Vec<&str> = split[1].trim().split(" ").collect();
                let me=sp[0].parse::<f64>().unwrap();
                let ro=((me / 1024.0 / 1024.0)*100.0).round()/100.0;
                ro.to_string()
            },
            Err(e) =>{
                println!("Error getting memory information,{:?}",e);
                "1".to_owned()}
        };

        if memory == "1" {
            println!("not getting memory");
            std::process::exit(0x0100);
        }
        println!("{}",t)
    }

    #[test]
    fn test_json(){

        let (sender, receiver) = mpsc::sync_channel(10);

        let s1=sender.clone();
        // Spawn off an expensive computation
        thread::spawn(move|| {
            s1.send("aaaaa").unwrap();
        });

        let s2=sender.clone();
        // Spawn off an expensive computation
        thread::spawn(move|| {
            thread::sleep(time::Duration::from_millis(2));
            s2.send("bbbbb").unwrap();
        });

        let s3=sender.clone();
        // Spawn off an expensive computation
        thread::spawn(move|| {
            thread::sleep(time::Duration::from_millis(3));
            s3.send("ccccc").unwrap();
        });

        println!("**************************");
        // Do some useful work for awhile

        for _ in 0..3{
            // Let's see what that answer was
            println!("{:?}", receiver.recv().unwrap());

            println!("----------------------");
        }

        let var : Arc<i32> = Arc::new(5);
        let share_var = var.clone();

        // 创建一个新线程
        let new_thread = thread::spawn(move|| {
            println!("pppppppppppppppppppppppp");
            println!("share value in new thread: {}, address: {:p}", share_var, &*share_var);
        });

        // 等待新建线程先执行
        new_thread.join().unwrap();

        println!("share value in main thread: {}, address: {:p}", var, &*var);
    }

    #[test]
    fn test_arc(){
        #[derive(Debug,Clone)]
        pub struct TestStruct  {
            pub data_int: u8,
            pub data_str: String,
            pub data_vector: Vec<u8>,
        }

        let mut object = TestStruct {
            data_int: 1,
            data_str: "homura".to_string(),
            data_vector: vec![2,3,4,5],
        };

        // // Serialize using `json::encode`
        // // 将TestStruct转意为字符串
        // let encoded = json::encode(&object).unwrap();
        // println!("{}",encoded);
        // // Deserialize using `json::decode`
        // // 将json字符串中的数据转化成TestStruct对应的数据，相当于初始化
        // let decoded: TestStruct = json::decode(&encoded).unwrap();
        // println!("{:?}",decoded.data_vector);


        let var  =  Arc::new(Mutex::new(object.clone()));
        let mut share_var = var.clone();


        // 创建一个新线程
        let data= thread::spawn(move|| {
            let mut rng =rand::thread_rng();
            loop {
                let mut t=share_var.clone();
                let mut data = t.lock().unwrap();
                data.data_int=rng.gen::<u8>();
                println!("data_int---->{}",data.data_int);
                drop(data);
                thread::sleep(time::Duration::from_millis(1));
            }
        });


        // 创建一个新线程
        let mut share=thread::spawn(move|| {
            loop{
                println!("share value in main thread: {}, address: {:p}", var.lock().unwrap().data_int, &*var);
                thread::sleep(time::Duration::from_millis(1))
            }
        });

        data.join().unwrap();
        println!("--------------");
        share.join().unwrap();
    }


}


// //检测是否P盘完成，如未完成则开始P盘，并开启挖矿 plot_dir,&cpuid,&url,&req_nonces,&pid
// fn start_plot_scan(plot_dir: PathBuf,cpuid:&str,url:&str,result_url: &str, req_nonces: u64,pid: u64)->(u64,u64){
//
//     let plot_dirs=vec![plot_dir];
//
//     let  use_direct_io = match cpuid {
//         "arm" => false,
//         _ =>true
//     };
//
//     let (plots,global_capacity)=miner::scan_plots(&plot_dirs,use_direct_io,false);
//
//     let mut nonces = 0u64;
//     let mut account_id = 0u64;
//
//     //todo:此处需要计算是否有未完成的P盘文件,并开始p盘
//     for (_,v) in plots{
//         for  p in v.as_slice() {
//             let plot=p.lock().unwrap();
//             println!("plot----->{:?}  capacity--->{} path={}",plot,global_capacity,plot.path);
//
//             account_id=plot.meta.account_id;
//             if nonces < plot.meta.nonces{
//                 nonces=plot.meta.nonces;
//             }
//         }
//     }
//     let numeric_id = 111;
//     let start_nonce = 0;
//     let mut buf = vec![0; 1 * plotter::NONCE_SIZE as usize];
//
//     poc_hashing::noncegen_rust(&mut buf, 0, numeric_id, start_nonce, 1);
//     let mut hasher = Sha256::new();
//     hasher.input(&buf);
//     println!("hash--->{}",hasher.result_str());
//
//     let cores = sys_info::cpu_num().unwrap();
//     let memory = sys_info::mem_info().unwrap();
//     let disk = sys_info::hostname().unwrap();
//     let os=sys_info::os_type().unwrap();
//
//     println!("os--->{}",os);
//     println!("hostname--->{}",disk);
//     println!("cores--->{}",cores);
//     println!("memory--->{:?}",memory);
//
//
//     let plo = Plotter::new();
//
//     let start = Instant::now();
//
//     plo.run(PlotterTask{
//         numeric_id,
//         start_nonce,
//         nonces:500u64,
//         output_path:"/home/sf/plotters".to_owned(),
//         mem:"100MB".to_owned(),
//         cpu_threads:2,
//         direct_io: true,
//         async_io:true,
//         quiet:false,
//         benchmark:false,
//         zcb:false,
//     });
//     println!("time cost: {:?} sec",start.elapsed().as_secs());
//     println!("time cost: {:?} ms", start.elapsed().as_millis());// ms
//     println!("time cost: {:?} us", start.elapsed().as_micros());// us
//     println!("time cost: {:?} ns", start.elapsed().as_nanos());// us
//
//     return (account_id,nonces)
// }

// curl -i -H 'Content-Type:application/json' -d '{"method":"eth_updateMiner","params":["02:42:b9:00:1b:ba","0x8eb24392c2bdbb61L","wo are you","Intel(R) Xeon(R) CPU E5649@2.53GHz","16","0.916"],"id":10}' http://47.112.224.137:8546

// let arg = App::new("ploscan")
// .version(crate_version!())
// .author(crate_authors!())
// .about(crate_description!())
// .setting(ArgRequiredElseHelp)
// .setting(DeriveDisplayOrder)
// .setting(VersionlessSubcommands)
// .arg(
// Arg::with_name("numeric_id")
// .short("i")
// .long("id")
// .value_name("numeric_ID")
// .help("your numeric Account ID is pid")
// .takes_value(true)
// .required(false),
// ).arg(
// Arg::with_name("ratio")
// .short("r")
// .long("ratio")
// .value_name("ratio")
// .help("p disk ratio")
// .takes_value(true)
// .required(true),
// ).arg(
// Arg::with_name("path")
// .short("p")
// .long("path")
// .value_name("path")
// .help("target path for plotfile (optional)")
// .takes_value(true)
// .required(true),
// ).arg(
// Arg::with_name("url")
// .short("u")
// .long("url")
// .value_name("url")
// .help("Mine pool url address")
// .takes_value(true)
// .required(true),
// ).arg(
// Arg::with_name("result_url")
// .short("o")
// .long("result_url")
// .value_name("result_url")
// .help("resultUrl of ploter and scanplo submit ")
// .takes_value(true)
// .required(true),
// );

// //检测url是否可用
// fn is_Url(url:&str)->bool{
//     match Url::parse(&url){
//
//         Ok(u) => {
//             let resp=ureq::post(url).timeout(Duration::from_secs(5))
//                 .send_json(serde_json::json!(&api::GetMiningInfoRequest{
//                 method: &String::from("eth_getMinerInfo"),
//                 id:  555,
//             })).into_string().unwrap();
//             println!("Update_miner-----> {}",resp);
//
//             match result_info(resp) {
//                 Ok(r) => {
//                     true
//                 },
//                 Err(e) => {
//                     false
//                 }
//             }
//         }
//         Err(e) => {
//             println!("Err URL format error :{:?}",e);
//             false
//         }
//     }
// }

// fn result_info(resp: String) -> Result<api::MiningInfoResponse,serde_json::Error> {
//     serde_json::from_str(&resp)
// }
