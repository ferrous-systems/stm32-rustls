#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
extern crate alloc;
use core::ops::Range;

use alloc::vec;

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::udp::{PacketMetadata, UdpSocket};

use embassy_net::dns::DnsQueryType;
use embassy_net::{IpAddress, IpEndpoint, Ipv4Address, Stack, StackResources};
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::eth::Ethernet;
use embassy_stm32::eth::PacketQueue;
use embassy_stm32::peripherals::ETH;
use embassy_stm32::rng::Rng;
use embassy_stm32::time::mhz;
use embassy_stm32::{bind_interrupts, eth, peripherals, rng, Config};
use embassy_time::{Duration, Instant, Timer};
use rustls_pki_types::UnixTime;
use static_cell::make_static;
use stm32f429 as _;
use stm32f429::exit;
use stm32f429::init_heap;
use stm32f429::now_plus_elapsed_since_1970;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

const DURATION_SINCE_1970: u64 = 1697530048;

type Device = Ethernet<'static, ETH, GenericSMI>;

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<Device>) -> ! {
    stack.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) -> ! {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);

    let now = Instant::now();

    // Must apparently be converted in coretime duration
    let since_1970 = UnixTime::since_unix_epoch(Duration::from_secs(DURATION_SINCE_1970).into());
    // Generate random seed.
    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    let _ = rng.async_fill_bytes(&mut seed).await;
    let seed = u64::from_le_bytes(seed);

    let mac_addr = [6, 5, 4, 3, 2, 1];

    let device = Ethernet::new(
        make_static!(PacketQueue::<16, 16>::new()),
        p.ETH,
        Irqs,
        p.PA1,
        p.PA2,
        p.PC1,
        p.PA7,
        p.PC4,
        p.PC5,
        p.PG13,
        p.PB13,
        p.PG11,
        GenericSMI::new(0),
        mac_addr,
    );

    let config = embassy_net::Config::dhcpv4(Default::default());

    // Init network stack
    let stack = &*make_static!(Stack::new(
        device,
        config,
        //dns_socket,
        make_static!(StackResources::<4>::new()),
        seed
    ));

    stack.wait_config_up().await;
    // Launch network task
    unwrap!(spawner.spawn(net_task(&stack)));

    //info!("Network task initialized");

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    //let current_time = get_time_from_ntp_server(&stack).await;
    //exit();

    // send a hello message
    init_heap();
    let msg = vec![104, 101, 108, 108, 111];
    info!("now we are trying with this dnssocket");
    // let ntp_host = "pool.ntp.org:123";

    // match stack.dns_query(ntp_host, DnsQueryType::A).await {
    //     Ok(r) => {
    //         info!("response {}", r)
    //     }
    //     Err(e) => {
    //         warn!("error {}", e)
    //     }
    // }
    //let dns_socket = Stack::dns_query(&stack, "google.com", DnsQueryType::Aaaa).await;

    //dbg!(dns_socket);

    get_time_from_ntp_server(stack).await;
    loop {
        //     info!("since_1970 {:?}", Debug2Format(&since_1970));
        //     info!(
        //         "now.elapsed().as_secs() {:?}",
        //         Debug2Format(&now.elapsed().as_secs())
        //     );
        //     info!(
        //         "NOW now_elapsed_since_1970 {:?}",
        //         Debug2Format(
        //             &(now_plus_elapsed_since_1970(since_1970.as_secs(), now.elapsed().as_secs()))
        //         )
        //     );
        //     let mut socket = embassy_net::tcp::TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        //     socket.set_timeout(Some(Duration::from_secs(1000)));
        //     let add = "192.168.50.67".parse::<Ipv4Address>().unwrap();
        //     info!("Listening on TCP:1234...");
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
    }
}

async fn get_time_from_ntp_server(stack: &'static Stack<Device>) -> u32 {
    const NTP_PACKET_SIZE: usize = 48;
    const TX_SECONDS: Range<usize> = 40..44;

    let ntp_server = IpEndpoint {
        // 162.159.200.1:123
        addr: IpAddress::Ipv4(Ipv4Address::new(162, 159, 200, 1)),
        port: 123,
    };

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    let mut sock = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    sock.bind(45698).unwrap();

    let mut request = [0u8; NTP_PACKET_SIZE];
    // this magic number means
    // - use NTPv3
    // - we are a client
    request[0] = 0x1b;

    debug!("before SENDING TO");
    sock.send_to(&request, ntp_server).await.unwrap();
    debug!("after SENDING TO");
    // reuse buffer
    // why that? such a small buffer of 48 bytes
    let mut response = request;
    debug!("before RECEIVE FROM");
    //let (read, peer) = sock.recv_from(&mut response).await.unwrap();
    //assert_eq!(NTP_PACKET_SIZE, read);

    for i in 1..10 {
        match sock.recv_from(&mut response).await {
            Ok(read) => {
                dbg!(read);
            }
            Err(e) => {
                dbg!(e);
            }
        };
    }

    // loop {
    //     let (n, ep) = sock.recv_from(&mut response).await.unwrap();
    //     if let Ok(s) = core::str::from_utf8(&buf[..n]) {
    //         info!("rxd from {}: {}", ep, s);
    //         break;
    //     }
    // }

    // seem to be stuck for ever.. try different server or DNS resolution
    debug!("AFTER RECEIVE FROM");
    // take note of the IP address
    // ok [src/bin/sketch.rs:38] peer = 192.121.108.100:123
    // no DNS resolution, that IP will change
    // NTP server can go down and the add can be updated on the mcu
    //dbg!(peer);

    // how does this fills? ah ok this is why we have range 40..44
    // the packet is 48 bytes
    // 1 bytes == header with protocol
    // then bytes 40..44 --> represents the ntp time in seconds
    // # of seconds that have elapsed from NTP epoch = 3_906_531_996 etc, like the stuff from 1970's

    let transmit_seconds = u32::from_be_bytes(response[TX_SECONDS].try_into().unwrap());
    info!("TRANSMIT SECONDS!!!!!!!! {}", transmit_seconds);
    transmit_seconds
}
