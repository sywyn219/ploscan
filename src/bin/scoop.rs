use local::scan::shabal256;
use local::scan::reader;
use local::scan::miner;
use local::plo::poc_hashing;
use crypto::digest::Digest;
use std::convert::TryInto;
use futures::sync::mpsc;

use tokio::runtime::Builder;

#[macro_use]

extern crate crypto;


struct ab {
    bs: u64
}

fn main() {

    let rt = Builder::new().core_threads(1).build().unwrap();

    let ex = rt.executor();

    let (tx_nonce_data, rx_nonce_data) = mpsc::channel(50);

    tx_nonce_data.send(ab{
        bs:450
    });

    ex.spawn( move || {
        rx_nonce_data.readv().unwrap()
    });
    rt.shutdown_on_idle().wait().unwrap();
}

fn calcudeadlien() {
    // 8a39d2abd3999ab73c34db2476849cddf303ce389b35826850f9a700589b4a90
    //d617344ea058868c76b44007d457a9ee4b63533650a9a216a8bf4c34db975bf2
    //d617344ea058868c76b44007d457a9ee4b63533650a9a216a8bf4c34db975bf2
    let mut cache:[u8;262144] = [0u8;262144];
    // let mut cache1 = cache.clone();
    poc_hashing::noncegen_rust(&mut cache,0,199040970012233,7930659,1);
    let str =String::from("00ca0325dc13d006fae376e76aaf83e3f62a0679708691adf326c9d201b9ae4c");

    let mut gen:Vec<u8> = match hex::decode(str){
        Ok(g) => g,
        Err(e) =>{
            println!("hex is error--------------------{:?}",e);
            vec![0u8;32]
        }
    };

    println!("{:?}",gen);
    let mut ges = gen.as_slice().try_into().expect("---------------------------------------------err-");

    const SCOOP_SIZE: usize = 64;
    let scoop = &cache[3547 * SCOOP_SIZE..3547 * SCOOP_SIZE + SCOOP_SIZE];

    let mut sha256 = crypto::sha2::Sha256::new();
    sha256.input(scoop);
    let rt = sha256.result_str();
    println!("rt----->{}",rt);

    let deadline = shabal256::shabal256_deadline_fast(scoop,&ges);
    println!("deadline----> {}",deadline);

}