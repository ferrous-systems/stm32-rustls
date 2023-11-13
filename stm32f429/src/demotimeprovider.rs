use core::ops::Range;

use defmt::dbg;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{IpAddress, IpEndpoint, Ipv4Address, Stack};
use embassy_stm32::eth::{generic_smi::GenericSMI, Ethernet};
use embassy_stm32::peripherals::ETH;
use embassy_time::{Duration, Instant};
use rustls_pki_types::UnixTime;

pub struct DemoTimeProvider;

impl DemoTimeProvider {
    pub fn new() -> Self {
        Self
    }
    pub async fn now_plus_elapsed_since_1900(
        &self,
        stack: &'static Stack<Ethernet<'static, ETH, GenericSMI>>,
    ) -> Result<UnixTime, ()> {
        let now = Instant::now().elapsed().as_secs();
        let elapsed_since_1900 = self.get_time_from_ntp_server(stack).await;
        let total = Duration::from_secs(now + elapsed_since_1900).into();
        Ok(UnixTime::since_unix_epoch(total))
    }
    pub async fn get_time_from_ntp_server(
        &self,
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
        let mut rx_buffer = [0; 4096];
        let mut tx_meta = [PacketMetadata::EMPTY; 16];
        let mut tx_buffer = [0; 4096];
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
        let (read, peer) = sock.recv_from(&mut response).await.unwrap();
        dbg!(read);
        dbg!(peer);
        let transmit_seconds = u32::from_be_bytes(response[TX_SECONDS].try_into().unwrap());
        transmit_seconds.into()
    }
}
