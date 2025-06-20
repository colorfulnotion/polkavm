#![no_std]
#![no_main]
#![feature(asm_const)]

extern crate alloc;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
const SIZE0: usize = 0x10000;
use alloc::boxed::Box;
// allocate memory for stack
use polkavm_derive::min_stack_size;
min_stack_size!(SIZE0);

const SIZE1: usize = 0x10000;
// allocate memory for heap
use simplealloc::SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc<SIZE1> = SimpleAlloc::new();

use utils::constants::FIRST_READABLE_ADDRESS;
use utils::functions::call_log;
use utils::functions::parse_accumulate_args;
use utils::functions::parse_refine_args;
use utils::hash_functions::blake2b_hash;
use utils::host_functions::{assign, fetch}; // Added 'export' here

fn build_combined_input(input: &[u8], wi_payload_address: u64, wi_payload_length: usize) -> Vec<u8> {
    let wi_payload = unsafe { core::slice::from_raw_parts(wi_payload_address as *const u8, wi_payload_length) };

    let mut buffer = Vec::with_capacity(input.len() + wi_payload.len());
    buffer.extend_from_slice(input);
    buffer.extend_from_slice(wi_payload);
    buffer
}

#[polkavm_derive::polkavm_export]
extern "C" fn refine(start_address: u64, length: u64) -> (u64, u64) {

    let p_u_32: [u8; 32] = [0u8; 32];
    let p_u_32_ptr = p_u_32.as_ptr() as u64;
    // Datatype 8: Should be fetching 32 bytes of p_u
    let result0 = unsafe { fetch(p_u_32_ptr, 0, 32, 8, 0, 0) }; 
    let input_slice = unsafe { core::slice::from_raw_parts(p_u_32.as_ptr(), 32) };
   
    // use fetch to get 4 byte payload of work item 0
    let payload_4: [u8; 4] = [0u8; 4];     // 'a' value
    let payload_4_ptr = payload_4.as_ptr() as u64;
    // Datatype 13: Should be fetching 4 bytes for 'a'
    let payload_result0 = unsafe { fetch(payload_4_ptr, 0, 4, 13, 0, 0) };

    let (_wi_service_index, wi_payload_start_address, _wi_payload_length, _wphash) =
        if let Some(args) = parse_refine_args(start_address, length) {
            (
                args.wi_service_index,
                args.wi_payload_start_address,
                args.wi_payload_length,
                args.wphash,
            )
        } else {
            return (FIRST_READABLE_ADDRESS as u64, 0);
        };

    // Change final_input_for_hash to be a Vec<u8> so it owns the data
    let mut final_input_for_hash: Vec<u8> = input_slice.to_vec(); // Initialize with a copy of input_slice

    if _wi_payload_length > 0 {
        // build_combined_input returns a Vec<u8>, directly assign it to final_input_for_hash
        final_input_for_hash = build_combined_input(input_slice, wi_payload_start_address, _wi_payload_length as usize);
    }

    let output = blake2b_hash(&final_input_for_hash); 
    unsafe {
      output_bytes_36[0..32].copy_from_slice(&output[0..32]);
      output_bytes_36[32..36].copy_from_slice(&payload_4);
    }

    let output_address = unsafe { output_bytes_36.as_ptr() as u64 };
    let output_length = unsafe { output_bytes_36.len() as u64 };

    unsafe {
        //call_log(2, None, &format!("auth_copy REFINE authHash={:x?}", output));
        //call_log(2, None, &format!("auth_copy REFINE payload_4={:x?}", payload_4));
        call_log(2, None, &format!("auth_copy REFINE output_bytes_36={:x?}", output_bytes_36));
    } 
    return (output_address, output_length);
}

const N_Q: usize = 80;
#[no_mangle]
static mut operand: [u8; 4104] = [0u8; 4104];
static mut output_bytes_36: [u8; 36] = [0u8; 36];
static mut authorization_hashes: [u8; 32 * N_Q] = [0u8; 32 * N_Q];
    
#[polkavm_derive::polkavm_export]
extern "C" fn accumulate(start_address: u64, length: u64) -> (u64, u64) {
    let output_bytes_36_ptr = unsafe { output_bytes_36.as_ptr() as u64 };
    let (_timeslot, _service_index, num_of_operands) = match parse_accumulate_args(start_address, length) {
      Some(args) => (args.t, args.s, args.number_of_operands),
      None => return (FIRST_READABLE_ADDRESS as u64, 0),
    };


    // fetch 36 byte output which will be (32 byte p_u_hash + 4 byte "a" from payload y)
    let operand_ptr = unsafe { operand.as_ptr() as u64 };
    let operand_len = unsafe { fetch(operand_ptr, 0, 4104, 15, 0, 0) }; 

    // copy the last 36 bytes of the operand to output_bytes_36
    if operand_len < 36 {
         unsafe {
            call_log(2, None, &format!("auth_copy ACC operand_len={} is less than 36 bytes, returning error", operand_len));
         }
         return (FIRST_READABLE_ADDRESS as u64, 0);
    }
    let start = operand_len as usize - 36;
    for j in 0..36 {
        unsafe {
          output_bytes_36[j] = operand[start+j];
	}
    }
    let a: u32 = unsafe {
	   u32::from_le_bytes(
            output_bytes_36[32..36]      
                .try_into()
                .expect("slice length is exactly 4"),
           )
    };
    
    unsafe {
        let p_u_hash = &output_bytes_36[..32]; 
        for i in 0..N_Q {   
            let start = i * 32;
            authorization_hashes[start..start + 32].copy_from_slice(p_u_hash);
        }
    }
    let authorization_hashes_address = unsafe {
        authorization_hashes.as_ptr() as u64
    };
    unsafe {
        assign(0, authorization_hashes_address, a as u64);
        assign(1, authorization_hashes_address, a as u64);
        call_log(2, None, &format!("assigned core 0+1 service {}", a));
    }
    (output_bytes_36_ptr, 32)
}

#[polkavm_derive::polkavm_export]
extern "C" fn on_transfer(_start_address: u64, _length: u64) -> (u64, u64) {
    return (FIRST_READABLE_ADDRESS as u64, 0);
}