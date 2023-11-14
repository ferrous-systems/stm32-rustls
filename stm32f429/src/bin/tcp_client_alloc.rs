#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use alloc::vec;

use defmt::*;

use embassy_executor::Spawner;
use embassy_net::Ipv4Address;
use embassy_stm32::Config;

use embassy_stm32::time::mhz;
use embassy_time::{Duration, Timer};

use stm32f429::demotimeprovider::DemoTimeProvider;
use stm32f429::{self as _, board::Board};
use stm32f429::{init_heap, network_task_init};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);
    let board = Board::new(p);

    let stack = network_task_init(spawner, board).await;

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    // send a hello message
    init_heap();
    let msg = vec![104, 101, 108, 108, 111];

    let time_provider = DemoTimeProvider::new();

    // make get_current_time instead that wraps
    let seconds = time_provider.get_current_time();

    loop {
        warn!(
            "Elapsed time with NTP info{:?}",
            Debug2Format(&(seconds.unwrap()))
        );
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(1000)));
        let add = "192.168.50.67".parse::<Ipv4Address>().unwrap();

        if let Err(e) = socket.connect((add, 1234)).await {
            warn!("connect error: {:?}", e);
            Timer::after(Duration::from_secs(3)).await;
            continue;
        }
        info!("Connected to {:?}", socket.remote_endpoint());
        loop {
            if let Err(e) = socket.write(&msg).await {
                warn!("write error: {:?}", e);
                break;
            }
            info!("txd: {}", core::str::from_utf8(&msg).unwrap());
            Timer::after(Duration::from_secs(10)).await;
        }
    }
}
