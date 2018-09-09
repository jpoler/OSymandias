use self::builtins::resolve_builtin;
use console::{_print, CONSOLE};
use stack_vec::StackVec;
use std::fmt;
use std::io;
use std::str::from_utf8;

mod builtins {
    use console::_print;

    pub fn resolve_builtin(cmd: &str) -> Option<fn(&mut [&str])> {
        match cmd {
            "echo" => Some(echo),
            _ => None,
        }
    }

    fn echo(args: &mut [&str]) {
        let args = &args[1..];
        if args.len() == 0 {
            kprintln!("");
        } else {
            for arg in &args[..args.len() - 1] {
                kprint!("{} ", arg);
            }
            kprintln!("{}", args[args.len() - 1])
        }
    }
}

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
    LineTooLong,
    UnknownCommand,
    InvalidUtf8,
    Io { error: io::Error },
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io { error }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            &Empty => write!(f, "empty command"),
            &TooManyArgs => write!(f, "too many arguments"),
            &LineTooLong => write!(f, "line too long"),
            // TODO this should have the command name but would require heap
            // allocation
            &UnknownCommand => write!(f, "unknown command"),
            &InvalidUtf8 => write!(f, "invalid utf8"),
            &Io { ref error } => write!(f, "{}", error),
        }
    }
}

struct BufferedIo;

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
    fn path(&self) -> &str {
        self.args[0]
    }
}

fn eval(prefix: &str) -> Result<(), Error> {
    let mut bufio = BufferedIo::new();
    let mut line: [u8; 512] = [0; 512];
    let mut args: [&str; 64] = [""; 64];

    kprint!("{}", prefix);

    let n = match bufio.readline(&mut line)? {
        0 => return Ok(()),
        n => n,
    };

    let cmd_str = from_utf8(&mut line[..n]).map_err(|_| Error::InvalidUtf8)?;
    let mut cmd = Command::parse(cmd_str, &mut args)?;

    let f = resolve_builtin(cmd.path()).ok_or(Error::UnknownCommand)?;

    f(&mut cmd.args[..]);

    Ok(())
}

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns: it is perpetually in a shell loop.
pub fn shell(prefix: &str) -> ! {
    loop {
        if let Err(err) = eval(prefix) {
            kprintln!("\nerror: {}", err)
        }
    }
}
