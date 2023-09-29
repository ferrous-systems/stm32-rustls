#![no_main]
#![no_std]

use core::time::Duration;

use cortex_m_semihosting::debug;

use defmt_rtt as _; // global logger

use stm32f4xx_hal::{
    self as _,
    timer::fugit::{self, Duration as FugitDuration},
}; // memory layout

use panic_probe as _;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}
pub trait DurationExt {
    fn to_core_duration(&self) -> Duration;
}
impl DurationExt for FugitDuration<u64, 1, 10_000> {
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
