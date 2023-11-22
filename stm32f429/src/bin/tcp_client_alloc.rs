#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use alloc::borrow::Cow;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};

use defmt::{dbg, info, warn, Debug2Format, Format};
use embassy_executor::Spawner;
use embassy_stm32::time::mhz;
use embassy_stm32::Config;
use embassy_time::{Duration, Instant, Timer};
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

const SERVER_NAME: &str = "www.rust-lang.org";

pub static CRYPTO_PROVIDER: &'static dyn rustls::crypto::CryptoProvider = &DemoCryptoProvider;

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);
    let board = Board::new(p);

    let stack = network_task_init(spawner, board).await;

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
    socket.set_timeout(Some(Duration::from_secs(1000)));

    let mut conn = LlClientConnection::new(
        Arc::new(client_config),
        rustls::ServerName::DnsName(DnsName::try_from(SERVER_NAME.to_string()).unwrap()),
    )
    .unwrap();
    let request = http_request(SERVER_NAME);

    let mut incoming_tls = [0; 16 * 1024];
    let mut incoming_used = 0;

    let mut outgoing_tls: Vec<u8> = vec![];
    let mut outgoing_used = 0;
    let mut open_connection = true;

    loop {
        while open_connection {
            let LlStatus { discard, state } = conn
                .process_tls_records(&mut incoming_tls[..incoming_used])
                .unwrap();
            //     match state {
            //         // logic similar to the one presented in the 'handling InsufficientSizeError' section is
            //         // used in these states
            //         LlState::MustEncodeTlsData(mut state) => {
            //             let written = match state.encode(&mut outgoing_tls[outgoing_used..]) {
            //                 Ok(written) => written,
            //                 Err(EncodeError::InsufficientSize(InsufficientSizeError {
            //                     required_size,
            //                 })) => {
            //                     let new_len = outgoing_used + required_size;
            //                     outgoing_tls.resize(new_len, 0);
            //                     warn!("resized `outgoing_tls` buffer to {}B", new_len);

            //                     // don't forget to encrypt the handshake record after resizing!
            //                     state
            //                         .encode(&mut outgoing_tls[outgoing_used..])
            //                         .expect("should not fail this time")
            //                 }
            //                 Err(err) => 0,
            //             };
            //             outgoing_used += written;
            //         }
            //         LlState::MustTransmitTlsData(state) => {
            //             socket.write(&outgoing_tls[..outgoing_used]).await;

            //             outgoing_used = 0;

            //             state.done();
            //         }

            //         LlState::NeedsMoreTlsData { .. } => {
            //             // NOTE real code needs to handle the scenario where `incoming_tls` is not big enough
            //             let read = socket
            //                 .read(&mut incoming_tls[incoming_used..])
            //                 .await
            //                 .unwrap();
            //             incoming_used += read;
            //         }

            //         LlState::AppDataAvailable(mut records) => {
            //             while let Some(result) = records.next_record() {
            //                 let AppDataRecord { payload, .. } = result.unwrap();
            //                 // needs to be converted from utf8I s
            //                 info!("response:\n{:?}", Debug2Format(payload));
            //             }
            //         }

            //         LlState::TrafficTransit(mut traffic_transit) => {
            //             // post-handshake logic
            //             let request = request.as_bytes();
            //             let len = traffic_transit
            //                 .encrypt(request, outgoing_tls.as_mut_slice())
            //                 .unwrap();
            //             socket.write(&outgoing_tls[..len]).await.unwrap();

            //             let read = socket
            //                 .read(&mut incoming_tls[incoming_used..])
            //                 .await
            //                 .unwrap();
            //             incoming_used += read;
            //         }

            //         LlState::ConnectionClosed => open_connection = false,
            //         _ => {}
            //     }

            //     // discard TLS records
            //     if discard != 0 {
            //         assert!(discard <= incoming_used);

            //         incoming_tls.copy_within(discard..incoming_used, 0);
            //         incoming_used -= discard;
            //     }
            // }
            // ()
            info!("{}", Debug2Format(&state))
        }
    }
}

fn http_request(server_name: &str) -> String<6400> {
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
