use futures::executor::block_on;
use std::{thread,time::Duration};

pub fn exeasync(){
    // tokio::future::poll_fn(async_main());
    // tokio::runtime::Runtime::block_on(async_main);
    // block_on(async_main());
    // tokio::future::poll_fn(async_main());

}
#[tokio::main]
async fn async_main() {
    let f1 = singWait();
    let f2 = dance();

    // future::
    // futures::join!(f1, f2);
}

async fn singWait() {
    sing().await;
}
async fn sing() {
    thread::sleep(Duration::new(5,0));
    println!("************sing*************")
}
async fn dance() {
    println!("************dance*************")
}


