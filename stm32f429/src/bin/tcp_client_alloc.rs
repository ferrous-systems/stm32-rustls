#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;

use defmt::*;

use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Stack};
use embassy_stm32::Config;

use embassy_stm32::time::mhz;
use embassy_time::{Duration, Instant, Timer};
use rustls::client::LlClientConnection;
use rustls::server::danger::DnsName;
use rustls::Error;
use rustls::{
    AppDataRecord, ClientConfig, EncodeError, InsufficientSizeError, LlState, LlStatus,
    RootCertStore,
};
use rustls_pemfile::Item;
use stm32f429::democryptoprovider::DemoCryptoProvider;
use stm32f429::demotimeprovider::{self, SINCE_START};
use stm32f429::{self as _, board::Board, http_request};
use stm32f429::{init_call_to_ntp_server, init_heap, network_task_init};
use {defmt_rtt as _, panic_probe as _};
pub static CRYPTO_PROVIDER: &'static dyn rustls::crypto::CryptoProvider = &DemoCryptoProvider;

const SERVER_NAME: &str = "www.rust-lang.org";
const PORT: u16 = 443;

#[derive(Debug)]
pub enum TCPOrTLSError {
    RustlsError(rustls::Error),
    RustlsEncodeError(rustls::EncodeError),
    RustlsEncryptError(rustls::EncryptError),
    TcpError(embassy_net::tcp::Error),
}

impl From<rustls::EncodeError> for TCPOrTLSError {
    fn from(err: rustls::EncodeError) -> Self {
        Self::RustlsEncodeError(err)
    }
}
impl From<rustls::EncryptError> for TCPOrTLSError {
    fn from(err: rustls::EncryptError) -> Self {
        Self::RustlsEncryptError(err)
    }
}
impl From<rustls::Error> for TCPOrTLSError {
    fn from(err: rustls::Error) -> TCPOrTLSError {
        Self::RustlsError(err)
    }
}

impl From<embassy_net::tcp::Error> for TCPOrTLSError {
    fn from(err: embassy_net::tcp::Error) -> Self {
        Self::TcpError(err)
    }
}
#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);
    let board = Board::new(p);

    let stack = network_task_init(spawner, board).await;

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
    // This seems correct
    //info!("{}", Debug2Format(&client_config));
    let _ = converse(client_config, stack).await;
    stm32f429::exit();
}

async fn converse(
    client_config: ClientConfig,
    stack: &Stack<
        embassy_stm32::eth::Ethernet<
            '_,
            embassy_stm32::peripherals::ETH,
            embassy_stm32::eth::generic_smi::GenericSMI,
        >,
    >,
) -> Result<(), TCPOrTLSError> {
    let mut conn = LlClientConnection::new(
        Arc::new(client_config),
        rustls::ServerName::DnsName(DnsName::try_from(SERVER_NAME.to_string()).unwrap()),
    )
    .unwrap();

    let sock_addr = (Ipv4Address::new(52, 85, 242, 98), PORT);
    dbg!(sock_addr);
    let mut incoming_tls = [0; 16 * 1024];
    let mut incoming_used = 0;

    let mut outgoing_tls = vec![];
    let mut outgoing_used = 0;
    let request = http_request(SERVER_NAME);

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];
    let mut open_connection = true;
    while open_connection {
        let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        let result = socket.accept(sock_addr).await;
        dbg!(result);

        let LlStatus { discard, state } =
            conn.process_tls_records(&mut incoming_tls[..incoming_used])?;
        match state {
            // logic similar to the one presented in the 'handling InsufficientSizeError' section is
            // used in these states
            LlState::MustEncodeTlsData(mut state) => {
                let written = match state.encode(&mut outgoing_tls[outgoing_used..]) {
                    Ok(written) => written,
                    Err(EncodeError::InsufficientSize(InsufficientSizeError { required_size })) => {
                        let new_len = outgoing_used + required_size;
                        outgoing_tls.resize(new_len, 0);
                        warn!("resized `outgoing_tls` buffer to {}B", new_len);

                        // don't forget to encrypt the handshake record after resizing!
                        state
                            .encode(&mut outgoing_tls[outgoing_used..])
                            .expect("should not fail this time")
                    }
                    Err(err) => return Err(TCPOrTLSError::RustlsEncodeError(err)),
                };
                outgoing_used += written;
            }
            LlState::MustTransmitTlsData(state) => {
                socket.write(&outgoing_tls[..outgoing_used]).await?;

                outgoing_used = 0;

                state.done();
            }

            LlState::NeedsMoreTlsData { .. } => {
                // NOTE real code needs to handle the scenario where `incoming_tls` is not big enough
                let read = socket.read(&mut incoming_tls[incoming_used..]).await?;
                incoming_used += read;
            }

            LlState::AppDataAvailable(mut records) => {
                while let Some(result) = records.next_record() {
                    let AppDataRecord { payload, .. } = result?;

                    info!(
                        "response:\n{:?}",
                        Debug2Format(&core::str::from_utf8(payload))
                    );
                }
            }

            LlState::TrafficTransit(mut traffic_transit) => {
                // post-handshake logic
                let request = request.as_bytes();
                let len = traffic_transit.encrypt(request, outgoing_tls.as_mut_slice())?;
                socket.write(&outgoing_tls[..len]).await?;

                let read = socket.read(&mut incoming_tls[incoming_used..]).await?;
                incoming_used += read;
            }

            LlState::ConnectionClosed => open_connection = false,
            _ => (),
        }

        // discard TLS records
        if discard != 0 {
            incoming_tls.copy_within(discard..incoming_used, 0);
            incoming_used -= discard;
        }
    }

    Ok(())
}
