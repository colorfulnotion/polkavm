#![no_std]
#![no_main]
#![feature(asm_const)]

extern crate alloc;
use alloc::format;

const SIZE0: usize = 0x10000;
// allocate memory for stack
use polkavm_derive::min_stack_size;
min_stack_size!(SIZE0);

const SIZE1: usize = 0x10000;
// allocate memory for heap
use simplealloc::SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc<SIZE1> = SimpleAlloc::new();

use utils::constants::{FIRST_READABLE_ADDRESS, NONE};
use utils::functions::{call_log, parse_accumulate_args, parse_transfer_args, write_result};
use utils::host_functions::{export, fetch, gas, write};

#[polkavm_derive::polkavm_export]
extern "C" fn refine(_start_address: u64, _length: u64) -> (u64, u64) {
    let offset: u64 = 0;
    let maxlen: u64 = unsafe { buffer.len() as u64 };
    let result = unsafe { fetch(buffer.as_mut_ptr() as u64, offset, maxlen, 5, 0, 0) }; // fetch segment 0 from work item 0

    if result != NONE {
        let n = unsafe { u32::from_le_bytes(buffer[0..4].try_into().unwrap()) };
        let fib_n = unsafe { u32::from_le_bytes(buffer[4..8].try_into().unwrap()) };
        let fib_n_minus_1 = unsafe { u32::from_le_bytes(buffer[8..12].try_into().unwrap()) };

        let new_fib_n = fib_n + fib_n_minus_1;
        unsafe {
            buffer[0..4].copy_from_slice(&(n + 1).to_le_bytes());
            buffer[4..8].copy_from_slice(&new_fib_n.to_le_bytes());
            buffer[8..12].copy_from_slice(&fib_n.to_le_bytes());
        }
    } else {
        unsafe {
            buffer[0..4].copy_from_slice(&1_u32.to_le_bytes());
            buffer[4..8].copy_from_slice(&1_u32.to_le_bytes());
            buffer[8..12].copy_from_slice(&0_u32.to_le_bytes());
            buffer[13] = 1;
            buffer[14] = 2;
            buffer[15] = 3;
            let mut n = 16;
            while n < 4104 {
                buffer[n] = ((buffer[n - 1] as u16 + buffer[n - 2] as u16 + buffer[n - 3] as u16) % 256) as u8;

                n += 1;
            }
        }
    }

    unsafe {
        export(buffer.as_ptr() as u64, buffer.len() as u64);
    }

    // set the output address to register a0 and output length to register a1
    let buffer_addr = unsafe { buffer.as_ptr() as u64 };
    let buffer_len = unsafe { buffer.len() as u64 };
    return (buffer_addr, buffer_len);
}

#[no_mangle]
static mut output_bytes_32: [u8; 32] = [0; 32];
#[no_mangle]
static mut buffer: [u8; 4104] = [0u8; 4104];

#[polkavm_derive::polkavm_export]
extern "C" fn accumulate(start_address: u64, length: u64) -> (u64, u64) {
    // parse accumulate args
    let (_timeslot, _service_index, work_result_address, work_result_length) =
        if let Some(args) = parse_accumulate_args(start_address, length, 0) {
            (args.t, args.s, args.work_result_ptr, args.work_result_len)
        } else {
            return (FIRST_READABLE_ADDRESS as u64, 0);
        };

    // write FIB result to storage
    let key = [0u8; 1];
    let _n: u64 = unsafe { (*(work_result_address as *const u32)).into() };
    unsafe {
        write(key.as_ptr() as u64, key.len() as u64, work_result_address, work_result_length);
    }

    // Option<hash> test
    // pad result to 32 bytes
    let output_address: u64;
    let output_length: u64;
    unsafe {
        output_bytes_32[..work_result_length as usize].copy_from_slice(&core::slice::from_raw_parts(
            work_result_address as *const u8,
            work_result_length as usize,
        ));

        output_address = core::ptr::addr_of!(output_bytes_32) as u64;
        output_length = 32_u64;
    }

    return (output_address, output_length);
}

#[polkavm_derive::polkavm_export]
extern "C" fn on_transfer(start_address: u64, length: u64) -> (u64, u64) {
    // Note: This part executes only if there are deferred transfers AND this service is the receiver.
    let mut i: u64 = 0;

    loop {
        let (timeslot, service_index, sender, receiver, amount, memo, gas_limit) =
            if let Some(args) = parse_transfer_args(start_address, length, i) {
                (args.t, args.s, args.ts, args.td, args.ta, args.tm, args.tg)
            } else {
                break;
            };

        call_log(
            2,
            None,
            &format!(
                "FIB on_transfer: timeslot={:?} service_index={:?} sender={:?} receiver={:?} amount={:?} memo={:?} gas_limit={:?}",
                timeslot, service_index, sender, receiver, amount, memo, gas_limit
            ),
        );

        let service_index_bytes = service_index.to_le_bytes();
        let service_index_ptr: u64 = service_index_bytes.as_ptr() as u64;
        let service_index_length: u64 = service_index_bytes.len() as u64;

        let memo_ptr: u64 = memo.as_ptr() as u64;
        let memo_length: u64 = memo.len() as u64;

        unsafe { write(service_index_ptr, service_index_length, memo_ptr, memo_length) };

        let gas_result = unsafe { gas() };
        write_result(gas_result, 4);
        call_log(2, None, &format!("FIB on_transfer gas: got {:?} (recorded at key 4)", gas_result));

        i += 1;
    }

    return (FIRST_READABLE_ADDRESS as u64, 0);
}
