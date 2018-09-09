#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(const_fn)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(repr_align)]
#![feature(attr_literals)]
#![feature(never_type)]
#![feature(ptr_internals)]

extern crate pi;
extern crate stack_vec;

#[macro_use]
pub mod console;
pub mod lang_items;
pub mod mutex;
pub mod shell;

use console::_print;
use pi::timer;
use pi::uart::MiniUart;

#[no_mangle]
pub extern "C" fn kmain() {
    shell::shell("> ");
}
