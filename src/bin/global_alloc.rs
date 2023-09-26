#![no_main]
#![no_std]

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::mem::MaybeUninit;
use embedded_alloc::Heap;

use f429 as _;
use stm32f4xx_hal::pac; // global logger + panicking-behavior + memory layout

const HEAP_SIZE: usize = 1024;
#[global_allocator]
static HEAP: Heap = Heap::empty();

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello Alloc!");
    {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let mut xs = Vec::new();
    xs.push(78);
    let arc_xs_a: Arc<Vec<i32>> = Arc::new(xs);
    let arc_xs_b: Arc<Vec<i32>> = arc_xs_a.clone();

    defmt::info!("arc_xs_b[0]: {:?}", arc_xs_b[0]);
    loop {}
}
