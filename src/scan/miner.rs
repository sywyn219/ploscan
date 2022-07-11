use crate::com::api::MiningInfoResponse as MiningInfo;
// use crate::config::Cfg;
use crate::scan::config::Cfg;
use crate::scan::cpu_worker::create_cpu_worker_task;
use crate::future::interval::Interval;
#[cfg(feature = "opencl")]
use crate::gpu_worker::create_gpu_worker_task;
#[cfg(feature = "opencl")]
use crate::gpu_worker_async::create_gpu_worker_task_async;
#[cfg(feature = "opencl")]
use crate::ocl::GpuBuffer;
#[cfg(feature = "opencl")]
use crate::ocl::GpuContext;
use crate::scan::plot::{Plot, SCOOP_SIZE};
use crate::scan::poc_hashing;
use crate::scan::reader::Reader;
use crate::scan::requests::RequestHandler;
use crate::scan::utils::{get_device_id, new_thread_pool};
use crossbeam_channel;
use filetime::FileTime;
use futures::sync::mpsc;
#[cfg(feature = "opencl")]
use ocl_core::Mem;
use std::cmp::{max};
use std::collections::HashMap;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::u64;
use stopwatch::Stopwatch;
use tokio::prelude::*;
use tokio::runtime::TaskExecutor;
use crate::scan::miner_power::MinerPower;
use crate::scan::scanbool::scanbool;

pub struct Miner {
    reader: Reader,
    request_handler: RequestHandler,
    rx_nonce_data: mpsc::Receiver<NonceData>,
    target_deadline: u64,
    account_id_to_target_deadline: HashMap<u64, u64>,
    state: Arc<Mutex<State>>,
    scan: Arc<Mutex<scanbool>>,
    reader_task_count: usize,
    get_mining_info_interval: u64,
    executor: TaskExecutor,
    wakeup_after: i64,
}
// #[derive(Clone,Debug)]
pub struct State {
    generation_signature: String,
    generation_signature_bytes: [u8; 32],
    height: u64,
    block: u64,
    account_id_to_best_deadline: HashMap<u64, u64>,
    base_target: u64,
    sw: Stopwatch,
    scanning: bool,
    processed_reader_tasks: usize,
    scoop: u32,
    first: bool,
    outage: bool,
}

impl State {
    fn new() -> Self {
        Self {
            generation_signature: "".to_owned(),
            height: 0,
            block: 0,
            scoop: 0,
            account_id_to_best_deadline: HashMap::new(),
            base_target: 1,
            processed_reader_tasks: 0,
            sw: Stopwatch::new(),
            generation_signature_bytes: [0; 32],
            scanning: false,
            first: true,
            outage: false,
        }
    }

    fn update_mining_info(&mut self, mining_info: &MiningInfo) {
        for best_deadlines in self.account_id_to_best_deadline.values_mut() {
            *best_deadlines = u64::MAX;
        }
        self.height = mining_info.result.blockHeight;
        self.block += 1;
        self.base_target = mining_info.result.baseTarget;
        self.generation_signature_bytes =
            poc_hashing::decode_gensig(&mining_info.result.generationSignature);
        self.generation_signature = mining_info.result.generationSignature.clone();

        let scoop =
            poc_hashing::calculate_scoop(mining_info.result.blockHeight, &self.generation_signature_bytes);
        info!(
            "{: <80}",
            format!("new block: height={}, scoop={}", mining_info.result.blockHeight, scoop)
        );
        self.scoop = scoop;

        self.sw.restart();
        self.processed_reader_tasks = 0;
        self.scanning = true;
    }
}

#[derive(Debug)]
pub struct NonceData {
    pub height: u64,
    pub block: u64,
    pub base_target: u64,
    pub deadline: u64,
    pub nonce: u64,
    pub reader_task_processed: bool,
    pub account_id: u64,
}

pub trait Buffer {
    fn get_buffer(&mut self) -> Arc<Mutex<Vec<u8>>>;
    fn get_buffer_for_writing(&mut self) -> Arc<Mutex<Vec<u8>>>;
    #[cfg(feature = "opencl")]
    fn get_gpu_buffers(&self) -> Option<&GpuBuffer>;
    #[cfg(feature = "opencl")]
    fn get_gpu_data(&self) -> Option<Mem>;
    fn unmap(&self);
    fn get_id(&self) -> usize;
}

