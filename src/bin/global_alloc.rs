#![no_main]
#![no_std]

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use core::mem::MaybeUninit;
use embedded_alloc::Heap;
use f429::{self as _, exit};
use spin;
// global logger + panicking-behavior + memory layout

const HEAP_SIZE: usize = 1024;
#[global_allocator]
static HEAP: Heap = Heap::empty();
static START: spin::Once = spin::Once::new();
#[cortex_m_rt::entry]
// remove the main function when compiling
// if not it will be 2 entry points!
#[cfg(not(test))]
fn main() -> ! {
    init_heap();

    let c = return_vector();

    let d = vec![1, 2, 3];
    defmt::info!("Element of d: {:?}", d[0]);
    defmt::info!("First element of arc_xs_b[0]: {:?}", c[0]);
    defmt::info!("Last element of arc_xs_b[0]: {:?}", c[3]);

    exit();
}

// make it safe
// make an atomic variable
// check Once
// https://docs.rs/spin/0.9.8/spin/once/struct.Once.html
// do not reinitialize the heap!!

fn init_heap() {
    START.call_once(|| {
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe {
            HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE);
        }
    });
}

fn return_vector() -> Arc<Vec<i32>> {
    let mut xs = Vec::new();
    xs.push(78);
    xs.push(79);
    xs.push(80);
    xs.push(108);
    let arc_xs_a: Arc<Vec<i32>> = Arc::new(xs);
    let arc_xs_b: Arc<Vec<i32>> = arc_xs_a.clone();
    arc_xs_b
}

#[cfg(test)]
#[defmt_test::tests]
mod tests {
    use super::*;
    use defmt::{assert, assert_eq};

    #[test]
    fn it_works() {
        assert!(true)
    }

    #[test]
    fn test_global_alloc() {
        init_heap();
        let arced_vec = return_vector();
        assert_eq!(78, arced_vec[0]);
    }
}
