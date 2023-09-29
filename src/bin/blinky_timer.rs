#![no_main]
#![no_std]

use core::time::Duration;
use f429::{self as _, DurationExt};
use rtic_monotonic::Monotonic;
use stm32f4xx_hal::{
    pac,
    prelude::{_fugit_RateExtU32, _stm32f4xx_hal_gpio_GpioExt},
    rcc::RccExt,
    timer::{fugit::ExtU64, ExtU32, MonoTimer64Ext, TimerExt},
};
// global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello Blinky");
    let p = pac::Peripherals::take().unwrap();
    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48u32.MHz()).freeze();

    let mut timer: stm32f4xx_hal::timer::MonoTimer64<pac::TIM2, 10000> =
        p.TIM2.monotonic64::<10_000>(&clocks);

    let gpiob = p.GPIOB.split();

    let mut led = gpiob.pb7.into_push_pull_output();
    let now = timer.now();

    loop {
        let dur = timer.now() - now;
        defmt::info!("&timer.now: {:?}", defmt::Debug2Format(&dur));
        let core_dur = dur.to_core_duration();
        defmt::info!("&timer.now: {:?}", defmt::Debug2Format(&core_dur));

        defmt::info!("&timer.now: {:?}", defmt::Debug2Format(&timer.now()));
    }
}
