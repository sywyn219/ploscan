// use crate::poc_hashing::noncegen_rust;
// use ploscan::plo::poc_hashing::noncegen_rust;
use crate::plo::poc_hashing::noncegen_rust;

// use libc::{c_void, size_t, uint64_t};
use std::slice::from_raw_parts_mut;
use std::sync::mpsc::Sender;

const NUM_SCOOPS: usize = 4096;
const SCOOP_SIZE: usize = 64;
const NONCE_SIZE: usize = NUM_SCOOPS * SCOOP_SIZE;


pub struct SafePointer {
    pub ptr: *mut u8,
}
unsafe impl Send for SafePointer {}
unsafe impl Sync for SafePointer {}

pub struct CpuTask {
    pub cache: SafePointer,
    pub cache_size: usize,
    pub chunk_offset: usize,
    pub numeric_id: u64,
    pub local_startnonce: u64,
    pub local_nonces: u64,
}

pub fn hash_cpu(
    tx: Sender<(u8, u8, u64)>,
    hasher_task: CpuTask
) -> impl FnOnce() {
    move || {
        unsafe {
            let data = from_raw_parts_mut(
                hasher_task.cache.ptr,
                hasher_task.cache_size * NONCE_SIZE,
            );
            noncegen_rust(
                data,
                hasher_task.chunk_offset,
                hasher_task.numeric_id,
                hasher_task.local_startnonce,
                hasher_task.local_nonces,
            );
        }

        // report hashing done
        tx.send((0u8, 1u8, 0))
            .expect("CPU task can't communicate with scheduler thread.");
        // report data in hostmem
        tx.send((0u8, 0u8, hasher_task.local_nonces))
            .expect("CPU task can't communicate with scheduler thread.");
    }
}

#[cfg(test)]
mod test {
}
