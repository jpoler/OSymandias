use console::_print;
use fat32::traits::BlockDevice;
use pi::timer;
use std::i32;
use std::io;

extern "C" {
    /// A global representing the last SD controller error that occured.
    static sd_err: i64;

    /// Initializes the SD card controller.
    ///
    /// Returns 0 if initialization is successful. If initialization fails,
    /// returns -1 if a timeout occured, or -2 if an error sending commands to
    /// the SD controller occured.
    fn sd_init() -> i32;

    /// Reads sector `n` (512 bytes) from the SD card and writes it to `buffer`.
    /// It is undefined behavior if `buffer` does not point to at least 512
    /// bytes of memory.
    ///
    /// On success, returns the number of bytes read: a positive number.
    ///
    /// On error, returns 0. The true error code is stored in the `sd_err`
    /// global. `sd_err` will be set to -1 if a timeout occured or -2 if an
    /// error sending commands to the SD controller occured. Other error codes
    /// are also possible but defined only as being less than zero.
    fn sd_readsector(n: i32, buffer: *mut u8) -> i32;
}

#[no_mangle]
pub fn wait_micros(us: u32) {
    timer::spin_sleep_us((100 * us).into())
}

#[derive(Debug)]
pub enum Error {
    Timeout,
    Control,
    Unknown(i32),
}

impl Error {
    fn from(n: i32) -> Option<Error> {
        match n {
            n if n >= 0 => None,
            -1 => Some(Error::Timeout),
            -2 => Some(Error::Control),
            n => Some(Error::Unknown(n)),
        }
    }
}

/// A handle to an SD card controller.
#[derive(Debug)]
pub struct Sd;

impl Sd {
    /// Initializes the SD card controller and returns a handle to it.
    pub fn new() -> Result<Sd, Error> {
        if let Some(err) = unsafe { Error::from(sd_init()) } {
            Err(err)
        } else {
            Ok(Sd)
        }
    }
}

impl BlockDevice for Sd {
    /// Reads sector `n` from the SD card into `buf`. On success, the number of
    /// bytes read is returned.
    ///
    /// # Errors
    ///
    /// An I/O error of kind `InvalidInput` is returned if `buf.len() < 512` or
    /// `n > 2^31 - 1` (the maximum value for an `i32`).
    ///
    /// An error of kind `TimedOut` is returned if a timeout occurs while
    /// reading from the SD card.
    ///
    /// An error of kind `Other` is returned for all other errors.
    fn read_sector(&mut self, sector: u64, buf: &mut [u8]) -> io::Result<usize> {
        if sector > i32::MAX as u64 || buf.len() < 512 {
            return Err(io::ErrorKind::InvalidInput.into());
        }

        let n = unsafe { sd_readsector(sector as i32, buf.as_mut_ptr()) };
        kprintln!("sector: {}, n: {}", sector, n);
        if n == 0 {
            let code = unsafe { sd_err };
            match Error::from(code as i32) {
                None => unreachable!("error code must be less than 0"),
                Some(Error::Timeout) => Err(io::ErrorKind::TimedOut.into()),
                Some(Error::Control) => Err(io::Error::new(io::ErrorKind::Other, "control error")),
                Some(Error::Unknown(n)) => Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("unknown error code: {}", n),
                )),
            }
        } else {
            Ok(n as usize)
        }
    }

    fn write_sector(&mut self, _n: u64, _buf: &[u8]) -> io::Result<usize> {
        unimplemented!("SD card and file system are read only")
    }
}
