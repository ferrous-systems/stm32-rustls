#![no_main]
#![no_std]

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use f429::init_heap;

use f429::{self as _, exit};

// global logger + panicking-behavior + memory layout

#[cortex_m_rt::entry]
// remove the main function when compiling
// if not it will be 2 entry points!
#[cfg(not(test))]
fn main() -> ! {
    use alloc::vec;

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
