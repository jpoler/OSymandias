#[feature(asm)]
use core::marker::PhantomData;

use common::{states, IO_BASE};
use volatile::prelude::*;
use volatile::{ReadVolatile, Reserved, Volatile, WriteVolatile};

/// An alternative GPIO function.
#[repr(u8)]
pub enum Function {
    Input = 0b000,
    Output = 0b001,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
    Alt4 = 0b011,
    Alt5 = 0b010,
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Debug)]
struct Registers {
    FSEL: [Volatile<u32>; 6],
    __r0: Reserved<u32>,
    SET: [WriteVolatile<u32>; 2],
    __r1: Reserved<u32>,
    CLR: [WriteVolatile<u32>; 2],
    __r2: Reserved<u32>,
    LEV: [ReadVolatile<u32>; 2],
    __r3: Reserved<u32>,
    EDS: [Volatile<u32>; 2],
    __r4: Reserved<u32>,
    REN: [Volatile<u32>; 2],
    __r5: Reserved<u32>,
    FEN: [Volatile<u32>; 2],
    __r6: Reserved<u32>,
    HEN: [Volatile<u32>; 2],
    __r7: Reserved<u32>,
    LEN: [Volatile<u32>; 2],
    __r8: Reserved<u32>,
    AREN: [Volatile<u32>; 2],
    __r9: Reserved<u32>,
    AFEN: [Volatile<u32>; 2],
    __r10: Reserved<u32>,
    PUD: Volatile<u32>,
    PUDCLK: [Volatile<u32>; 2],
}

/// Possible states for a GPIO pin.
states! {
    Uninitialized, Input, Output, Alt
}

/// A GPIP pin in state `State`.
///
/// The `State` generic always corresponds to an uninstantiatable type that is
/// use solely to mark and track the state of a given GPIO pin. A `Gpio`
/// structure starts in the `Uninitialized` state and must be transitions into
/// one of `Input`, `Output`, or `Alt` via the `into_input`, `into_output`, and
/// `into_alt` methods before it can be used.
pub struct Gpio<State> {
    pin: u8,
    registers: &'static mut Registers,
    _state: PhantomData<State>,
}

/// The base address of the `GPIO` registers.
const GPIO_BASE: usize = IO_BASE + 0x200000;

impl<T> Gpio<T> {
    /// Transitions `self` to state `S`, consuming `self` and returning a new
    /// `Gpio` instance in state `S`. This method should _never_ be exposed to
    /// the public!
    #[inline(always)]
    fn transition<S>(self) -> Gpio<S> {
        Gpio {
            pin: self.pin,
            registers: self.registers,
            _state: PhantomData,
        }
    }

    #[inline]
    fn pin_index(&self) -> usize {
        (self.pin as usize) / 32
    }

    #[inline]
    fn pin_mask(&self) -> u32 {
        1 << ((self.pin as u32) % 32)
    }
}

impl Gpio<Uninitialized> {
    /// Returns a new `GPIO` structure for pin number `pin`.
    ///
    /// # Panics
    ///
    /// Panics if `pin` > `53`.
    pub fn new(pin: u8) -> Gpio<Uninitialized> {
        if pin > 53 {
            panic!("Gpio::new(): pin {} exceeds maximum of 53", pin);
        }

        Gpio {
            registers: unsafe { &mut *(GPIO_BASE as *mut Registers) },
            pin: pin,
            _state: PhantomData,
        }
    }

    pub fn new_test(stack_ptr: *mut u32, pin: u8) -> Gpio<Uninitialized> {
        if pin > 53 {
            panic!("Gpio::new(): pin {} exceeds maximum of 53", pin);
        }

        Gpio {
            registers: unsafe { &mut *(stack_ptr as *mut Registers) },
            pin: pin,
            _state: PhantomData,
        }
    }

    #[inline]
    fn fsel_reg_index(&self) -> usize {
        (self.pin as usize) / 10
    }

    #[inline]
    fn fsel_reg_shift(&self) -> u32 {
        ((self.pin as u32) % 10) * 3
    }

    #[inline]
    fn fsel_set_function_bits(reg: u32, bits: u32, shift: u32) -> u32 {
        let mask: u32 = !(0b111 << shift);
        (reg & mask) | (bits << shift)
    }

    fn write_fsel(&mut self, function: Function) {
        let index = self.fsel_reg_index();
        let shift = self.fsel_reg_shift();

        let val = self.registers.FSEL[index].read();
        let val = Gpio::fsel_set_function_bits(val, function as u32, shift);

        self.registers.FSEL[index].write(val);
    }

    /// Enables the alternative function `function` for `self`. Consumes self
    /// and returns a `Gpio` structure in the `Alt` state.
    pub fn into_alt(mut self, function: Function) -> Gpio<Alt> {
        self.write_fsel(function);

        Gpio {
            pin: self.pin,
            registers: self.registers,
            _state: PhantomData,
        }
    }

    /// Sets this pin to be an _output_ pin. Consumes self and returns a `Gpio`
    /// structure in the `Output` state.
    pub fn into_output(self) -> Gpio<Output> {
        self.into_alt(Function::Output).transition()
    }

    /// Sets this pin to be an _input_ pin. Consumes self and returns a `Gpio`
    /// structure in the `Input` state.
    pub fn into_input(self) -> Gpio<Input> {
        self.into_alt(Function::Input).transition()
    }
}

impl Gpio<Output> {
    pub fn set(&mut self) {
        let index = self.pin_index();
        let mask = self.pin_mask();

        self.registers.SET[index].write(mask);
    }

    pub fn clear(&mut self) {
        let index = self.pin_index();
        let mask = self.pin_mask();

        self.registers.CLR[index].write(mask);
    }
}

impl Gpio<Input> {
    /// Reads the pin's value. Returns `true` if the level is high and `false`
    /// if the level is low.
    pub fn level(&mut self) -> bool {
        let index = self.pin_index();
        let mask = self.pin_mask();

        (self.registers.LEV[index].read() & mask) == 0
    }
}

impl Gpio<Alt> {
    pub fn disable_pull_up_down(&mut self) {
        self.registers.PUD.write(0);
        for _ in 0..150 {
            unsafe { asm!("NOP") }
        }
        let pin_mask = self.pin_mask();
        self.registers.PUDCLK[0].or_mask(pin_mask);
        for _ in 0..150 {
            unsafe { asm!("NOP") }
        }
        self.registers.PUDCLK[0].write(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity_check() {
        let mut registers: [u32; 41] = [0; 41];
        let ptr: *mut u32 = &mut registers[0] as *mut u32;

        let mut gpio = Gpio::new_test(ptr, 16).into_output();
        assert_eq!(registers[1], 0x00040000);

        gpio.set();
        assert_eq!(registers[7], 0x00010000);

        gpio.clear();
        assert_eq!(registers[10], 0x00010000);
    }
}
