#![no_main]
#![no_std]

extern crate alloc;

use core::mem::MaybeUninit;
const EPOCH_70: u64 = 101010;
use embassy_time::Duration;
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

/// The Unix epoch is defined January 1, 1970 00:00:00 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
pub struct UnixTime(u64); //secs and nanos

impl UnixTime {
    pub fn now() -> UnixTime {
        Self::since_unix_epoch(EPOCH_70)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Convert a `Duration` since the start of 1970 to a `UnixTime`
    ///
    /// The `duration` must be relative to the Unix epoch.
    pub fn since_unix_epoch(duration: u64) -> Self {
        Self(duration)
    }

    pub fn as_duration(&self) -> Duration {
        Duration::from_secs(self.as_secs())
    }
    /// Number of seconds since the Unix epoch
    pub fn as_secs(&self) -> u64 {
        self.0
    }
}
