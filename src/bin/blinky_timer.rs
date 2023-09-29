#![no_main]
#![no_std]

use core::time::Duration;

use f429 as _;
// Timer should reexport monotonic?
// and monotonic exports fugit?

use rtic_monotonic::Monotonic;
use stm32f4xx_hal::{
    pac,
    prelude::{_fugit_RateExtU32, _stm32f4xx_hal_gpio_GpioExt},
    rcc::RccExt,
    timer::{fugit::ExtU64, ExtU32, MonoTimer64Ext, TimerExt},
}; // global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello Blinky");
    // are those different https://github.com/stm32-rs/stm32f4xx-hal/blob/a7406b69e46254a529fc7a4360ba1c9efd27ca50/examples/dwt-blinky.rs#L17
    let p = pac::Peripherals::take().unwrap();
    //let cp = cortex_m::peripheral::Peripherals::take().unwrap();
    //https://github.com/stm32-rs/stm32f4xx-hal/blob/a7406b69e46254a529fc7a4360ba1c9efd27ca50/examples/dwt-blinky.rs#L29
    let rcc = p.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48u32.MHz()).freeze();
    //let dwt = cp.DWT.constrain(cp.DCB, &clocks);
    //let dwt = cp.DWT;
    //let dcb = cp.DCB;
    //let timer = MonoTimer::new(dwt, dcb, clocks);
    //p.TIM1.monotonic_us(&clocks);
    //let mut mono = p.TIM3.monotonic64::<64_000_000>(&clocks);
    // calling timer every 60ms
    // reduce the timer to
    let mut mono = p.TIM3.monotonic64::<10_000>(&clocks);
    //let mono = MonoTimer::new(dwt, dcb, &clocks);
    //let mono = MonoTimer::new(dwt, dcb, clocks);
    let mut delay = p.TIM2.delay_ms(&clocks);

    let gpiob = p.GPIOB.split();

    let mut led = gpiob.pb7.into_push_pull_output();
    let mut now = mono.now();

    loop {
        // old now minus new now
        // if (mono.now() - now) < 1_u64.secs::<1, 64_000_000>() {
        //     led.set_high();
        // } else {
        //     led.set_low();
        //     delay.delay(1_u32.secs());
        //     now = mono.now();
        // };
        let now1 = mono.now();
        //::info!("time: {:?}", defmt::Debug2Format(&mono.now()));
        // usually timers have flags
        // duration type that is not core::time
        // ticks is not a coretime duration
        //core time duration has no type parameters

        //delay.delay(100_u32.millis());
        // for _ in 0..4 {
        //     cortex_m::asm::delay(12_000_000);
        //     mono.on_interrupt();
        // }
        // test with stopwatch!
        // concrete associated type of trait implementation
        // from fugit to coretime duration --> conversion inside an EXTtrait
        // do some assertions
        // do that in lib.rs
        // looking at the monotonic trait, has an associated type Duration
        // hal documentation has the exact type
        // as the impls

        // check when to call that exactly
        // if the delay  is longer than the
        let mil_sec = (mono.now() - now1).to_millis();
        // defmt::info!("milsecs: {}", mil_sec);
        let mil_sec2 = mil_sec / 1000;
        // defmt::info!("milsecs 2: {}", mil_sec2);
        let nan = ((mil_sec % 1000) * 1_000_000) as u32;
        // defmt::info!("nan: {}", nan);
        let dur = Duration::new(mil_sec2, nan);
        //defmt::info!("{:?}", defmt::Debug2Format(&(dur)));
        // defmt::info!(
        //     "subsec millis {:?}",
        //     defmt::Debug2Format(&(dur.subsec_millis()))
        // );

        // this is housekeeping
        // ideally call it from the interrupt of the timer itself

        led.set_high();
        delay.delay(1_u32.secs());
        defmt::info!("{:?}", defmt::Debug2Format(&mono.now()));
        mono.on_interrupt();
        led.set_low();
        delay.delay(1_u32.secs());
    }
}

// should I implement a wrapping sub
// is it OK to use rtic features like this one rtic-monotonic = "1.0.0"
// what is the diff here
// let mut mono = p.TIM3.monotonic64::<64_000_000>(&clocks);
// let mut mono = p.TIM3.monotonic64_us(&clocks);
// let mono = MonoTimer::new(dwt, dcb, &clocks);
// cortex_m --> available to all microcontrollers, for example there is no ethernet!
// pac --> specific to ur microcontroller
// the fields are different
// whichever is fine in this case
// Ext == extension trait, u can add as many methods,
// but if defined in a 3rd party traits, you can add methods
// to a third party but you need to import those traits into scope
// inherent methods (impl) are possible in the crate where the type is defined
