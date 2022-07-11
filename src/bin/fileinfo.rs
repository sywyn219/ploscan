#![warn(unused_extern_crates)]

use ploscan::plo::utils::{free_disk_space, get_sector_size, preallocate,remove_nonces};
use ploscan::plo::writer::{create_writer_thread, read_resume_info, write_resume_info};
use std::path::{Path, PathBuf};
use std::ops::Deref;
use std::fs::remove_file;
use ploscan::scan::miner;

use std::sync::{Arc,Mutex};
use std::collections::HashMap;
use ploscan::scan::plot;
use crossbeam_channel::{bounded,Sender};
use std::thread;
use std::time::Duration;

#[macro_use]
extern crate log;

pub const SCOOP_SIZE: u64 = 64;
pub const NUM_SCOOPS: u64 = 4096;
pub const NONCE_SIZE: u64 = SCOOP_SIZE * NUM_SCOOPS;

fn main() {


    // let k:f64 = 10000f64 * 256_f64 / 1024f64 / 1024f64;
    // let y:f64 = k * 1024f64;
    // println!("{}",k);
    // println!("{}",y);
    // let x:f64 = 10f64 / 3f64;
    // println!("{}", x);
    // arc_power();
    // cross_channel()
    // outinner()
    scan_file()
    // plotter_file()
}
use tokio::runtime::Builder;
use ploscan::scan::reader::Reader;

// fn tokio_task() {
//     let rt = Builder::new().core_threads(1).build().unwrap();
//     rt.executor().clone().spawn(
//         aa()
//     );
// }

async fn aa() {
    println!("---------")
}

fn arc_power() {

    //
    // let mut power = miner_power::miner_power::new();
    // let mut arc_power = Arc::new(Mutex::new(power));
    //
    // let mut arc_power1 = arc_power.clone();
    // let mut arc_power2 = arc_power.clone();
    // let mut arc_power3 = arc_power.clone();
    // let mut arc_power4 = arc_power.clone();
    //
    // let th1 = thread::spawn(move || {
    //     for _ in 0..100 {
    //         arc_power1.lock().unwrap().up_actual_power(10);
    //         // println!("{}",arc_power1.lock().unwrap().actual_power)
    //     }
    // });
    // let th2 = thread::spawn(move || {
    //     for _ in 0..100 {
    //         arc_power2.lock().unwrap().up_actual_power(10);
    //         // println!("{}",arc_power1.lock().unwrap().actual_power)
    //     }
    // });
    // let th3 = thread::spawn(move || {
    //     for _ in 0..100 {
    //         arc_power3.lock().unwrap().up_actual_power(10);
    //         // println!("{}",arc_power1.lock().unwrap().actual_power)
    //     }
    // });
    // let th4 = thread::spawn(move || {
    //     for _ in 0..100 {
    //         arc_power4.lock().unwrap().up_actual_power(10);
    //         // println!("{}",arc_power1.lock().unwrap().actual_power)
    //     }
    // });
    // th1.join();
    // th2.join();
    // th3.join();
    // th4.join();
    // println!("th1");
    // println!(".....{:?}",arc_power.lock().unwrap().get_miner_power());
}


//test crossbeam_channel
fn cross_channel() {
    let (s, r) = bounded(10);
    thread::spawn(|| fibonacci(s));

    // Print the first 20 Fibonacci numbers.
    for num in r.iter().take(20) {
        thread::sleep(Duration::new(2,0));
        println!("{}", num);
    }
}
// Sends the Fibonacci sequence into the channel until it becomes disconnected.
fn fibonacci(sender: Sender<u64>) {
    let (mut x, mut y) = (0, 1);
    while sender.send(x).is_ok() {
      println!("----->{}",x)
    }
}


fn outinner(){
  'fefw:  for i in 0..10  {
        println!("{}",i);
        if i >3 {
            break 'fefw
        }
    }
}

use ploscan::scan::poc_hashing;
fn scan_file(){
    let output_path=String::from("./");
    let mut  path = PathBuf::new();
    path.push(output_path);
    let dirs =vec![path];

    let (drive_id_to_plots, total_size) =
        miner::scan_plots(&dirs, false, false);

    println!("len==={}",drive_id_to_plots.len());

    println!("plots=={:?}",drive_id_to_plots);

    let gin = String::from("00ca0325dc13d006fae376e76aaf83e3f62a0679708691adf326c9d201b9ae4c");
    let gsing= poc_hashing::decode_gensig(&gin);

    let scoop =
        poc_hashing::calculate_scoop(1345, &gsing);

    // let (tx_empty_buffers, rx_empty_buffers) =
    //     crossbeam_channel::bounded(buffer_count as usize);

    drive_id_to_plots.iter().map(move|drive,plots| {
        'outer: for (i_p, p) in plots.iter().enumerate() {
            let mut p = p.lock().unwrap();
            if let Err(e) = p.prepare(scoop) {
                error!(
                    "reader: error preparing {} for reading: {} -> skip one round",
                    p.meta.name, e
                );
                continue 'outer;
            }
                let mut bs=vec![0u8, 32];
                let (bytes_read, start_nonce, next_plot) = match p.read(&mut bs, scoop) {
                    Ok(x) => x,
                    Err(e) => {
                        error!(
                            "reader: error reading chunk from {}: {} -> skip one round",
                            p.meta.name, e
                        );
                        (0, 0, true)
                    }
                };
        }
    });

    //
    // drive_id_to_plots.iter().map(|(drive, plots)| {
    //     create_writer_thread()
    //     let (interupt, task) = if self.show_progress {
    //         self.create_read_task(
    //             Some(pb.clone()),
    //             drive.clone(),
    //             plots.clone(),
    //             height,
    //             block,
    //             base_target,
    //             scoop,
    //             gensig.clone(),
    //             self.show_drive_stats,
    //         )
    //     } else {
    //         self.create_read_task(
    //             None,
    //             drive.clone(),
    //             plots.clone(),
    //             height,
    //             block,
    //             base_target,
    //             scoop,
    //             gensig.clone(),
    //             self.show_drive_stats,
    //         )
    //     };
    //
    //     self.pool.spawn(task);
    //     interupt
    // }).collect();


    // for  (k,v) in drive_id_to_plots{
    //     println!("drive===> {}",k);
    //     let vs = &v[..];
    //     println!("vs -->{:?}",vs);
    // }
