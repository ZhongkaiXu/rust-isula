use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::time::Duration;

use tokio::io::unix::AsyncFd;

use crate::grpc::start as rpc;

// Use nonblocking stdin so an idle terminal cannot prevent container exit.
// Regular files cannot be registered with epoll and are read directly.
pub(crate) struct Input {
    readiness: Option<AsyncFd<RawFd>>,
    file: File,
    flags: i32,
}

impl Input {
    pub(crate) fn new() -> io::Result<Self> {
        // SAFETY: duplicate stdin; File takes ownership only of the new descriptor.
        let fd = unsafe { libc::fcntl(0, libc::F_DUPFD_CLOEXEC, 3) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = unsafe { File::from_raw_fd(fd) };
        // SAFETY: fd is open and both commands take integer arguments.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut input = Self {
            readiness: None,
            file,
            flags,
        };
        input.readiness = match AsyncFd::new(fd) {
            Ok(fd) => Some(fd),
            Err(err) if err.raw_os_error() == Some(libc::EPERM) => None,
            Err(err) => return Err(err),
        };
        Ok(input)
    }

    pub(crate) async fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        loop {
            let result = if let Some(fd) = &self.readiness {
                match fd.readable().await?.try_io(|_| self.file.read(buffer)) {
                    Ok(result) => result,
                    Err(_) => continue,
                }
            } else {
                self.file.read(buffer)
            };
            match result {
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                result => return result,
            }
        }
    }
}

impl Drop for Input {
    fn drop(&mut self) {
        // SAFETY: file is still open; dup shares status flags with the caller's stdin.
        unsafe {
            libc::fcntl(self.file.as_raw_fd(), libc::F_SETFL, self.flags);
        }
    }
}

pub(super) struct Terminal(Option<libc::termios>);

impl Terminal {
    pub(super) fn new(raw: bool) -> io::Result<Self> {
        let mut terminal = Self(None);
        if raw && unsafe { libc::isatty(0) } == 1 {
            // SAFETY: termios is initialized and its pointer is valid for each syscall.
            let mut original = unsafe { std::mem::zeroed() };
            if unsafe { libc::tcgetattr(0, &mut original) } != 0 {
                return Err(io::Error::last_os_error());
            }
            let mut settings = original;
            unsafe {
                libc::cfmakeraw(&mut settings);
            }
            settings.c_oflag |= libc::OPOST;
            if unsafe { libc::tcsetattr(0, libc::TCSAFLUSH, &settings) } != 0 {
                return Err(io::Error::last_os_error());
            }
            terminal.0 = Some(original);
        }
        Ok(terminal)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if let Some(settings) = &self.0 {
            // SAFETY: restore the settings saved from this terminal on all return paths.
            unsafe {
                libc::tcsetattr(0, libc::TCSAFLUSH, settings);
            }
        }
    }
}

pub(super) async fn copy_input(mut input: Input, fifo: AsyncFd<File>, tty: bool) -> io::Result<()> {
    let mut buffer = [0; 16 * 1024];
    let mut escape = false;
    loop {
        let size = input.read(&mut buffer).await?;
        if size == 0 {
            // The native local console keeps stdin open until container output closes.
            return std::future::pending().await;
        }
        let mut length = 0;
        let mut detached = false;
        for index in 0..size {
            let byte = buffer[index];
            if tty {
                // Native local TTY escape is Ctrl-A followed by q.
                if byte == 1 && !escape {
                    escape = true;
                    continue;
                }
                if escape && byte == b'q' {
                    detached = true;
                    break;
                }
                escape = false;
            }
            buffer[length] = byte;
            length += 1;
        }
        let mut bytes = &buffer[..length];
        while !bytes.is_empty() {
            let mut ready = fifo.writable().await?;
            match ready.try_io(|fd| fd.get_ref().write(bytes)) {
                Ok(Ok(0)) => return Err(io::ErrorKind::WriteZero.into()),
                Ok(Ok(size)) => bytes = &bytes[size..],
                Ok(Err(err)) if err.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(err)) if err.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
                Ok(Err(err)) => return Err(err),
                Err(_) => continue,
            }
        }
        if detached {
            return Ok(());
        }
        tokio::task::yield_now().await;
    }
}

pub(super) async fn resize(host: &str, id: &str, tty: bool) {
    resize_exec(host, id, "", tty).await
}

pub(super) async fn resize_exec(host: &str, id: &str, suffix: &str, tty: bool) {
    if !tty || unsafe { libc::isatty(0) } != 1 {
        return std::future::pending().await;
    }
    let mut previous = (0, 0);
    loop {
        // Match the native client's one-second window-size check, without a thread.
        tokio::time::sleep(Duration::from_secs(1)).await;
        let mut size: libc::winsize = unsafe { std::mem::zeroed() };
        // SAFETY: size is writable storage for TIOCGWINSZ.
        if unsafe { libc::ioctl(0, libc::TIOCGWINSZ, &mut size) } != 0 {
            continue;
        }
        let current = (size.ws_row, size.ws_col);
        if current != previous
            && rpc::resize_exec(host, id, suffix, current.0.into(), current.1.into())
                .await
                .is_ok()
        {
            previous = current;
        }
    }
}
