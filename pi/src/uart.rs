use core::fmt;

use volatile::prelude::*;
use volatile::{ReadVolatile, Volatile};

use common::IO_BASE;
use gpio::{Function, Gpio};
use timer;

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

const AUX_ENABLES: usize = IO_BASE + 0x215004;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u32)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
    TxIdle = 1 << 6,
}

#[repr(u32)]
enum LcrSettings {
    DataSize8Bit = 0b11,
    DlabAccess = 1 << 7,
}

#[repr(u32)]
enum CntlSettings {
    EnableRx = 1,
    EnableTx = 1 << 1,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IO: Volatile<u32>,
    IER: Volatile<u32>,
    IIR: Volatile<u32>,
    LCR: Volatile<u32>,
    MCR: Volatile<u32>,
    LSR: ReadVolatile<u32>,
    MSR: ReadVolatile<u32>,
    SCRATCH: Volatile<u32>,
    CNTL: Volatile<u32>,
    STAT: ReadVolatile<u32>,
    BAUD: Volatile<u32>,
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<u32>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new() -> MiniUart {
        Gpio::new(14).into_alt(Function::Alt5);
        Gpio::new(15).into_alt(Function::Alt5);
        MiniUart::new_inner(AUX_ENABLES, MU_REG_BASE)
    }

    pub fn new_test(stack_ptr: &mut [u32; 53]) -> MiniUart {
        let gpio_ptr = &mut stack_ptr[0] as *mut u32;
        let aux_enable_ptr = (&mut stack_ptr[41] as *mut u32) as usize;
        let uart_ptr = (&mut stack_ptr[42] as *mut u32) as usize;

        Gpio::new_test(gpio_ptr, 14).into_alt(Function::Alt5);
        Gpio::new_test(gpio_ptr, 15).into_alt(Function::Alt5);

        MiniUart::new_inner(aux_enable_ptr, uart_ptr)
    }

    fn new_inner(aux_enables: usize, registers_ptr: usize) -> MiniUart {
        // The `AUXENB` register from page 9 of the BCM2837 documentation.
        let aux_enables: *mut Volatile<u8> = aux_enables as *mut Volatile<u8>;

        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*aux_enables).or_mask(1);
            &mut *(registers_ptr as *mut Registers)
        };

        // 8 bit mode
        registers.LCR.or_mask(LcrSettings::DataSize8Bit as u32);

        // enable access to the baudrate register
        registers.LCR.or_mask(LcrSettings::DlabAccess as u32);

        // TODO if you are getting garbage out of the UART check here first
        // set baud rate to 250e6 / 8*(270+1) ~= 115200
        registers.BAUD.write(270);

        // disable access to the baudrate registers, which must be cleared
        // during operation
        registers.LCR.and_mask(!(LcrSettings::DlabAccess as u32));

        // enable tx and rx
        registers
            .CNTL
            .or_mask((CntlSettings::EnableRx as u32) | (CntlSettings::EnableTx as u32));

        MiniUart {
            registers,
            timeout: None,
        }
    }

    /// Set the read timeout to `milliseconds` milliseconds.
    pub fn set_read_timeout(&mut self, milliseconds: u32) {
        self.timeout = Some(milliseconds);
    }

    #[inline]
    fn write_fifo_full(&self) -> bool {
        self.registers.LSR.read() & (LsrStatus::TxAvailable as u32) != 0
    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while self.write_fifo_full() {
            timer::spin_sleep_us(10);
        }
        self.registers.IO.write(byte as u32);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        (self.registers.LSR.read() & (LsrStatus::DataReady as u32)) != 0
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        if let Some(milliseconds) = self.timeout {
            let microseconds = (milliseconds * 1000) as u64;
            let deadline = timer::current_time() + microseconds;
            while deadline - timer::current_time() > 0 {
                if self.has_byte() {
                    return Ok(());
                }

                // justification for this number:
                // >>> bps = 115200 / 10.0
                // >>> 1 / bps = 8.680555555555556e-05
                //
                // Where 10 is start bit + stop bit + 8 bits in a byte, and
                // 115200 is the baud rate. This means we could get a byte every
                // 8.7 microseconds
                timer::spin_sleep_us(10);
            }
        } else {
            loop {
                if self.has_byte() {
                    return Ok(());
                }

                timer::spin_sleep_us(10);
            }
        }

        Err(())
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        self.registers.IO.read() as u8
    }
}

// A b'\r' byte should be written before writing any b'\n' byte.
impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            match b {
                b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                }
                _ => self.write_byte(b),
            }
        }

        Ok(())
    }
}

#[cfg(feature = "std")]
mod uart_io {
    use super::{LsrStatus, MiniUart};
    use std::io;
    use timer;
    use volatile::Readable;

    // The `io::Read::read()` implementation must respect the read timeout by
    // waiting at most that time for the _first byte_. It should not wait for
    // any additional bytes but _should_ read as many bytes as possible. If the
    // read times out, an error of kind `TimedOut` should be returned.
    impl io::Read for MiniUart {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let mut n = 0;
            if let Err(_) = self.wait_for_byte() {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "uart: read timeout",
                ));
            }

            while n < buf.len() && self.has_byte() {
                buf[n] = self.read_byte();
                n += 1;
            }

            Ok(n)
        }
    }

    // The `io::Write::write()` method must write all of the requested bytes
    // before returning.
    impl io::Write for MiniUart {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            for b in buf {
                self.write_byte(*b);
            }

            Ok(buf.len())
        }

        // TODO figure out if flush should timeout
        fn flush(&mut self) -> io::Result<()> {
            while (self.registers.LSR.read() & (LsrStatus::TxIdle as u32)) != 0 {
                timer::spin_sleep_us(10);
            }

            Ok(())
        }
    }
}

#[cfg(feature = "std")]
#[cfg(test)]
mod tests {
    use super::*;

    struct TestStack {
        stack: [u32; 53],
    }

    impl TestStack {
        fn new() -> TestStack {
            TestStack { stack: [0; 53] }
        }
    }

    impl fmt::Debug for TestStack {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "gpio registers:\n");
            for (i, reg) in self.stack[..41].iter().enumerate() {
                write!(f, "\t[{}]\t{:#010x}\n", i, reg);
            }

            write!(f, "\naux enable register:\n");
            write!(f, "\t\t{:#010x}\n", self.stack[41]);

            write!(f, "\nuart registers: \n");
            for (i, reg) in self.stack[42..].iter().enumerate() {
                write!(f, "\t[{}]\t{:#010x}\n", i, reg);
            }
            Ok(())
        }
    }

    #[test]
    fn sanity_check() {
        let mut stack_space = TestStack::new();
        let mut uart = MiniUart::new_test(&mut stack_space.stack);

        println!("{:?}", stack_space);
    }
}
