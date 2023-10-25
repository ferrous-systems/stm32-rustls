#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use alloc::vec;

use defmt::*;

use embassy_executor::Spawner;
use embassy_net::Ipv4Address;

use embassy_time::{Duration, Instant, Timer};

use stm32f429 as _;
use stm32f429::{
    get_time_from_ntp_server, init_heap, network_task_init, now_plus_elapsed_since_1900,
};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {

    let stack = network_task_init(spawner).await;

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    // send a hello message
    init_heap();
    let msg = vec![104, 101, 108, 108, 111];
    let now = Instant::now();
    let transmit_seconds = get_time_from_ntp_server(stack).await;

    loop {
        info!(
            "Elapsed time with NTP info{:?}",
            Debug2Format(&(now_plus_elapsed_since_1900(transmit_seconds, now.elapsed().as_secs())))
        );
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(1000)));
        let add = "192.168.50.67".parse::<Ipv4Address>().unwrap();
        info!("Listening on TCP:1234...");
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
