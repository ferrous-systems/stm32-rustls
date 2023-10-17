#![no_main]
#![no_std]

extern crate alloc;
use core::mem::MaybeUninit;
use embassy_time::{Duration, Instant};
use rustls_pki_types::UnixTime;
const EPOCH_70: u64 = 100;
use embedded_alloc::Heap;
use spin;

const HEAP_SIZE: usize = 1024;
#[global_allocator]
static HEAP: Heap = Heap::empty();
static START: spin::Once = spin::Once::new();

pub fn init_heap() {
    START.call_once(|| {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    });
}

pub fn now_plus_elapsed_since_1970(unix: u64, monotonic_now: u64) -> u64 {
    monotonic_now + unix
}
