use common::IO_BASE;
use volatile::prelude::*;
use volatile::{ReadVolatile, Volatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IRQ_BASIC_PENDING: ReadVolatile<u32>,
    IRQ_PENDING: [ReadVolatile<u32>; 2],
    FIQ_CONTROL: Volatile<u32>,
    ENABLE_IRQS: [Volatile<u32>; 2],
    ENABLE_BASIC_IRQS: Volatile<u32>,
    DISABLE_IRQS: [Volatile<u32>; 2],
    DISABLE_BASIC_IRQS: Volatile<u32>,
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers,
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    fn index_and_offset(&self, irqno: u8) -> (usize, usize) {
        let irqno = irqno as usize;
        (irqno / 32, irqno % 32)
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
        let (index, offset) = self.index_and_offset(int as u8);
        self.registers.ENABLE_IRQS[index].write(1 << offset);
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
        let (index, offset) = self.index_and_offset(int as u8);
        self.registers.DISABLE_IRQS[index].write(1 << offset);
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
        let (index, offset) = self.index_and_offset(int as u8);
        (self.registers.IRQ_PENDING[index].read() & (1 << offset)) != 0
    }
}
