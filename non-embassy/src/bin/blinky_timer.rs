#![no_main]
#![no_std]

use f429::{self as _, DurationExt, TEN_KHZ};
use rtic_monotonic::Monotonic;
use stm32f4xx_hal::{
    pac,
    prelude::{_fugit_RateExtU32, _stm32f4xx_hal_gpio_GpioExt},
    rcc::RccExt,
    timer::{ExtU32, MonoTimer64Ext, TimerExt},
}; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello Blinky");
    let p = pac::Peripherals::take().unwrap();
    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(96u32.MHz()).freeze();

    let mut timer: stm32f4xx_hal::timer::MonoTimer64<pac::TIM2, TEN_KHZ> =
        p.TIM2.monotonic64::<TEN_KHZ>(&clocks);
    let gpiob = p.GPIOB.split();
    let mut led = gpiob.pb7.into_push_pull_output();

    let mut delay = p.TIM3.delay::<TEN_KHZ>(&clocks);

    let now = timer.now();

    loop {
        let dur = timer.now() - now;
        defmt::info!("Fugit Duration (ticks): {:?}", defmt::Debug2Format(&dur));
        let core_dur = dur.to_core_duration();
        defmt::info!(
            "Converted to core duration {:?}",
            defmt::Debug2Format(&core_dur)
        );

        defmt::info!("&timer.now: {:?}", defmt::Debug2Format(&timer.now()));

        delay.delay(1_u32.secs());
        led.toggle();
    }
}