pub struct CpuBuffer {
    data: Arc<Mutex<Vec<u8>>>,
}

impl CpuBuffer {
    pub fn new(buffer_size: usize) -> Self {
        let pointer = aligned_alloc::aligned_alloc(buffer_size, page_size::get());
        let data: Vec<u8>;
        unsafe {
            data = Vec::from_raw_parts(pointer as *mut u8, buffer_size, buffer_size);
        }
        CpuBuffer {
            data: Arc::new(Mutex::new(data)),
        }
    }
}

impl Buffer for CpuBuffer {
    fn get_buffer(&mut self) -> Arc<Mutex<Vec<u8>>> {
        self.data.clone()
    }
    fn get_buffer_for_writing(&mut self) -> Arc<Mutex<Vec<u8>>> {
        self.data.clone()
    }
    #[cfg(feature = "opencl")]
    fn get_gpu_buffers(&self) -> Option<&GpuBuffer> {
        None
    }
    #[cfg(feature = "opencl")]
    fn get_gpu_data(&self) -> Option<Mem> {
        None
    }
    fn unmap(&self) {}
    fn get_id(&self) -> usize {
        0
    }
}

pub fn scan_plots(
    plot_dirs: &[PathBuf],
    use_direct_io: bool,
    dummy: bool,
) -> (HashMap<String, Arc<Vec<Mutex<Plot>>>>, u64) {
    let mut drive_id_to_plots: HashMap<String, Vec<Mutex<Plot>>> = HashMap::new();
    let mut global_capacity: u64 = 0;

    for plot_dir in plot_dirs {
        let mut num_plots = 0;
        let mut local_capacity: u64 = 0;
        for file in read_dir(plot_dir).unwrap() {
            let file = &file.unwrap().path();

            if let Ok(p) = Plot::new(file, use_direct_io, dummy) {
                let drive_id = get_device_id(&file.to_str().unwrap().to_string());
                let plots = drive_id_to_plots.entry(drive_id).or_insert(Vec::new());

                local_capacity += p.meta.nonces as u64;
                plots.push(Mutex::new(p));
                num_plots += 1;
            }
        }

        info!(
            "path={}, files={}, size={:.4} TiB",
            plot_dir.to_str().unwrap(),
            num_plots,
            local_capacity as f64 / 4.0 / 1024.0 / 1024.0
        );

        global_capacity += local_capacity;
        if num_plots == 0 {
            warn!("no plots in {}", plot_dir.to_str().unwrap());
        }
    }

    // sort plots by filetime and get them into an arc
    let drive_id_to_plots: HashMap<String, Arc<Vec<Mutex<Plot>>>> = drive_id_to_plots
        .drain()
        .map(|(drive_id, mut plots)| {
            plots.sort_by_key(|p| {
                let m = p.lock().unwrap().fh.metadata().unwrap();
                -FileTime::from_last_modification_time(&m).unix_seconds()
            });
            (drive_id, Arc::new(plots))
        })
        .collect();

    info!(
        "plot files loaded: total drives={}, total capacity={:.4} TiB",
        drive_id_to_plots.len(),
        global_capacity as f64 / 4.0 / 1024.0 / 1024.0
    );

    (drive_id_to_plots, global_capacity * 64)
}

