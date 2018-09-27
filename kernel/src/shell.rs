use console::{_print, CONSOLE};
use fat32::traits::{
    Dir as DirTrait, Entry as EntryTrait, FileSystem as FileSystemTrait, Metadata as MetadataTrait,
};
use fs::FileSystem;
use stack_vec::StackVec;
use std::fmt;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::{Component, Path, PathBuf};
use std::str::from_utf8;

trait CanonicalJoin
where
    Self: Sized,
{
    fn canonical_join(&self, other: &Self) -> Result<Self, Error>;
}

impl CanonicalJoin for PathBuf {
    fn canonical_join(&self, other: &Self) -> Result<Self, Error> {
        let mut pathbuf = self.clone();

        for component in Path::new(other).components() {
            match component {
                Component::Prefix(prefix) => {
                    return Err(Error::Path {
                        path: PathBuf::from(other),
                        message: format!("prefix not supported: {:?}", prefix),
                    })
                }
                Component::RootDir => {
                    pathbuf.reset();
                }
                Component::ParentDir => {
                    if let None = pathbuf.parent() {
                        return Err(Error::Path {
                            path: PathBuf::from(other),
                            message: "invalid target directory".into(),
                        });
                    }

                    pathbuf.pop();
                }
                Component::CurDir => {}
                Component::Normal(normal) => {
                    pathbuf.push(normal);
                }
            }
        }

        Ok(pathbuf)
    }
}

trait Reset {
    fn reset(&mut self) {}
}

impl Reset for PathBuf {
    fn reset(&mut self) {
        while self.pop() {}
    }
}

struct Shell<'a> {
    prefix: &'a str,
    cwd: PathBuf,
    fs: &'static FileSystem,
}

impl<'a> Shell<'a> {
    pub fn new(fs: &'static FileSystem, prefix: &'a str) -> Shell<'a> {
        Shell {
            prefix,
            cwd: PathBuf::from("/"),
            fs,
        }
    }

    fn repl(mut self) -> ! {
        loop {
            kprint!("[{}] {} ", self.cwd.display(), self.prefix);
            if let Err(err) = self.eval() {
                kprintln!("{}", err)
            }
        }
    }

    fn eval(&mut self) -> Result<(), Error> {
        let mut bufio = BufferedIo::new();
        let mut line: [u8; 512] = [0; 512];
        let mut args: [&str; 64] = [""; 64];

        let n = match bufio.readline(&mut line)? {
            0 => return Ok(()),
            n => n,
        };

        let cmd_str = from_utf8(&mut line[..n]).map_err(|_| Error::InvalidUtf8)?;
        let cmd = Command::parse(cmd_str, &mut args)?;

        self.dispatch(&cmd)
    }

    pub fn dispatch(&mut self, command: &Command) -> Result<(), Error> {
        let args = &command.args[1..];
        match command.path() {
            "echo" => self.echo(args),
            "pwd" => self.pwd(args),
            "cd" => self.cd(args),
            "ls" => self.ls(args),
            "cat" => self.cat(args),
            path => Err(Error::UnknownCommand {
                command: path.to_string(),
            }),
        }
    }

    fn echo(&self, args: &[&str]) -> Result<(), Error> {
        kprintln!("{}", args.join(" "));
        Ok(())
    }

    fn pwd(&self, args: &[&str]) -> Result<(), Error> {
        if args.len() > 0 {
            return Err(Error::InvalidArgs {
                message: "usage: pwd".into(),
            });
        }

        kprintln!("{}", self.cwd.display());

        Ok(())
    }

    fn cd(&mut self, args: &[&str]) -> Result<(), Error> {
        if args.len() != 1 {
            return Err(Error::InvalidArgs {
                message: "usage: cd <directory>".into(),
            });
        }

        let cwd = self.cwd.canonical_join(&PathBuf::from(args[0]))?;

        self.fs.open_dir(&cwd)?;

        self.cwd = cwd;

        Ok(())
    }

    fn ls(&self, args: &[&str]) -> Result<(), Error> {
        let usage_err = || {
            Err(Error::InvalidArgs {
                message: "usage: ls [-a] [directory]".into(),
            })
        };

        let (path, show_hidden) = match args.len() {
            0 => (self.cwd.clone(), false),
            1 => (PathBuf::from(args[0]), false),
            2 => {
                if args[0] != "-a" {
                    return usage_err();
                }
                (PathBuf::from(args[1]), true)
            }
            _ => return usage_err(),
        };

        let path = if path.is_relative() {
            self.cwd.canonical_join(&path)?
        } else {
            path
        };

        self.fs
            .open_dir(path)?
            .entries()?
            .filter(|entry| show_hidden || !entry.metadata().hidden())
            .for_each(|entry| kprintln!("{}", entry));
        Ok(())
    }

    fn cat(&self, args: &[&str]) -> Result<(), Error> {
        if args.len() == 0 {
            return Err(Error::InvalidArgs {
                message: "usage: cat <file> [file]..".into(),
            });
        }

        args.iter()
            .map(|arg| self.cwd.canonical_join(&PathBuf::from(arg)))
            .collect::<Result<Vec<_>, Error>>()?
            .iter()
            .filter_map(|path| match self.fs.open_file(path) {
                Ok(file) => Some(file),
                Err(err) => {
                    kprintln!("{}: {}", path.display(), err);
                    None
                }
            }).map(|file| {
                let file = BufReader::new(file);
                for line in file.lines() {
                    kprintln!("{}", line?);
                }
                Ok(())
            }).collect::<io::Result<Vec<_>>>()?;
        Ok(())
    }
}

/// Error type for `Command` parse failures.
#[derive(Debug)]
enum Error {
    Empty,
    TooManyArgs,
    InvalidArgs { message: String },
    LineTooLong,
    UnknownCommand { command: String },
    InvalidUtf8,
    Io { error: io::Error },
    Path { path: PathBuf, message: String },
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
            &InvalidArgs { ref message } => write!(f, "invalid arguments -- {}", message),
            &LineTooLong => write!(f, "line too long"),
            &UnknownCommand { ref command } => write!(f, "unknown command -- {}", command),
            &InvalidUtf8 => write!(f, "invalid utf8"),
            &Io { ref error } => write!(f, "{}", error),
            &Path {
                ref path,
                ref message,
            } => write!(f, "{}: {}", path.display(), message),
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

/// Starts a shell using `prefix` as the prefix for each line. This function
/// never returns: it is perpetually in a shell loop.
pub fn shell(fs: &'static FileSystem, prefix: &str) -> ! {
    let shell = Shell::new(fs, prefix);
    shell.repl();
}
