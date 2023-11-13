#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

mod democryptoprovider;

extern crate alloc;

use embassy_time::{Duration, Instant};
use rustls_pki_types::UnixTime;
use static_cell::make_static;

use core::{mem::MaybeUninit, ops::Range};
use cortex_m_semihosting::debug;
use defmt::{dbg, info, unwrap};
use embassy_executor::Spawner;
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    IpAddress, IpEndpoint, Ipv4Address, Stack, StackResources,
};
use embassy_stm32::eth::{generic_smi::GenericSMI, Ethernet};
use embassy_stm32::peripherals::ETH;
use embassy_stm32::rng::Rng;
use embassy_stm32::{bind_interrupts, eth, peripherals, rng};
use embassy_stm32::{eth::PacketQueue, peripherals::RNG};
use embedded_alloc::Heap;
use spin;
// https://dev.to/apollolabsbin/sharing-data-among-tasks-in-rust-embassy-synchronization-primitives-59hk
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

static RNG_MUTEX: Mutex<ThreadModeRawMutex, Option<embassy_stm32::rng::Rng<'_, RNG>>> =
    Mutex::new(None);

// Separating the board from the network init task
pub struct Board {
    // Pins for ethernet
    peri: embassy_stm32::peripherals::ETH,
    irqs: Irqs,
    ref_clk: embassy_stm32::peripherals::PA1,
    // management data input output between PHY and MAC layers
    mdio: embassy_stm32::peripherals::PA2,
    // management data clock, for sync between PHY and MAC
    mdc: embassy_stm32::peripherals::PC1,
    // carrier sense, sensing if data is transmitted
    crs: embassy_stm32::peripherals::PA7,
    rx_d0: embassy_stm32::peripherals::PC4,
    rx_d1: embassy_stm32::peripherals::PC5,
    tx_d0: embassy_stm32::peripherals::PG13,
    tx_d1: embassy_stm32::peripherals::PB13,
    // transmit enable
    tx_en: embassy_stm32::peripherals::PG11,
    // our random souce
    rng: embassy_stm32::rng::Rng<'static, RNG>,
}

impl Board {
    pub fn new(p: embassy_stm32::Peripherals) -> Self {
        Self {
            peri: p.ETH,
            irqs: Irqs,
            ref_clk: p.PA1,
            mdio: p.PA2,
            mdc: p.PC1,
            crs: p.PA7,
            rx_d0: p.PC4,
            rx_d1: p.PC5,
            tx_d0: p.PG13,
            tx_d1: p.PB13,
            tx_en: p.PG11,
            rng: Rng::new(p.RNG, Irqs),
        }
    }
}

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

const HEAP_SIZE: usize = 1024;
#[global_allocator]
static HEAP: Heap = Heap::empty();
static START: spin::Once = spin::Once::new();

pub fn init_heap() {
    START.call_once(|| {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    });
}

pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

type Device = Ethernet<'static, ETH, GenericSMI>;

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<Device>) -> ! {
    stack.run().await
}

pub async fn network_task_init(
    spawner: Spawner,
    mut board: Board,
) -> &'static Stack<Ethernet<'static, ETH, GenericSMI>> {
    // Using RNG ...
    let mut seed = [0; 8];
    let _ = board.rng.async_fill_bytes(&mut seed).await;
    let seed = u64::from_le_bytes(seed);
    // ... before putting it in the mutex for access from other modules
    RNG_MUTEX.lock().await.replace(board.rng);

    let mac_addr = [6, 5, 4, 3, 2, 1];

    let device = Ethernet::new(
        make_static!(PacketQueue::<16, 16>::new()),
        board.peri,
        board.irqs,
        board.ref_clk,
        board.mdio,
        board.mdc,
        board.crs,
        board.rx_d0,
        board.rx_d1,
        board.tx_d0,
        board.tx_d1,
        board.tx_en,
        GenericSMI::new(0),
        mac_addr,
    );

    let config = embassy_net::Config::dhcpv4(Default::default());

    //Init network stack
    let stack = &*make_static!(Stack::new(
        device,
        config,
        //Needs more socket, or error: adding a socket to a full SocketSet
        make_static!(StackResources::<3>::new()),
        seed
    ));

    //Launch network task
    unwrap!(spawner.spawn(net_task(&stack)));
    stack.wait_config_up().await;

    info!("Network task initialized");
    stack
}
