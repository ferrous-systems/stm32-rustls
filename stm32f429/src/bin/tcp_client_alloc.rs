#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use core::f32::consts::E;
use core::str::FromStr;

use alloc::borrow::Cow;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};

use defmt::{dbg, info, warn, Debug2Format, Format};
use embassy_executor::Spawner;
use embassy_net::{dns, Ipv4Address};
use embassy_stm32::time::mhz;
use embassy_stm32::Config;
use embassy_time::{Duration, Instant, Timer};
use futures::TryFutureExt;
use heapless::String;
use rustls::client::{ClientConnectionData, InvalidDnsNameError, LlClientConnection};
use rustls::server::danger::DnsName;
use rustls::version::{TLS12, TLS13};
use rustls::{AppDataRecord, ClientConfig, InsufficientSizeError, LlState, RootCertStore};
use rustls::{EncodeError, LlStatus};
use rustls_pemfile::Item;
use stm32_rustls::democryptoprovider::DemoCryptoProvider;
use stm32_rustls::demotimeprovider::SINCE_START;
use stm32_rustls::{self as _, board::Board};
use stm32_rustls::{demotimeprovider, init_call_to_ntp_server, init_heap, network_task_init};
use {defmt_rtt as _, panic_probe as _};

// url scheme = https://
const SERVER_NAME: &str = "www.rust-lang.org";
const PORT: u16 = 443;
pub static CRYPTO_PROVIDER: &'static dyn rustls::crypto::CryptoProvider = &DemoCryptoProvider;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);
    let board = Board::new(p);
    dbg!("before stack");
    let stack = network_task_init(spawner, board).await;
    dbg!("after stack");
    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];

    init_heap();

    init_call_to_ntp_server(stack).await;

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
        .with_custom_certificate_verifier(stm32_rustls::certificate_verifier(root_store))
        .with_no_client_auth();

    let now: Instant = Instant::now();
    SINCE_START.lock().await.replace(now);

    client_config.time_provider = demotimeprovider::time_provider();

    let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(1)));

    // got this from dig +short rust-lang.org
    let remote_endpoint = (Ipv4Address::new(52, 85, 242, 98), PORT);

    let connection_result = socket.connect(remote_endpoint).await;

    match connection_result {
        Ok(_) => info!("connection worked"),
        Err(e) => info!("connection error {}", &e),
    }

    //TLS starts here
    let mut conn = LlClientConnection::new(
        Arc::new(client_config),
        rustls::ServerName::DnsName(DnsName::try_from(SERVER_NAME.to_string()).unwrap()),
    )
    .unwrap();
    let request = http_request(SERVER_NAME);
    dbg!("Going to log request");
    dbg!(request);
    let mut incoming_tls: [u8; 16384] = [0; 16 * 1024];
    let mut incoming_used = 0;

    let mut outgoing_tls: Vec<u8> = vec![];
    let mut outgoing_used = 0;
    let mut open_connection = true;
    let mut i = 150;
    loop {}
    // 'externe: loop {
    //     while open_connection {
    //         let LlStatus { discard, state } = conn
    //             .process_tls_records(&mut incoming_tls[..incoming_used])
    //             .unwrap();

    //         info!("{}", Debug2Format(&state));
    //         match state {
    //             LlState::MustEncodeTlsData(mut state) => {
    //                 let written = match state.encode(&mut outgoing_tls[outgoing_used..]) {
    //                     Ok(written) => {
    //                         info!("WRITTEN {}", Debug2Format(&written));
    //                         written
    //                     }
    //                     Err(e) => {
    //                         info!("ERROR {}", Debug2Format(&e));
    //                         0
    //                     }
    //                 };
    //                 i -= 1;
    //             }
    //             LlState::TrafficTransit(mut traffic_transit) => {
    //                 // post-handshake logic
    //                 let request = request.as_bytes();
    //                 let len = traffic_transit
    //                     .encrypt(request, outgoing_tls.as_mut_slice())
    //                     .unwrap();
    //                 info!("Going to write to socket!");
    //                 // let _ = socket.write(&outgoing_tls[..len]).await;

    //                 // let read = socket
    //                 //     .read(&mut incoming_tls[incoming_used..])
    //                 //     .await
    //                 //     .unwrap();
    //                 // incoming_used += read;
    //             }
    //             LlState::NeedsMoreTlsData { num_bytes } => {
    //                 info!("I NEED MORE TLS DATA");
    //                 // let read = socket.read(&mut incoming_tls[incoming_used..]).await;
    //                 // info!("ERROR {}", Debug2Format(&read));

    //                 // match read {
    //                 //     Ok(read) => incoming_used += read,
    //                 //     Err(e) => info!("Error on NeedsMoreTlsData"),
    //                 // }
    //                 i -= 1;
    //             }
    //             _ => info!("{}", Debug2Format(&state)),
    //         }
    //         if i <= 0 {
    //             break 'externe;
    //         }
    //     }
    // }
    // stm32_rustls::exit();
}

fn http_request(server_name: &str) -> String<1024> {
    const HTTP_SEPARATOR: &str = "\r\n";

    let lines = [
        Cow::Borrowed("GET / HTTP/1.1"),
        format!("Host: {server_name}").into(),
        "Connection: close".into(),
        "Accept-Encoding: identity".into(),
        "".into(), // body
    ];

    let mut req = String::new();
    for line in lines {
        let _ = req.push_str(&line);
        let _ = req.push_str(HTTP_SEPARATOR);
    }

    req
}
