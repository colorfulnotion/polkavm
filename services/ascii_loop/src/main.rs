#![no_std]
#![no_main]
#![feature(asm_const)] 

extern crate alloc;
use alloc::format;
use alloc::vec;
use alloc::string::String;

const SIZE0 : usize = 0x100000;
// allocate memory for stack
use polkavm_derive::min_stack_size;
min_stack_size!(SIZE0);

const SIZE1 : usize = 0x8000000;
// allocate memory for heap
use simplealloc::SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc<SIZE1> = SimpleAlloc::new();

use utils::constants::{FIRST_READABLE_ADDRESS, FIRST_READABLE_PAGE, NONE, SEGMENT_SIZE};
use utils::functions::{call_log};
use utils::host_functions::{export, fetch};
fn encode_image(width: u16, height: u16, pixels: &[u8]) -> vec::Vec<u8> {
    let mut buf = vec::Vec::with_capacity(4 + pixels.len());
    buf.extend_from_slice(&width.to_be_bytes());
    buf.extend_from_slice(&height.to_be_bytes());
    buf.extend_from_slice(pixels);
    buf
}

fn decode_image(data: &[u8]) -> Option<(u16, u16, &[u8])> {
    if data.len() < 4 {
        return None;
    }
    let width = u16::from_be_bytes([data[0], data[1]]);
    let height = u16::from_be_bytes([data[2], data[3]]);
    let expected = width as usize * height as usize * 3;
    if data.len() != 4 + expected {
        return None;
    }
    Some((width, height, &data[4..]))
}

/// Convert a raw image byte buffer into ASCII art.
///
/// # Arguments
/// * `image_bytes`: raw RGB8 bytes (width * height * 3 bytes)
/// * `src_w`: original image width in pixels
/// * `src_h`: original image height in pixels
/// * `dst_w`: desired output characters per line
/// * `dst_h`: desired output lines
///
/// # Returns
/// A `vec::Vec<u8>` containing UTF-8 ASCII characters (including newline) representing the image.
fn image_to_ascii(
    image_bytes: &[u8],
    src_w: u32,
    src_h: u32,
    dst_w: u32,
    dst_h: u32,
) -> vec::Vec<u8> {
    // Convert to grayscale
    let mut gray = vec::Vec::with_capacity((src_w * src_h) as usize);
    for chunk in image_bytes.chunks_exact(3) {
        let lum = ((chunk[0] as u32 * 30 + chunk[1] as u32 * 59 + chunk[2] as u32 * 11) / 100) as u8;
        gray.push(lum);
    }

    // Nearest-neighbor resize to dst_w x dst_h
    let mut resized = vec::Vec::with_capacity((dst_w * dst_h) as usize);
    for y in 0..dst_h {
        let src_y = y * src_h / dst_h;
        for x in 0..dst_w {
            let src_x = x * src_w / dst_w;
            let i = (src_y * src_w + src_x) as usize;
            resized.push(gray[i]);
        }
    }

    // Map brightness to ASCII gradient
    let gradient = b"@%#*+=- . ";
    let mut ascii = vec::Vec::with_capacity((dst_w * dst_h + dst_h) as usize);
    for y in 0..dst_h {
        for x in 0..dst_w {
            let lum = resized[(y * dst_w + x) as usize];
            let idx = (lum as usize * (gradient.len() - 1)) / 255;
            ascii.push(gradient[idx]);
        }
         ascii.push(b'\n');
    }
    ascii
}

fn bytes_to_hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}
static mut pic: [u8; 2764800] = [0u8; 2764800];
#[polkavm_derive::polkavm_export]
extern "C" fn refine(start_address: u64, length: u64) -> (u64, u64) {
    call_log(1, None, &format!("refine: start_address: {:x}, length: {}", start_address, length));
    // todo : use payload to change the algorithm behavior
    let ptr = unsafe { pic.as_ptr() as u64 };
    call_log(1, None, &format!("refine: start_address: {:x}, length: {}", start_address, length));
    let fetch_result = unsafe { fetch(ptr, 0, 2764800, 30, 0, 0) };
    if fetch_result == NONE {
        call_log(2, None, "refine: fetch failed");
        return (FIRST_READABLE_ADDRESS as u64, 0);
    }
    // Assume fetch_result points to an RGB8 image buffer of known size (e.g., 128x128)
    // For demonstration, let's use 128x128 as source dimensions
    let src_w = 1280;
    let src_h = 720;
    let dst_w = 128;
    let dst_h = 72;

    // Safety: fetch_result is a pointer to the image buffer
    let image_bytes = unsafe { core::slice::from_raw_parts(ptr as *const u8, src_w * src_h * 3) };
    let ascii_art = image_to_ascii(image_bytes, src_w as u32, src_h as u32, dst_w as u32, dst_h as u32);
    call_log(1, None, &format!("fetch_result: {:x}, first bytes: {:?}", fetch_result, &image_bytes[..16]));
    // Optionally log or export the ASCII art
    call_log(1, None, &format!("refine: ASCII art generated, length: {}, expected: {}", ascii_art.len(), dst_w * dst_h + dst_h));
    call_log(10, None, &ascii_art.iter().map(|&b| b as char).collect::<String>());
    
    
    return (ascii_art.as_ptr() as u64, ascii_art.len() as u64);
}

#[polkavm_derive::polkavm_export]
extern "C" fn accumulate(start_address: u64, length: u64) -> (u64, u64) {
    return (FIRST_READABLE_ADDRESS as u64, 0);
}

#[polkavm_derive::polkavm_export]
extern "C" fn on_transfer(_start_address: u64, _length: u64) -> (u64, u64) {
    return (FIRST_READABLE_ADDRESS as u64, 0);
}