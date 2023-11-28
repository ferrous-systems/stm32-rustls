use core::ops::Range;

use crate::{ELAPSED_SINCE_1900, TIME_BETWEEN_1900_1970};
use defmt::dbg;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpAddress, IpEndpoint, Ipv4Address, Stack};
use embassy_stm32::eth::{generic_smi::GenericSMI, Ethernet};
use embassy_stm32::peripherals::ETH;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embassy_time::{Duration, Instant};
use rustls::time_provider::{GetCurrentTime, TimeProvider};
use rustls_pki_types::UnixTime;

pub static SINCE_START: Mutex<ThreadModeRawMutex, Option<Instant>> = Mutex::new(None);

pub fn time_provider() -> TimeProvider {
    TimeProvider::new(DemoTimeProvider)
}
struct DemoTimeProvider;

impl GetCurrentTime for DemoTimeProvider {
    fn get_current_time(&self) -> Result<UnixTime, ()> {
        let elapsed_since_1900 = embassy_futures::block_on(async {
            let provisory = ELAPSED_SINCE_1900.lock().await;
            *provisory.as_ref().unwrap()
        });
        let now = embassy_futures::block_on(async {
            let provisory = SINCE_START.lock().await;
            provisory.as_ref().unwrap().as_secs()
        });
        let remove_before_1970 = elapsed_since_1900 - TIME_BETWEEN_1900_1970;
        // this 100 needs to be the NOW
        let total = Duration::from_secs(remove_before_1970 + now).into();
        Ok(UnixTime::since_unix_epoch(total))
    }
}

pub async fn get_time_from_ntp_server(
    stack: &'static Stack<Ethernet<'static, ETH, GenericSMI>>,
) -> u64 {
    const NTP_PACKET_SIZE: usize = 48;
    const TX_SECONDS: Range<usize> = 40..44;

    let ntp_server = IpEndpoint {
        // Picked up this server address from sketch
        addr: IpAddress::Ipv4(Ipv4Address::new(162, 159, 200, 1)),
        port: 123,
    };

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 6400];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 6400];
    let mut buf = [0u8; NTP_PACKET_SIZE];

    let mut sock = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    sock.bind(45698).unwrap();

    // this magic number means
    // - use NTPv3
    // - we are a client
    buf[0] = 0x1b;
    sock.send_to(&buf, ntp_server).await.unwrap();

    let mut response = buf;

    let (_read, _ntc_peer) = sock.recv_from(&mut response).await.unwrap();

    //dbg!(read);
    //dbg!(ntc_peer);
    let transmit_seconds = u32::from_be_bytes(response[TX_SECONDS].try_into().unwrap());

    transmit_seconds.into()
}
