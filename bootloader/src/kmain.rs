#![feature(asm, lang_items)]

extern crate pi;
extern crate xmodem;

use pi::timer;
use pi::uart::MiniUart;
use std::fmt::Write;
use std::io::ErrorKind;
use std::slice;
use xmodem::{Xmodem, DEBUG_BUFFER, DEBUG_BUFFER_OFFSET};

pub mod lang_items;

/// Start address of the binary to load and of the bootloader.
const BINARY_START_ADDR: usize = 0x80000;
const BOOTLOADER_START_ADDR: usize = 0x4000000;

/// Pointer to where the loaded binary expects to be laoded.
const BINARY_START: *mut u8 = BINARY_START_ADDR as *mut u8;

/// Free space between the bootloader and the loaded binary's start address.
const MAX_BINARY_SIZE: usize = BOOTLOADER_START_ADDR - BINARY_START_ADDR;

/// Branches to the address `addr` unconditionally.
fn jump_to(addr: *mut u8) -> ! {
    unsafe {
        asm!("br $0" : : "r"(addr as usize));
        loop {
            asm!("nop" :::: "volatile")
        }
    }
}

#[no_mangle]
pub extern "C" fn kmain() {
    let err;
    loop {
        let mut uart = MiniUart::new();
        uart.read_byte();
        let mut binary = unsafe { slice::from_raw_parts_mut(BINARY_START, MAX_BINARY_SIZE) };
        let n = match Xmodem::receive(uart, &mut binary) {
            Ok(n) => n,
            Err(error) => match error.kind() {
                ErrorKind::TimedOut => continue,
                _ => {
                    err = error;
                    break;
                }
            },
        };

        if n > 0 && n < MAX_BINARY_SIZE {
            jump_to(BINARY_START);
        }
    }

    let mut uart = MiniUart::new();
    loop {
        timer::spin_sleep_ms(1000);
        write!(&mut uart, "{}\n", err).unwrap();
        unsafe {
            write!(&mut uart, "{:?}", &DEBUG_BUFFER[..DEBUG_BUFFER_OFFSET]).unwrap();
        }
    }
}
