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
use embassy_time::{Duration, Instant, Timer};

use rustls::{ClientConfig, RootCertStore};
use rustls_pemfile::Item;
use stm32f429::democryptoprovider::DemoCryptoProvider;
use stm32f429::demotimeprovider::{self, DemoTimeProvider, SINCE_START};
use stm32f429::{self as _, board::Board};
use stm32f429::{init_call_to_ntp_server, init_heap, network_task_init};
use {defmt_rtt as _, panic_probe as _};
pub static CRYPTO_PROVIDER: &'static dyn rustls::crypto::CryptoProvider = &DemoCryptoProvider;
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
    init_call_to_ntp_server(stack).await;

    let now = Instant::now();
    SINCE_START.lock().await.replace(now);

    // Starting TCP

    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let mut certfile: &[_] = include_bytes!("/home/aissata/.local/share/mkcert/rootCA.pem");
    let mut certs = vec![];
    while let Ok(Some((item, rest))) = rustls_pemfile::read_one_from_slice(certfile) {
        certfile = rest;
        if let Item::X509Certificate(cert) = item {
            certs.push(cert);
        }
    }
    dbg!(certs.len());
    root_store.add_parsable_certificates(certs);

    let mut client_config = ClientConfig::builder_with_provider(CRYPTO_PROVIDER)
        .with_safe_defaults()
        .dangerous()
        .with_custom_certificate_verifier(stm32f429::certificate_verifier(root_store))
        .with_no_client_auth();
    client_config.time_provider = demotimeprovider::time_provider();

    dbg!("{}", Debug2Format(&client_config));
    loop {
        // let seconds = time_provider.get_current_time(now.elapsed().as_secs());
        // warn!("Elapsed time: {:?}", Debug2Format(&(seconds.unwrap())));
        // let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        // socket.set_timeout(Some(Duration::from_secs(1000)));
        // let add = "192.168.50.67".parse::<Ipv4Address>().unwrap();

        // if let Err(e) = socket.connect((add, 1234)).await {
        //     warn!("connect error: {:?}", e);
        //     Timer::after(Duration::from_secs(3)).await;
        //     continue;
        // }
        // info!("Connected to {:?}", socket.remote_endpoint());
        // loop {
        //     if let Err(e) = socket.write(&msg).await {
        //         warn!("write error: {:?}", e);
        //         break;
        //     }
        //     info!("txd: {}", core::str::from_utf8(&msg).unwrap());
        //     Timer::after(Duration::from_secs(10)).await;
        // }
    }
}
