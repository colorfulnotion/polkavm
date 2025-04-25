#![no_std]
#![no_main]
#![feature(asm_const)] 

extern crate alloc;
use alloc::format;
use alloc::vec;

// allocate memory for stack
use polkavm_derive::min_stack_size;
min_stack_size!(16773119); // 2^24 - 1 - 4096, should not greater than 2^24 - 1 (16777215)

// allocate memory for heap
use simplealloc::SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc<16773119> = SimpleAlloc::new(); // 2^24 - 1 - 4096, should not greater than 2^24 - 1 (16777215)

use utils::functions::{call_log};
use utils::constants::{PAGE_SIZE,INPUT_ARGS_ADDRESS};

#[polkavm_derive::polkavm_export]
extern "C" fn main(n: u64) -> (u64, u64) {
    let result: u32;
    if n < 2 {
        // fib(0) = 1, fib(1) = 1
        result = 1;
        call_log(2, None, &format!("CHILD BASE n={:?} result={:?}", n, result));
    } else {
        // fib(n) = fib(n-2) + fib(n-1)
        unsafe {
            let addr2 = (INPUT_ARGS_ADDRESS as u64) + (n-2) * PAGE_SIZE;
            let fib_n_2 = core::ptr::read_volatile(addr2 as *const u32);
            let addr1 = (INPUT_ARGS_ADDRESS as u64) + (n-1) * PAGE_SIZE;
            let fib_n_1 = core::ptr::read_volatile(addr1 as *const u32);
            result = fib_n_2 + fib_n_1;
            call_log(2, None, &format!("CHILD NORM n={:?} fib_n_2={:?} (addr2={:?}) fib_n_1={:?} (addr1={:?}) result={:?}", n, fib_n_2, addr2, fib_n_1, addr1, result));
        }
    }
    
    let out_address = INPUT_ARGS_ADDRESS + (n * PAGE_SIZE) as u32;
    call_log(2, None, &format!("CHILD WRITE out_address={:?} result={:?}", out_address, result));
    unsafe {
        core::ptr::write_volatile(out_address  as *mut u32, result);
    }
    return (out_address as u64, PAGE_SIZE);
}
