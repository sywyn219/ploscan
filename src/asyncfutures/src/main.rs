use futures::FutureExt;
use async_std::task;
use std::{thread,time};
mod timefuture;
mod executor;
mod asyncawait;
mod jsonrpc;
mod http_server;
mod wake_executor;

async fn say_hello() {
    task::sleep(time::Duration::from_secs(5)).await;
    println!("Hello, world!");
}
async fn after() {
    println!("******");
}

async fn tday() -> u64 {
    80
}

async fn all() {
    after().await;
    say_hello().await;
}

fn main() {

    let pid = 4567896u64;
    let pid = format!("0x{:x}",pid);
    println!("{}",pid);

    task::block_on(all());
    let a = task::block_on(tday());

    println!("----------");
    println!("{}",a);
    return;

    let a = (0.21556 * 1024f64 * 1024f64  / 256f64);
        let b = a as u64;
    println!("{}", a);
    println!("{}", b);




    let power = (0.5 * 1024f64 * 1024f64 / 256f64) as u64;
    let mut ex_power = 0u64;

    println!("power ---> {}",power);

    let mut  v = Vec::new();
    let mut  plo1 = plotter{nonces: 1000};
    let mut  plo2 = plotter{nonces: 200};
    let mut plo3 =  plotter{nonces: 3000};
    let mut plo4 =  plotter{nonces: 40000};

    v.push(plo1);
    v.push(plo2);
    v.push(plo3);
    v.push(plo4);

    for plo in v.iter_mut()  {

        if ex_power >= power {
            plo.nonces=0;
            continue
        }
        ex_power = ex_power + plo.nonces;
        if ex_power > power {
            plo.nonces = plo.nonces - (ex_power - power);
        }
    }

    println!("{:?}",v)

}

#[derive(Debug)]
struct plotter {
    nonces: u64
}



fn runexecutor(){
    executor::run_executor();
}
