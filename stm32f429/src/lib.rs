#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

pub mod board;
pub mod democryptoprovider;
pub mod demotimeprovider;
mod verify;
extern crate alloc;

use alloc::{borrow::Cow, sync::Arc};
use board::Board;

use heapless::String;
use static_cell::make_static;

use core::mem::MaybeUninit;
use cortex_m_semihosting::debug;
use defmt::{unwrap, Format};
use embassy_executor::Spawner;
use embassy_net::{Stack, StackResources};
use embassy_stm32::{
    bind_interrupts,
    eth::{self, generic_smi::GenericSMI, Ethernet, PacketQueue},
    peripherals::{self, ETH, RNG},
    rng,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};
use embedded_alloc::Heap;

use spin;

use crate::demotimeprovider::get_time_from_ntp_server;
// https://dev.to/apollolabsbin/sharing-data-among-tasks-in-rust-embassy-synchronization-primitives-59hk

static RNG_MUTEX: Mutex<ThreadModeRawMutex, Option<embassy_stm32::rng::Rng<'_, RNG>>> =
    Mutex::new(None);

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

const HEAP_SIZE: usize = 1024 + 5840 + 1024 + 2048 + 1245 + 11680;
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
const TIME_BETWEEN_1900_1970: u64 = 2_208_988_800;

static ELAPSED_SINCE_1900: Mutex<ThreadModeRawMutex, Option<u64>> = Mutex::new(None);
// We don't want this to be called man times but
// START.call_once(|| {
//    embassy_futures::block_on(async {
// is not appropriate here!
pub async fn init_call_to_ntp_server(stack: &'static Stack<Ethernet<'static, ETH, GenericSMI>>) {
    let ntp_time = get_time_from_ntp_server(stack).await;
    ELAPSED_SINCE_1900.lock().await.replace(ntp_time);
}
pub fn certificate_verifier(
    roots: rustls::RootCertStore,
) -> Arc<dyn rustls::client::danger::ServerCertVerifier> {
    rustls::client::WebPkiServerVerifier::builder(roots.into())
        .with_signature_verification_algorithms(verify::ALGORITHMS)
        .build()
        .unwrap()
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
        Irqs,
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
    stack
}

pub fn http_request(server_name: &str) -> String<64> {
    const HTTP_SEPARATOR: &str = "\r\n";

    let lines = [
        Cow::Borrowed("GET / HTTP/1.1"),
        alloc::format!("Host: {server_name}").into(),
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
