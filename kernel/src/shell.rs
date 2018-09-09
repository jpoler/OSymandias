use console::{_print, CONSOLE};
use stack_vec::StackVec;
use std::io::{self, Read, Write};

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
    LineTooLong,
    Io { error: io::Error },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io { error }
    }
}

struct BufferedIo {}

impl BufferedIo {
    fn new() -> BufferedIo {
        BufferedIo {}
    }

    #[inline]
    fn push<'a>(stack: &mut StackVec<'a, u8>, b: u8) -> Result<(), Error> {
        stack.push(b).map_err(|_| Error::LineTooLong)
    }

    fn readline(&mut self, mut line: &mut [u8]) -> Result<usize, Error> {
        let mut line_stack = StackVec::new(&mut line);

        loop {
            {
                let mut console = CONSOLE.lock();
                let b = console.read_byte();
                match b {
                    b'\r' | b'\n' => {
                        console.write_byte(b'\r');
                        console.write_byte(b'\n');
                        return Ok(line_stack.len());
                    }
                    8 | 127 => {
                        if let Some(_) = line_stack.pop() {
                            console.write_byte(8);
                            console.write_byte(b' ');
                            console.write_byte(8);
                        }
                    }
                    32...126 => {
                        console.write_byte(b);
                        BufferedIo::push(&mut line_stack, b)?;
                    }
                    _ => {
                        console.write_byte(7);
                    }
                }
            }
        }
    }
}

/// A structure representing a single shell command.
struct Command<'a> {
    args: StackVec<'a, &'a str>,
}

impl<'a> Command<'a> {
    /// Parse a command from a string `s` using `buf` as storage for the
    /// arguments.
    ///
    /// # Errors
    ///
    /// If `s` contains no arguments, returns `Error::Empty`. If there are more
    /// arguments than `buf` can hold, returns `Error::TooManyArgs`.
    fn parse(s: &'a str, buf: &'a mut [&'a str]) -> Result<Command<'a>, Error> {
        let mut args = StackVec::new(buf);
        for arg in s.split(' ').filter(|a| !a.is_empty()) {
            args.push(arg).map_err(|_| Error::TooManyArgs)?;
        }

        if args.is_empty() {
            return Err(Error::Empty);
        }

        Ok(Command { args })
    }

    /// Returns this command's path. This is equivalent to the first argument.
    fn path(&self) -> Option<&str> {
        if self.args.len() > 1 {
            Some(self.args.as_slice()[0])
        } else {
            None
        }
    }
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns: it is perpetually in a shell loop.
pub fn shell(prefix: &str) -> ! {
    let mut bufio = BufferedIo::new();
    let mut line_buf: [u8; 512] = [0; 512];

    loop {
        // read a line
        kprint!("\n{}", prefix);
        match bufio.readline(&mut line_buf) {
            Ok(n) => kprintln!("line of length: {}", n),
            Err(err) => kprintln!("\nerror: {:?}", err),
        }

        // validate command
        // execute command
    }
}