//{" 802": [Mutex { data: Plot { meta: Meta { account_id: 200, start_nonce: 0, nonces: 1000, name: "200_0_1000" },
// path: "./200_0_1000", fh: File { fd: 4, path: "/home/sf/code/rust/ploscan/200_0_1000", read: true, write: false },
// read_offset: 0, use_direct_io: false, sector_size: 512, dummy: false } },
// Mutex { data: Plot { meta: Meta { account_id: 100, start_nonce: 0, nonces: 100, name: "100_0_100" },
// path: "./100_0_100", fh: File { fd: 5, path: "/home/sf/code/rust/ploscan/100_0_100", read: true, write: false },
// read_offset: 0, use_direct_io: false, sector_size: 512, dummy: false } }]}

    println!("total_size----> {}",total_size/64);



    let reader_thread_count = 1;
    //
    // let (tx_empty_buffers, rx_empty_buffers) =
    //     crossbeam_channel::bounded(buffer_count as usize);
    // let (tx_read_replies_cpu, rx_read_replies_cpu) =
    //     crossbeam_channel::bounded(cpu_buffer_count);

    //
    // let reader = Reader::new(
    //     drive_id_to_plots,
    //     total_size,
    //     reader_thread_count,
    //     rx_empty_buffers,
    //     tx_empty_buffers,
    //     tx_read_replies_cpu,
    //     true,
    //     true,
    //     false,
    //     false,
    //     Arc::new(Mutex::new(miner_power::miner_power::new())),
    // );


}

//p盘
fn plotter_file(){
    let (numeric_id,start_nonce,nonces) = (200u64,0u64,1000u64);

    let output_path=String::from("./");

    let file = Path::new(&output_path).join(format!(
        "{}_{}_{}",
        numeric_id, start_nonce, nonces
    ));
    let plotsize = nonces * NONCE_SIZE;

    let mut progress = 0;
    if file.exists(){
        let resume_info = read_resume_info(&file);
        match resume_info {
            Ok(x) => {
                progress = x;
                println!("progress is {}",progress)
            },
            Err(_) => {
                println!("Error: couldn't read resume info from file '{}'", file.display());
                // println!("If you are sure that this file is incomplete \
                //               or corrupted, then delete it before continuing.");
                // println!("Shutting Down...");
                match remove_nonces(file.as_path()) {
                    Ok(()) => {
                        println!("delete file {:?}",file.as_path().as_os_str());

                        //预分配
                        preallocate(&file,plotsize,true);
                        if write_resume_info(&file, 0u64).is_err() {
                            println!("Error: couldn't write resume info");
                        }else {
                            println!("create file {:?}",file.as_path().as_os_str())
                        }
                    }
                    Err(error) => {
                        panic!("Error delete the file: {:?}", error)
                    },
                }
            }
        }

    }else {
        preallocate(&file,plotsize,true)
    }
}



//move
fn move_copy() {

    let mut num1 = 5;
    let mut f1 = move |x: i32| {num1 = x + num1;
        println!("nei {}",num1)
    };
    let data1 = f1(num1);
    println!("num1:{:?} data1:{:?}", num1, data1);


    let mut num = 5;
    {
        let mut add_num = |x: i32| num += x;
        add_num(5);
    }
    println!("num----> {}",num);
    assert_eq!(10, num);



    let mut str2 = "julia book".to_string();
    let str2_0 = "i love it";
    let mut f4 = move |x: &str| str2 = str2 + x + str2_0;
    let _str2 = f4(" 2013");
    println!("str2_0:{:?}", str2_0);//无影响
    // println!("str2:{:?}", str2); //=> error: str2也已经被借用了


    let mut str2 = "julia book".to_string();
    let str2_0 = "i love it";
    let mut f4 = |x: &str| str2 = str2 + x + str2_0;
    let _str2 = f4(" 2013");
    println!("str2_0:{:?}", str2_0); //不变化


    let mut mybook = "rust".to_string();
    let comment: &str = "ok?";
    {
        let f = move |x: &str| {
            mybook = mybook + x + comment;
            println!("move=> mybook:{:?}", mybook);
        };
        f("primer is a good book!");
    };
    println!("comment:{:?}", comment);//=> comment仍可用

    let mut mybook = "rust".to_string();
    let comment: &str = "ok?";
    {
        let f = |x: &str| {
            mybook = mybook + x + comment;
            println!("无move=> mybook:{:?}", mybook);
        };
        f("primer is a good book!");
    };
    println!("comment:{:?}", comment); //comment仍可用
    //println!("mybook:{:?}", mybook); //=> 报错，mybook已经被f move走了，不存在
    //println!("mybook:{:?}", mybook); //=> 报错，mybook已经被f move走了，不存在

    //println!("str2:{:?}", str2); //=> error: str2也已经被借用了
}