impl Miner {
    pub fn new(cfg: Cfg, executor: TaskExecutor, mac: String,miner_power: Arc<Mutex<MinerPower>>, scan: Arc::<Mutex<scanbool>>) -> Miner {
        let (drive_id_to_plots, total_size) =
            scan_plots(&cfg.plot_dirs, cfg.hdd_use_direct_io, cfg.benchmark_cpu());

        let cpu_threads = cfg.cpu_threads;
        let cpu_worker_task_count = cfg.cpu_worker_task_count;

        let cpu_buffer_count = cpu_worker_task_count
            + if cpu_worker_task_count > 0 {
                cpu_threads
            } else {
                0
            };

        let reader_thread_count = if cfg.hdd_reader_thread_count == 0 {
            drive_id_to_plots.len()
        } else {
            cfg.hdd_reader_thread_count
        };


        #[cfg(not(feature = "opencl"))]
        {
            info!(
                "reader-threads={} CPU-threads={}",
                reader_thread_count, cpu_threads
            );
            info!("CPU-buffer={}(+{})", cpu_worker_task_count, cpu_threads);
            {
                if cpu_threads * cpu_worker_task_count == 0 {
                    error!(
                    "CPU: no active workers. Check thread and task configuration. Shutting down..."
                );
                    process::exit(0);
                }
            }
        }

        #[cfg(not(feature = "opencl"))]
        let buffer_count = cpu_buffer_count;
        let buffer_size_cpu = cfg.cpu_nonces_per_cache * SCOOP_SIZE as usize;
        let (tx_empty_buffers, rx_empty_buffers) =
            crossbeam_channel::bounded(buffer_count as usize);
        let (tx_read_replies_cpu, rx_read_replies_cpu) =
            crossbeam_channel::bounded(cpu_buffer_count);


        for _ in 0..cpu_buffer_count {
            let cpu_buffer = CpuBuffer::new(buffer_size_cpu);
            tx_empty_buffers
                .send(Box::new(cpu_buffer) as Box<dyn Buffer + Send>)
                .unwrap();
        }


        let (tx_nonce_data, rx_nonce_data) = mpsc::channel(buffer_count);

        thread::spawn({
            create_cpu_worker_task(
                cfg.benchmark_io(),
                new_thread_pool(cpu_threads, cfg.cpu_thread_pinning),
                rx_read_replies_cpu.clone(),
                tx_empty_buffers.clone(),
                tx_nonce_data.clone(),
            )
        });

        #[cfg(not(feature = "opencl"))]
        Miner {
            reader_task_count: drive_id_to_plots.len(),
            reader: Reader::new(
                drive_id_to_plots,
                total_size,
                1,
                rx_empty_buffers,
                tx_empty_buffers,
                tx_read_replies_cpu,
                cfg.show_progress,
                cfg.show_drive_stats,
                cfg.cpu_thread_pinning,
                cfg.benchmark_cpu(),
                miner_power
            ),
            rx_nonce_data,
            target_deadline: cfg.target_deadline,
            account_id_to_target_deadline: cfg.account_id_to_target_deadline,
            request_handler: RequestHandler::new(
                cfg.url,
                cfg.timeout,
                (total_size * 4 / 1024 / 1024) as usize,
                executor.clone(),
                mac,
            ),
            state: Arc::new(Mutex::new(State::new())),
            // floor at 1s to protect servers
            get_mining_info_interval: max(1000, cfg.get_mining_info_interval),
            executor,
            wakeup_after: cfg.hdd_wakeup_after * 1000, // ms -> s
            scan: scan
        }
    }

