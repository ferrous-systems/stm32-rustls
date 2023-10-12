#![no_main]
#![no_std]

use core::time::Duration;

use core::mem::MaybeUninit;
use cortex_m_semihosting::debug;

use defmt_rtt as _; // global logger

use embedded_alloc::Heap;
use stm32f4xx_hal::{self as _, timer::fugit::Duration as FugitDuration};
pub const DENOM: u32 = 1;
pub const TEN_KHZ: u32 = 10_000;
use panic_probe as _;
use spin; // memory layout
          // same panicking *behavior* as `panic-probe` but doesn't print a panic message
          // this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
pub trait DurationExt {
    fn to_core_duration(&self) -> Duration;
}
impl DurationExt for FugitDuration<u64, DENOM, TEN_KHZ> {
    fn to_core_duration(&self) -> Duration {
        let total_in_millis = self.to_millis();
        let seconds = total_in_millis / 1000;
        let nanos = ((total_in_millis % 1_000) * 1_000_000) as u32;
        Duration::new(seconds, nanos)
    }
}
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cortex_m_rt::exception]
unsafe fn HardFault(_frame: &cortex_m_rt::ExceptionFrame) -> ! {
    loop {
        debug::exit(debug::EXIT_FAILURE);
    }
}

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
