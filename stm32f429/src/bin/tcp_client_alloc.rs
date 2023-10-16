#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
extern crate alloc;
use alloc::vec;

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{Ipv4Address, Stack, StackResources};
use embassy_stm32::eth::generic_smi::GenericSMI;
use embassy_stm32::eth::Ethernet;
use embassy_stm32::eth::PacketQueue;
use embassy_stm32::peripherals::ETH;
use embassy_stm32::rng::Rng;
use embassy_stm32::time::mhz;
use embassy_stm32::{bind_interrupts, eth, peripherals, rng, Config};
use embassy_time::{driver, Duration, Instant, Timer};
use static_cell::make_static;
use stm32f429 as _;
use stm32f429::init_heap;
use stm32f429::UnixTime;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

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
    let mut now = Instant::now();
    let since_1970 = UnixTime::now();
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
        make_static!(StackResources::<2>::new()),
        seed
    ));

    // Launch network task
    unwrap!(spawner.spawn(net_task(&stack)));

    //info!("Network task initialized");

    // Then we can use it!
    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    // send a hello message
    init_heap();
    let msg = vec![104, 101, 108, 108, 111];

    loop {
        info!("since_1970 {:?}", Debug2Format(&since_1970));
        info!(
            "NOW {:?}",
            Debug2Format(&(now.elapsed() + since_1970.as_duration()))
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
