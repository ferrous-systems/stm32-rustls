#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]


use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::Config;
use {defmt_rtt as _, panic_probe as _};



#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    let config = Config::default();
    let _p = embassy_stm32::init(config);
    info!("going to panic");
    panic!();
}