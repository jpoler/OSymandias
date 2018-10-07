use pi::interrupt::Interrupt;

use process::TICK;
use timer::tick_in;
use traps::TrapFrame;

pub fn handle_irq(interrupt: Interrupt, tf: &mut TrapFrame) {
    match interrupt {
        Interrupt::Timer1 => tick_in(TICK),
        _ => panic!("unexpected interrupt: {:?}", interrupt),
    }
}
