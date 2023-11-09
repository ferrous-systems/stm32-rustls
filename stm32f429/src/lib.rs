#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

mod aead;
mod hash;
mod hmac;
mod kx;
extern crate alloc;
// https://dev.to/apollolabsbin/sharing-data-among-tasks-in-rust-embassy-synchronization-primitives-59hk
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
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
use embassy_stm32::time::mhz;
use embassy_stm32::{bind_interrupts, eth, peripherals, rng, Config};
use embassy_stm32::{eth::PacketQueue, peripherals::RNG};
use embedded_alloc::Heap;
use rustls::crypto::CryptoProvider;
use spin;

static ALL_CIPHER_SUITES: &[rustls::SupportedCipherSuite] =
    &[TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256];

// static RNG_CELL: StaticCell<Mutex<ThreadModeRawMutex, embassy_stm32::rng::Rng<'_, RNG>>> =
//     StaticCell::new();

static RNG_MUTEX: Mutex<ThreadModeRawMutex, Option<embassy_stm32::rng::Rng<'_, RNG>>> = Mutex::new(None);

pub static TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256: rustls::SupportedCipherSuite =
    rustls::SupportedCipherSuite::Tls12(&rustls::Tls12CipherSuite {
        common: rustls::cipher_suite::CipherSuiteCommon {
            suite: rustls::CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
            hash_provider: &hash::Sha256,
        },
        kx: rustls::crypto::KeyExchangeAlgorithm::ECDHE,
        sign: &[
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
        ],
        prf_provider: &rustls::crypto::tls12::PrfUsingHmac(&hmac::Sha256Hmac),
        aead_alg: &aead::Chacha20Poly1305,
    });
#[derive(Debug)]
struct DemoCryptoProvider;
impl CryptoProvider for DemoCryptoProvider {
    fn fill_random(&self, bytes: &mut [u8]) -> Result<(), rustls::crypto::GetRandomFailed> {
        // This is a non async task so I need embassy_futures
        embassy_futures::block_on(async {
            let mut  binding = RNG_MUTEX.lock().await;
            let rng = binding.as_mut().unwrap();
            rng.async_fill_bytes(bytes).
                await.map_err(|_| rustls::crypto::GetRandomFailed)
        })
    }

    fn default_cipher_suites(&self) -> &'static [rustls::SupportedCipherSuite] {
        ALL_CIPHER_SUITES
    }

    fn default_kx_groups(&self) -> &'static [&'static dyn rustls::crypto::SupportedKxGroup] {
        kx::ALL_KX_GROUPS
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

pub fn now_plus_elapsed_since_1900(unix: u64, monotonic_now: u64) -> u64 {
    monotonic_now + unix
}

pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

// wrap that in a trait object
// the method must be wrapped in a struct
// this is the init
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

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});
type Device = Ethernet<'static, ETH, GenericSMI>;

#[embassy_executor::task]
async fn net_task(stack: &'static Stack<Device>) -> ! {
    stack.run().await
}

pub async fn network_task_init(
    spawner: Spawner,
) -> &'static Stack<Ethernet<'static, ETH, GenericSMI>> {
    let mut config = Config::default();
    config.rcc.sys_ck = Some(mhz(100));
    let p = embassy_stm32::init(config);
    // Using RNG ...
    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    let _ = rng.async_fill_bytes(&mut seed).await;
    let seed = u64::from_le_bytes(seed);
    // ... before putting it in the mutex
    RNG_MUTEX.lock().await.replace(rng);

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
        //Needs more socket, or error: adding a socket to a full SocketSet
        make_static!(StackResources::<3>::new()),
        seed
    ));

    // Launch network task
    unwrap!(spawner.spawn(net_task(&stack)));
    stack.wait_config_up().await;

    info!("Network task initialized");
    stack
}
