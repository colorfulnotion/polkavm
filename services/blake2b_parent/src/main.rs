#![no_std]
#![no_main]
#![feature(asm_const)] 

extern crate alloc;
use alloc::format;
use alloc::vec;

const SIZE0 : usize = 0x10000;
// allocate memory for stack
use polkavm_derive::min_stack_size;
min_stack_size!(SIZE0);

const SIZE1 : usize = 0x8000000;
// allocate memory for heap
use simplealloc::SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc<SIZE1> = SimpleAlloc::new();

use utils::constants::{FIRST_READABLE_ADDRESS, OOG};
use utils::functions::{
    parse_accumulate_args, parse_refine_args, parse_standard_program_initialization_args,
    call_log, standard_program_initialization_for_child, initialize_pvm_registers,
    serialize_gas_and_registers,extract_memory_from_machine,write_memory_to_machine, deserialize_gas_and_registers
};
use utils::host_functions::{write, machine, invoke, expunge,peek};
use utils::hash_functions::blake2b_hash;

#[polkavm_derive::polkavm_export]
extern "C" fn refine(start_address: u64, length: u64) -> (u64, u64) {
   const CHILDCODE: &[u8] = include_bytes!("../../blake2b_child/blake2b_child.pvm");
    let raw_blob_address = CHILDCODE.as_ptr() as u64;
    let raw_blob_length = CHILDCODE.len() as u64;

   let (z, s, o_bytes_address, o_bytes_length, w_bytes_address, w_bytes_length, c_bytes_address, c_bytes_length) =
        if let Some(blob) = parse_standard_program_initialization_args(raw_blob_address, raw_blob_length) {
            (
                blob.z,
                blob.s,
                blob.o_bytes_address,
                blob.o_bytes_length,
                blob.w_bytes_address,
                blob.w_bytes_length,
                blob.c_bytes_address,
                blob.c_bytes_length,
            )
        } else {
            call_log(1, None, &format!("Parent: parse_standard_program_initialization_args failed for raw_blob_address: {:?} raw_blob_length: {:?}", raw_blob_address, raw_blob_length));
            return (FIRST_READABLE_ADDRESS as u64, 0);
        };
    call_log(2, None, &format!("Parent: z={:?} s={:?} o_bytes_address={:?} o_bytes_length={:?} w_bytes_address={:?} w_bytes_length={:?} c_bytes_address={:?} c_bytes_length={:?}",
        z, s, o_bytes_address, o_bytes_length, w_bytes_address, w_bytes_length, c_bytes_address, c_bytes_length));
    // new child VM
    let new_machine_idx = unsafe { machine(c_bytes_address, c_bytes_length, 0) };
    call_log(2, None, &format!("Parent: machine new index={:?}", new_machine_idx));

    // StandardProgramInitializationForChild
    standard_program_initialization_for_child(
        z,
        s,
        o_bytes_address,
        o_bytes_length,
        w_bytes_address,
        w_bytes_length,
        new_machine_idx as u32,
    );
    call_log(2, None, &format!("Parent: standard_program_initialization_for_child done"));

    // init 100 gas
    // invoke child VM
    let mut init_gas: u64 = 100;
    let mut child_vm_registers = initialize_pvm_registers();
    let g_w = serialize_gas_and_registers(init_gas, &child_vm_registers);
    let g_w_address = g_w.as_ptr() as u64;
    let (invoke_result, omega_8) = unsafe { invoke(new_machine_idx as u64, g_w_address) };
    if invoke_result == OOG  {
        call_log(1, None, &format!("Parent: invoke failed with result: {:?}", invoke_result));
    }
    let extracted_memory = match utils::functions::extract_memory_from_machine(
        z,
        s,
        o_bytes_address,
        o_bytes_length,
        w_bytes_address,
        w_bytes_length,
        new_machine_idx as u32,
    ) {
        Ok(mem) => mem,
        Err(e) => {
            call_log(1, None, &format!("Parent: extract_memory_from_machine failed: {:?}", e));
            return (FIRST_READABLE_ADDRESS as u64, 0);
        }
    };
    (_, child_vm_registers) = deserialize_gas_and_registers(&g_w);
    let pc_counter = unsafe { expunge(new_machine_idx as u64) }; 
    let new_machine_idx_2 = unsafe { machine(c_bytes_address, c_bytes_length, pc_counter) };
    call_log(2, None, &format!("Parent: machine new index after expunge={:?}", new_machine_idx_2));
    
    if let Err(e) = write_memory_to_machine(&extracted_memory, new_machine_idx_2 as u32) {
        call_log(1, None, &format!("Parent: write_memory_to_machine failed: {:?}", e));
        return (FIRST_READABLE_ADDRESS as u64, 0);
    }
    call_log(2, None, &format!("Parent: write_memory_to_machine done"));
    init_gas = 0x0FFF_FFFF_FFFF_FFFF;
    let new_gw = serialize_gas_and_registers(init_gas, &child_vm_registers);
    let g_w_address = new_gw.as_ptr() as u64;
    // invoke child VM again
    let (invoke_result, omega_8) = unsafe { invoke(new_machine_idx_2 as u64, g_w_address) };
    call_log(2, None, &format!("Parent: invoke again result={:?} omega_8={:?}", invoke_result, omega_8));
    let (gas, child_vm_registers) = deserialize_gas_and_registers(&new_gw);
    let hash_output_address = child_vm_registers[7];
    let hash_length = child_vm_registers[8];
    call_log(2, None, &format!("Parent: hash output address={:?} length={:?}", hash_output_address, hash_length));
    // read the hash result from out 
    let mut hash_result = [0u8; 32];
    let peek_result = unsafe{peek(new_machine_idx_2 as u64,  hash_result.as_mut_ptr() as u64,hash_output_address, 32)};
    if peek_result != 0 {
        call_log(1, None, &format!("Parent: peek failed with result: {:?}", peek_result));
        return (FIRST_READABLE_ADDRESS as u64, 0);
    }
   return (hash_output_address, 32); // return the address and length of the hash result

}

#[polkavm_derive::polkavm_export]
extern "C" fn accumulate(start_address: u64, length: u64) -> (u64, u64) {
    return (FIRST_READABLE_ADDRESS as u64, 0);
}

#[polkavm_derive::polkavm_export]
extern "C" fn on_transfer(_start_address: u64, _length: u64) -> (u64, u64) {
    return (FIRST_READABLE_ADDRESS as u64, 0);
}
