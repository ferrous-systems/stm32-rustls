#![no_main]
#![no_std]

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::mem::MaybeUninit;
use embedded_alloc::Heap;

use f429 as _;
// global logger + panicking-behavior + memory layout

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
    let c = return_vector();
    defmt::info!("First element of arc_xs_b[0]: {:?}", c[0]);

    loop {}
}

fn return_vector() -> Arc<Vec<i32>> {
    let mut xs = Vec::new();
    xs.push(78);
    let arc_xs_a: Arc<Vec<i32>> = Arc::new(xs);
    let arc_xs_b: Arc<Vec<i32>> = arc_xs_a.clone();

    arc_xs_b
}
#[cfg(test)]
#[defmt_test::tests]
mod unit_tests {
    use super::*;
    use defmt::{assert, assert_eq};

    #[test]
    fn it_works() {
        assert!(true)
    }

    #[test]
    fn test_vector() {
        let arced_vec = return_vector();
        assert_eq!(78, arced_vec[0]);
    }
}
