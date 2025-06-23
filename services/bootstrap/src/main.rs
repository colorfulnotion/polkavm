#![no_std]
#![no_main]
#![feature(asm_const)]

extern crate alloc;
use alloc::boxed::Box;
use alloc::format;
use alloc::vec;
const SIZE0: usize = 0x10000;
// allocate memory for stack
use polkavm_derive::min_stack_size;
min_stack_size!(SIZE0);

const SIZE1: usize = 0x10000;
// allocate memory for heap
use simplealloc::SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc<SIZE1> = SimpleAlloc::new();

use utils::constants::FIRST_READABLE_ADDRESS;
use utils::functions::{call_log, parse_accumulate_args, parse_refine_args};
use utils::host_functions::{fetch, new, transfer, write};

#[polkavm_derive::polkavm_export]
extern "C" fn refine(start_address: u64, length: u64) -> (u64, u64) {
    // parse refine args
    let (_wi_index, _wi_service_index, wi_payload_start_address, wi_payload_length, _wphash) =
        if let Some(args) = parse_refine_args(start_address, length) {
            (
                args.wi_index,
                args.wi_service_index,
                args.wi_payload_start_address,
                args.wi_payload_length,
                args.wphash,
            )
        } else {
            return (FIRST_READABLE_ADDRESS as u64, 0);
        };

    return (wi_payload_start_address, wi_payload_length);
}

#[no_mangle]
static mut output_bytes_36: [u8; 36] = [0; 36];

#[polkavm_derive::polkavm_export]
extern "C" fn accumulate(start_address: u64, length: u64) -> (u64, u64) {
    // parse args
    let (_timeslot, _service_index, number_of_operands) = if let Some(args) = parse_accumulate_args(start_address, length) {
        (args.t, args.s, args.number_of_operands)
    } else {
        return (FIRST_READABLE_ADDRESS as u64, 0);
    };

    let ptr = output_bytes_36.as_ptr() as u64;

    for i in 0..number_of_operands {
        let result0 = unsafe { fetch(ptr, 0, 36, 15, i.into(), 0) };
        // len is the 32..36 of the output_bytes_36 which is output by refine result  
        let len: u32 = unsafe {
            u32::from_le_bytes(
                    output_bytes_36[32..36]      
                        .try_into()
                        .expect("slice length is exactly 4"),
            )
        };
       
        call_log(2, None, &format!("createService output_bytes_address {} {:?}", ptr, output_bytes_36));
        let omega_9: u64 = 100;  // g
        let omega_10: u64 = 100;  // m
        let omega_11: u64 = 1024; // gratis f

        let result = unsafe { new(ptr, len as u64, omega_9, omega_10, omega_11) };
        let result_bytes = &result.to_le_bytes()[..4];
        // write result to storage
        let storage_key: [u8; 4] = (i as u32).to_le_bytes();
        unsafe {
            write(
                storage_key.as_ptr() as u64,
                storage_key.len() as u64,
                result_bytes.as_ptr() as u64,
                result_bytes.len() as u64,
            );
        }
        call_log(2, None, &format!("SERVICEID={} storage_key={:x?}", result, storage_key));
        let memo = [0u8; 128];
        unsafe {
            transfer(result, 500000, 100, memo.as_ptr() as u64);
        }
        let start = (i * 4) as usize;
        let end = ((i + 1) * 4) as usize;
        output_bytes_36[start..end].copy_from_slice(result_bytes);
    }
    (ptr, 32)
}

#[polkavm_derive::polkavm_export]
extern "C" fn on_transfer(_start_address: u64, _length: u64) -> (u64, u64) {
    return (FIRST_READABLE_ADDRESS as u64, 0);
}
