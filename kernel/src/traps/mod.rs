mod irq;
mod syndrome;
mod syscall;
mod trap_frame;

use pi::interrupt::{Controller, Interrupt};
use shell;

pub use self::trap_frame::TrapFrame;

use self::irq::handle_irq;
use self::syndrome::Syndrome;
use self::syscall::handle_syscall;
use aarch64;
use console::_print;

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn handle_exception(info: Info, esr: u32, tf: &mut TrapFrame) {
    kprintln!("info: {:?}", info);
    kprintln!("esr: {:x}", esr);
    kprintln!("tf: {:#?}", tf);

    match (info.kind, Syndrome::from(esr)) {
        (Kind::Synchronous, Syndrome::Brk(x)) => {
            shell::shell(&::FILE_SYSTEM, "?");
            tf.elr += 4;
        }
        (Kind::Synchronous, Syndrome::Svc(x)) => {
            let elapsed = handle_syscall(x, tf);
        }
        (Kind::Irq, _) => {
            let int = if Controller::new().is_pending(Interrupt::Timer1) {
                Interrupt::Timer1
            } else {
                panic!("unexpected interrupt");
            };
            handle_irq(int, tf);
        }
        (_, syndrome) => panic!("unexpected syndrome: {:?}", syndrome),
    }
}
