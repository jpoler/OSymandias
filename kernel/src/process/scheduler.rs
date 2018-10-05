use std::collections::VecDeque;

use mutex::Mutex;
use process::{Id, Process, State};
use shell;
use traps::TrapFrame;
use FILE_SYSTEM;

/// The `tick` time.
// FIXME: When you're ready, change this to something more reasonable.
pub const TICK: u32 = 2 * 1000 * 1000;

/// Process scheduler for the entire machine.
#[derive(Debug)]
pub struct GlobalScheduler(Mutex<Option<Scheduler>>);

extern "C" fn start_shell() {
    unsafe {
        asm!("brk 1" :::: "volatile");
    }
    unsafe {
        asm!("brk 2" :::: "volatile");
    }
    shell::shell(&FILE_SYSTEM, "user0> ");
    unsafe {
        asm!("brk 3" :::: "volatile");
    }
    loop {
        shell::shell(&FILE_SYSTEM, "user1> ");
    }
}

impl GlobalScheduler {
    /// Returns an uninitialized wrapper around a local scheduler.
    pub const fn uninitialized() -> GlobalScheduler {
        GlobalScheduler(Mutex::new(None))
    }

    /// Adds a process to the scheduler's queue and returns that process's ID.
    /// For more details, see the documentation on `Scheduler::add()`.
    pub fn add(&self, process: Process) -> Option<Id> {
        self.0
            .lock()
            .as_mut()
            .expect("scheduler uninitialized")
            .add(process)
    }

    /// Performs a context switch using `tf` by setting the state of the current
    /// process to `new_state`, saving `tf` into the current process, and
    /// restoring the next process's trap frame into `tf`. For more details, see
    /// the documentation on `Scheduler::switch()`.
    #[must_use]
    pub fn switch(&self, new_state: State, tf: &mut TrapFrame) -> Option<Id> {
        self.0
            .lock()
            .as_mut()
            .expect("scheduler uninitialized")
            .switch(new_state, tf)
    }

    /// Initializes the scheduler and starts executing processes in user space
    /// using timer interrupt based preemptive scheduling. This method should
    /// not return under normal conditions.
    pub fn start(&self) {
        let mut process = Process::new().expect("first process");
        let tf = &mut *(process.trap_frame);
        tf.sp = process.stack.top().as_u64();
        tf.elr = start_shell as *const fn() as u64;
        // spsr is already in the proper state when zeroed
        unsafe {
            asm!("mov x0, $0
                  bl  context_restore
                  ldr x0, =_start
                  add sp, x0, #0
                  mov x0, #0
                  eret"
                 :: "r"(tf)
                 :: "volatile");
        }
    }
}

#[derive(Debug)]
struct Scheduler {
    processes: VecDeque<Process>,
    current: Option<Id>,
    last_id: Option<Id>,
}

impl Scheduler {
    /// Returns a new `Scheduler` with an empty queue.
    fn new() -> Scheduler {
        unimplemented!("Scheduler::new()")
    }

    /// Adds a process to the scheduler's queue and returns that process's ID if
    /// a new process can be scheduled. The process ID is newly allocated for
    /// the process and saved in its `trap_frame`. If no further processes can
    /// be scheduled, returns `None`.
    ///
    /// If this is the first process added, it is marked as the current process.
    /// It is the caller's responsibility to ensure that the first time `switch`
    /// is called, that process is executing on the CPU.
    fn add(&mut self, mut process: Process) -> Option<Id> {
        unimplemented!("Scheduler::add()")
    }

    /// Sets the current process's state to `new_state`, finds the next process
    /// to switch to, and performs the context switch on `tf` by saving `tf`
    /// into the current process and restoring the next process's trap frame
    /// into `tf`. If there is no current process, returns `None`. Otherwise,
    /// returns `Some` of the process ID that was context switched into `tf`.
    ///
    /// This method blocks until there is a process to switch to, conserving
    /// energy as much as possible in the interim.
    fn switch(&mut self, new_state: State, tf: &mut TrapFrame) -> Option<Id> {
        unimplemented!("Scheduler::switch()")
    }
}
