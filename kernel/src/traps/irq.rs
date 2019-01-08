use pi::interrupt::Interrupt;

use process::{State, TICK};
use timer::tick_in;
use traps::TrapFrame;
use SCHEDULER;

pub fn handle_irq(interrupt: Interrupt, tf: &mut TrapFrame) {
    match interrupt {
        Interrupt::Timer1 => {
            SCHEDULER.switch(State::Ready, tf);
            tick_in(TICK)
        }
        _ => panic!("unexpected interrupt: {:?}", interrupt),
    }
}
