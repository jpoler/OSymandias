#[derive(Debug)]
pub struct Error {}

impl From<u64> for Error {
    fn from(code: u64) -> Self {
        Error {}
    }
}

pub fn sleep(ms: u32) -> Result<u32, Error> {
    let error: u64;
    let elapsed: u64;
    unsafe {
        asm!("mov x0, $2
              svc 1
              mov $0, x0
              mov $1, x7"
             : "=r"(elapsed), "=r"(error)
             : "r"(ms)
             : "x0", "x7")
    }
    if error == 0 {
        Ok(elapsed as u32)
    } else {
        Err(Error::from(error))
    }
}
