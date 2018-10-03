#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(const_fn)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(repr_align)]
#![feature(attr_literals)]
#![feature(exclusive_range_pattern)]
#![feature(i128_type)]
#![feature(never_type)]
#![feature(unique)]
#![feature(pointer_methods)]
#![feature(naked_functions)]
#![feature(fn_must_use)]
#![feature(alloc, allocator_api, global_allocator)]
#![feature(pointer_methods)]

#[macro_use]
#[allow(unused_imports)]
extern crate alloc;
extern crate fat32;
extern crate pi;
extern crate stack_vec;

#[macro_use]
pub mod console;

pub mod aarch64;
pub mod allocator;
pub mod fs;
pub mod lang_items;
pub mod mutex;
pub mod process;
pub mod shell;
pub mod traps;
pub mod vm;

use console::{_print, CONSOLE};
use pi::atags::Atags;

#[cfg(not(test))]
use allocator::Allocator;
use fat32::traits::{Dir as DirTrait, Entry as EntryTrait, FileSystem as FileSystemTrait};
use fat32::MasterBootRecord;
use fs::sd::Sd;
use fs::FileSystem;
use pi::timer;
use process::GlobalScheduler;

#[cfg(not(test))]
#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();

pub static FILE_SYSTEM: FileSystem = FileSystem::uninitialized();

pub static SCHEDULER: GlobalScheduler = GlobalScheduler::uninitialized();

// TODO: enable data cache with sctlr
//       must invalidate cache before enabling
#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn kmain() {
    timer::spin_sleep_ms(1000);
    ALLOCATOR.initialize();
    FILE_SYSTEM.initialize();

    kprintln!("{:x}", unsafe { aarch64::current_el() });
    kprintln!("{:x}", aarch64::sctlr());

    shell::shell(&FILE_SYSTEM, ">");
}
