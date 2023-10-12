#![no_main]
#![no_std]

use f429 as _;
use stm32f4xx_hal::{pac, prelude::*}; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello Blinky");
    let p = pac::Peripherals::take().unwrap();

    let gpiob = p.GPIOB.split();

    let mut led = gpiob.pb7.into_push_pull_output();
    loop {
        for _ in 0..1_000_000 {
            led.set_high();
        }
        for _ in 0..1_000_000 {
            led.set_low();
        }
    }
}
