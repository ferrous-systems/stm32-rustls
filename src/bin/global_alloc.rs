#![no_main]
#![no_std]

extern crate alloc;
use alloc::vec::Vec;
use core::mem::MaybeUninit;
use embedded_alloc::Heap;

use f429 as _;
use stm32f4xx_hal::{pac, prelude::*}; // global logger + panicking-behavior + memory layout

const HEAP_SIZE: usize = 1024;
#[global_allocator]
static HEAP: Heap = Heap::empty();

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello Alloc!");
    let p = pac::Peripherals::take().unwrap();

    {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let mut xs = Vec::new();
    xs.push(78);

    defmt::info!("{:?}", xs[0]);
    loop {}
}
