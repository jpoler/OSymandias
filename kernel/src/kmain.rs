#![feature(lang_items)]
#![feature(core_intrinsics)]
#![feature(const_fn)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(repr_align)]
#![feature(attr_literals)]
#![feature(exclusive_range_pattern)]
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

pub mod allocator;
pub mod fs;
pub mod lang_items;
pub mod mutex;
pub mod shell;

use console::{_print, CONSOLE};
use pi::atags::Atags;

#[cfg(not(test))]
use allocator::Allocator;
use fat32::traits::{Dir as DirTraitEntry, Entry as EntryTrait, FileSystem as FileSystemTrait};
use fat32::MasterBootRecord;
use fs::sd::Sd;
use fs::FileSystem;
use pi::timer;

#[cfg(not(test))]
#[global_allocator]
pub static ALLOCATOR: Allocator = Allocator::uninitialized();

pub static FILE_SYSTEM: FileSystem = FileSystem::uninitialized();

#[no_mangle]
#[cfg(not(test))]
pub extern "C" fn kmain() {
    timer::spin_sleep_ms(1000);
    ALLOCATOR.initialize();
    FILE_SYSTEM.initialize();

    shell::shell(&FILE_SYSTEM, "> ");
}