    pub fn run(self) {
        let request_handler = self.request_handler.clone();
        let total_size = self.reader.total_size;

        // TODO: this doesn't need to be arc mutex if we manage to separate
        // reader from miner so that we can simply move it
        let reader = Arc::new(Mutex::new(self.reader));

        let state = self.state.clone();
        // there might be a way to solve this without two nested moves
        let get_mining_info_interval = self.get_mining_info_interval;
        let wakeup_after = self.wakeup_after;
        let sc = self.scan.clone();

        self.executor.clone().spawn(
            Interval::new_interval(Duration::from_millis(get_mining_info_interval))
                .for_each(move |_| {
                    let state = state.clone();
                    let reader = reader.clone();
                    let scan = sc.clone();
                    request_handler.get_mining_info().then(move |mining_info| {
                        match mining_info {
                            Ok(mining_info) => {
                                let mut state = state.lock().unwrap();
                                state.first = false;
                                if state.outage {
                                    error!("{: <80}", "outage resolved.");
                                    state.outage = false;
                                }

                                //获取最新算力
                                let newPower = reader.lock().unwrap().miner_power.lock().unwrap().get_plots_scan();
                                reader.lock().unwrap().drive_id_to_plots = newPower;

                                if mining_info.result.generationSignature != state.generation_signature ||
                                    scan.lock().unwrap().scan{

                                    state.update_mining_info(&mining_info);

                                    scan.lock().unwrap().update(false);

                                    reader.lock().unwrap().start_reading(
                                        mining_info.result.blockHeight,
                                        state.block,
                                        mining_info.result.baseTarget,
                                        state.scoop,
                                        &Arc::new(state.generation_signature_bytes),
                                    );
                                    drop(state);
                                    drop(scan);
                                } else if !state.scanning
                                    && wakeup_after != 0
                                    && state.sw.elapsed_ms() > wakeup_after
                                {
                                    info!("HDD, wakeup!");
                                    reader.lock().unwrap().wakeup();
                                    state.sw.restart();
                                }
                            }
                            _ => {
                                let mut state = state.lock().unwrap();
                                if state.first {
                                    error!(
                                        "{: <80}",
                                        "error getting mining info, please check server config"
                                    );
                                    state.first = false;
                                    state.outage = true;
                                } else {
                                    if !state.outage {
                                        error!(
                                            "{: <80}",
                                            "error getting mining info => connection outage..."
                                        );
                                    }
                                    state.outage = true;
                                }
                            }
                        }
                        future::ok(())
                    })
                })
                .map_err(|e| panic!("interval errored: err={:?}", e)),
        );

        let target_deadline = self.target_deadline;
        let account_id_to_target_deadline = self.account_id_to_target_deadline;
        let request_handler = self.request_handler.clone();
        let state = self.state.clone();
        let reader_task_count = self.reader_task_count;
        self.executor.clone().spawn(
            self.rx_nonce_data
                .for_each(move |nonce_data| {
                    let mut state = state.lock().unwrap();
                    let deadline = nonce_data.deadline / nonce_data.base_target;
                    if state.height == nonce_data.height {
                        let best_deadline = *state
                            .account_id_to_best_deadline
                            .get(&nonce_data.account_id)
                            .unwrap_or(&u64::MAX);
                        if best_deadline > deadline
                            && deadline
                                <  *(account_id_to_target_deadline
                                        .get(&nonce_data.account_id)
                                        .unwrap_or(&target_deadline))
                        {
                            state
                                .account_id_to_best_deadline
                                .insert(nonce_data.account_id, deadline);
                            request_handler.submit_nonce(
                                nonce_data.account_id,
                                nonce_data.nonce,
                                nonce_data.height,
                                nonce_data.block,
                                nonce_data.deadline,
                                deadline,
                                state.generation_signature_bytes,
                            );
                        }

                        if nonce_data.reader_task_processed {
                            state.processed_reader_tasks += 1;
                            if state.processed_reader_tasks == reader_task_count {
                                info!(
                                    "{: <80}",
                                    format!(
                                        "round finished: roundtime={}ms, speed={:.2}MiB/s",
                                        state.sw.elapsed_ms(),
                                        total_size as f64 * 1000.0
                                            / 1024.0
                                            / 1024.0
                                            / state.sw.elapsed_ms() as f64
                                    )
                                );
                                state.sw.restart();
                                state.scanning = false;
                            }
                        }
                    }
                    Ok(())
                })
                .map_err(|e| panic!("interval errored: err={:?}", e)),
        );
    }
}

#[cfg(test)]
mod tests{
    use std::thread;
    use std::time::Duration;
    use std::fs;
    use std::path::PathBuf;
    use crate::scan::config::load_cfg;
    use crate::scan::miner::scan_plots;
    use crate::scan::miner::Miner;


    #[test]
    fn test_thread_tokio(){
     let handle=  thread::spawn(|| {
           for i in 1..10{
               println!("hi number {}",i);
               thread::sleep(Duration::from_millis(1));
           }
       });

    for i in 1..5{
        println!("hi number {} from thread",i);
        thread::sleep(Duration::from_secs(1));
    }

        handle.join().unwrap();
    }

    #[test]
    fn test_file_size(){
        let cfg = load_cfg("config.yaml");
        let (drive_id_to_plots, total_size) = scan_plots(&cfg.plot_dirs, cfg.hdd_use_direct_io, cfg.benchmark_cpu());

        println!("len==={}",drive_id_to_plots.len());

        for  (k,v) in drive_id_to_plots {
            println!("drive===> {}",k);
            let vs = &v[..];
            println!("vs -->{:?}",vs);
        }

        println!("total_size----> {}",total_size);
    }
}