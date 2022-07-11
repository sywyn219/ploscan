use crate::plo::cpu_hasher::{hash_cpu, CpuTask, SafePointer};
use crate::plo::buffer::PageAlignedByteBuffer;
#[cfg(feature = "opencl")]
use crate::gpu_hasher::{create_gpu_hasher_thread, GpuTask};
#[cfg(feature = "opencl")]
use crate::ocl::gpu_init;
use crate::plo::plotter::{PlotterTask, NONCE_SIZE};
#[cfg(feature = "opencl")]
use crossbeam_channel::unbounded;
use crossbeam_channel::{Receiver, Sender};
use std::cmp::min;
use std::sync::mpsc::channel;
use std::sync::Arc;
#[cfg(feature = "opencl")]
use std::thread;

const CPU_TASK_SIZE: u64 = 64;

pub fn create_scheduler_thread(
    task: Arc<PlotterTask>,
    thread_pool: rayon::ThreadPool,
    mut nonces_hashed: u64,
    mut pb: Option<pbr::ProgressBar<pbr::Pipe>>,
    rx_empty_buffers: Receiver<PageAlignedByteBuffer>,
    tx_buffers_to_writer: Sender<PageAlignedByteBuffer>
) -> impl FnOnce() {
    move || {
        // synchronisation chanel for all hashing devices (CPU+GPU)
        // message protocol:    (hash_device_id: u8, message: u8, nonces processed: u64)
        // hash_device_id:      0=CPU, 1=GPU0, 2=GPU1...
        // message:             0 = data ready to write
        //                      1 = device ready to compute next hashing batch
        // nonces_processed:    nonces hashed / nonces writen to host buffer
        let (tx, rx) = channel();

        for buffer in rx_empty_buffers {
            let mut_bs = &buffer.get_buffer();
            let mut bs = mut_bs.lock().unwrap();
            let buffer_size = (*bs).len() as u64;
            let nonces_to_hash = min(buffer_size / NONCE_SIZE, task.nonces - nonces_hashed);

            let mut requested = 0u64;
            let mut processed = 0u64;

            for _ in 0..task.cpu_threads {
                let task_size = min(CPU_TASK_SIZE, nonces_to_hash - requested);
                if task_size > 0 {
                    let task = hash_cpu(
                        tx.clone(),
                        CpuTask {
                            cache: SafePointer {
                                ptr: bs.as_mut_ptr(),
                            },
                            cache_size: (buffer_size / NONCE_SIZE) as usize,
                            chunk_offset: requested as usize,
                            numeric_id: task.numeric_id,
                            local_startnonce: task.start_nonce + nonces_hashed + requested,
                            local_nonces: task_size,
                        }
                    );
                    thread_pool.spawn(task);
                }
                requested += task_size;
            }

            // control loop
            let rx = &rx;
            for msg in rx {
                match msg.1 {
                    // process a request for work: provide a task or signal completion
                    1 => {
                        let task_size = {
                                // schedule next cpu task
                                let task_size = min(CPU_TASK_SIZE, nonces_to_hash - requested);
                                if task_size > 0 {
                                    let task = hash_cpu(
                                        tx.clone(),
                                        CpuTask {
                                            cache: SafePointer {
                                                ptr: bs.as_mut_ptr(),
                                            },
                                            cache_size: (buffer_size / NONCE_SIZE) as usize,
                                            chunk_offset: requested as usize,
                                            numeric_id: task.numeric_id,
                                            local_startnonce: task.start_nonce
                                                + nonces_hashed
                                                + requested,
                                            local_nonces: task_size,
                                        }
                                    );
                                    thread_pool.spawn(task);
                                }
                            task_size
                        };

                        requested += task_size;
                        //println!("Debug: Device: {} asked for work. {} nonces assigned. Total requested: {}\n\n\n",msg.0,task_size,requested);
                    }
                    // process work completed message
                    0 => {
                        processed += msg.2;
                        match &mut pb {
                            Some(pb) => {
                                pb.add(msg.2 * NONCE_SIZE);
                            }
                            None => (),
                        }
                    }
                    _ => {}
                }
                if processed == nonces_to_hash {
                    break;
                }
            }

            nonces_hashed += nonces_to_hash;

            // queue buffer for writing
            tx_buffers_to_writer.send(buffer).unwrap();

            // thread end
            if task.nonces == nonces_hashed {
                match &mut pb {
                    Some(pb) => {
                        pb.finish()
                    }
                    None => (),
                }
                break;
            };
        }
    }
}
