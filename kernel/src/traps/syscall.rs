use process::Process;
use process::State;
use timer::Timer;
use traps::TrapFrame;
use SCHEDULER;

/// Sleep for `ms` milliseconds.
///
/// This system call takes one parameter: the number of milliseconds to sleep.
///
/// In addition to the usual status value, this system call returns one
/// parameter: the approximate true elapsed time from when `sleep` was called to
/// when `sleep` returned.
pub fn sleep(ms: u32, tf: &mut TrapFrame) {
    let us = (ms as u64) * 1000;
    let timer = Timer::new();
    let start_time = timer.read();

    let ready: Box<FnMut(&mut Process) -> bool + Send> = Box::new(move |process| {
        let timer = Timer::new();
        let elapsed = timer.read() - start_time;
        if elapsed > us {
            process.trap_frame.x0 = elapsed / 1000;
            process.trap_frame.x7 = 0;
            true
        } else {
            false
        }
    });
    SCHEDULER.switch(State::Waiting(ready), tf);
}

pub fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    match num {
        1 => {
            let duration = tf.x0 as u32;
            sleep(duration, tf);
        }
        _ => unimplemented!("unknown syscall: {}", num),
    }
}
