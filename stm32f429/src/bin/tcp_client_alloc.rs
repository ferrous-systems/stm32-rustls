#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use alloc::vec;

use embassy_executor::Spawner;
use embassy_stm32::Config;

use embassy_stm32::time::mhz;
use embassy_time::{Duration, Instant, Timer};
use rustls::client::{ClientConfig, InvalidDnsNameError, LlClientConnection};
use stm32f429::demotimeprovider::DemoTimeProvider;
use stm32f429::{self as _, board::Board};
use stm32f429::{init_call_to_ntp_server, init_heap, network_task_init};
use {defmt_rtt as _, panic_probe as _};

const SERVER_NAME: &str = "localhost";
const PORT: u16 = 1443;
#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);
    let board = Board::new(p);

    let stack = network_task_init(spawner, board).await;

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    init_heap();
    let msg = vec![104, 101, 108, 108, 111];

    init_call_to_ntp_server(stack).await;

    let time_provider = DemoTimeProvider::new();

    let mut client_config = ClientConfig::builder_with_provider(demo::CRYPTO_PROVIDER)
        .with_safe_defaults()
        .dangerous()
        .with_custom_certificate_verifier(stm32f429::certificate_verifier(root_store))
        .with_no_client_auth();

    client_config.time_provider = time_provider;
    let now = Instant::now();
    let mut conn = LlClientConnection::new(
        Arc::new(config),
        rustls::ServerName::DnsName(DnsName::try_from(SERVER_NAME.to_string())?),
    )?;
    let mut outgoing_tls = vec![];
    let mut outgoing_used = 0;

    let mut open_connection = true;

    while open_connection {
        let Status { discard, state } =
            conn.process_tls_records(&mut incoming_tls[..incoming_used]);
    }
    // loop {
    //     let seconds = time_provider.get_current_time(now.elapsed().as_secs());
    //     warn!("Elapsed time: {:?}", Debug2Format(&(seconds.unwrap())));
    //     let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    //     socket.set_timeout(Some(Duration::from_secs(1000)));
    //     let add = "192.168.50.67".parse::<Ipv4Address>().unwrap();

    //     if let Err(e) = socket.connect((add, 1234)).await {
    //         warn!("connect error: {:?}", e);
    //         Timer::after(Duration::from_secs(3)).await;
    //         continue;
    //     }
    //     info!("Connected to {:?}", socket.remote_endpoint());
    //     loop {
    //         if let Err(e) = socket.write(&msg).await {
    //             warn!("write error: {:?}", e);
    //             break;
    //         }
    //         info!("txd: {}", core::str::from_utf8(&msg).unwrap());
    //         Timer::after(Duration::from_secs(10)).await;
    //     }
    // }
}
