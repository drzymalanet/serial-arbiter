use std::{collections::VecDeque, fs::File, io::{self, Error, Read, Write}, os::fd::{AsRawFd, BorrowedFd, FromRawFd}, path::Path, time::Instant};

use nix::{errno::Errno, poll::{PollFd, PollFlags, PollTimeout}};
use termios::Termios;


/// Open the file under the given path with flags specific for non blocking driect i/o access.
/// 
/// # Safety
/// 
/// The fd passed in is an owned file descriptor and it is open because
/// we get the file descriptor from the fcntl::open function call.
pub fn port_open(path: impl AsRef<Path>) -> io::Result<File> {
    use nix::fcntl::OFlag;
    use nix::sys::stat::Mode;

    let oflag = 
        // Open for reading and writing.
        OFlag::O_RDWR | 
        // The file offset shall be set to the end of the file prior to each write.
        OFlag::O_APPEND | 
        // Write I/O operations shall complete as defined by synchronized I/O data integrity completion
        OFlag::O_DSYNC | 
        // Read I/O operations shall complete as defined by synchronized I/O data integrity completion
        OFlag::O_RSYNC |
        // Write I/O operations shall complete as defined by synchronized I/O file integrity completion.
        OFlag::O_SYNC |
        // open() shall not cause the terminal device to become the controlling terminal for the process.
        OFlag::O_NOCTTY |
        // File I/O is done directly to/from user-space buffers.
        // OFlag::O_DIRECT |
        // open() function shall return without blocking for the device to be ready or available
        OFlag::O_NONBLOCK |
        OFlag::O_NDELAY;
        // The application need not specify the O_TTY_INIT flag when opening pseudo-terminals

    let fd = nix::fcntl::open(path.as_ref(), oflag, Mode::empty())?;

    let mut termios = Termios::from_fd(fd)?;
    termios::tcgetattr(fd, &mut termios)?;

    // println!("Input modes: ");
    // println!("    BRKINT  [{}] Signal interrupt on break.", termios.c_iflag & termios::BRKINT as u32);
    // println!("    ICRNL   [{}] Map CR to NL on input.", termios.c_iflag & termios::ICRNL as u32);
    // println!("    IGNBRK  [{}] Ignore break condition.", termios.c_iflag & termios::IGNBRK as u32);
    // println!("    IGNCR   [{}] Ignore CR.", termios.c_iflag & termios::IGNCR as u32);
    // println!("    IGNPAR  [{}] Ignore characters with parity errors.", termios.c_iflag & termios::IGNPAR as u32);
    // println!("    INLCR   [{}] Map NL to CR on input.", termios.c_iflag & termios::INLCR as u32);
    // println!("    INPCK   [{}] Enable input parity check.", termios.c_iflag & termios::INPCK as u32);
    // println!("    ISTRIP  [{}] Strip character.", termios.c_iflag & termios::ISTRIP as u32);
    // println!("    IXANY   [{}] Enable any character to restart output.", termios.c_iflag & termios::IXANY as u32);
    // println!("    IXOFF   [{}] Enable start/stop input control.", termios.c_iflag & termios::IXOFF as u32);
    // println!("    IXON    [{}] Enable start/stop output control.", termios.c_iflag & termios::IXON as u32);
    // println!("    PARMRK  [{}] Mark parity errors.", termios.c_iflag & termios::PARMRK as u32);
    // println!("");

    // println!("Output modes:");
    // println!("    OPOST       [{}] Post-process output.", termios.c_oflag & termios::OPOST as u32);
    // println!("    ONLCR       [{}] Map NL to CR-NL on output.", termios.c_oflag & termios::ONLCR as u32);
    // println!("    OCRNL       [{}] Map CR to NL on output.", termios.c_oflag & termios::OCRNL as u32);
    // println!("    ONOCR       [{}] No CR output at column 0.", termios.c_oflag & termios::ONOCR as u32);
    // println!("    ONLRET      [{}] NL performs CR function.", termios.c_oflag & termios::ONLRET as u32);
    // println!("    OFDEL       [{}] Fill is DEL.", termios.c_oflag & termios::os::linux::OFDEL as u32);
    // println!("    OFILL       [{}] Use fill characters for delay.", termios.c_oflag & termios::os::linux::OFILL as u32);
    // println!("    NLDLY.NL0   [{}] Newline delay type 0.", termios.c_oflag & termios::os::linux::NL0 as u32);
    // println!("    NLDLY.NL1   [{}] Newline delay type 1.", termios.c_oflag & termios::os::linux::NL1 as u32);
    // println!("    CRDLY.CR0   [{}] Carriage-return delay type 0.", termios.c_oflag & termios::os::linux::CR0 as u32);
    // println!("    CRDLY.CR1   [{}] Carriage-return delay type 1.", termios.c_oflag & termios::os::linux::CR1 as u32);
    // println!("    CRDLY.CR2   [{}] Carriage-return delay type 2.", termios.c_oflag & termios::os::linux::CR2 as u32);
    // println!("    CRDLY.CR3   [{}] Carriage-return delay type 3.", termios.c_oflag & termios::os::linux::CR3 as u32);
    // println!("    TABDLY.TAB0 [{}] Horizontal-tab delay type 0.", termios.c_oflag & termios::os::linux::TAB0 as u32);
    // println!("    TABDLY.TAB1 [{}] Horizontal-tab delay type 1.", termios.c_oflag & termios::os::linux::TAB1 as u32);
    // println!("    TABDLY.TAB2 [{}] Horizontal-tab delay type 2.", termios.c_oflag & termios::os::linux::TAB2 as u32);
    // println!("    TABDLY.TAB3 [{}] Expand tabs to spaces.", termios.c_oflag & termios::os::linux::TAB3 as u32);
    // println!("    BSDLY.BS0   [{}] Backspace-delay type 0.", termios.c_oflag & termios::os::linux::BS0 as u32);
    // println!("    BSDLY.BS1   [{}] Backspace-delay type 1.", termios.c_oflag & termios::os::linux::BS1 as u32);
    // println!("    VTDLY.VT0   [{}] Vertical-tab delay type 0.", termios.c_oflag & termios::os::linux::VT0 as u32);
    // println!("    VTDLY.VT1   [{}] Vertical-tab delay type 1.", termios.c_oflag & termios::os::linux::VT1 as u32);
    // println!("    FFDLY.FF0   [{}] Form-feed delay type 0.", termios.c_oflag & termios::os::linux::FF0 as u32);
    // println!("    FFDLY.FF1   [{}] Form-feed delay type 1.", termios.c_oflag & termios::os::linux::FF1 as u32);

    // println!("Output modes:  0x{:08X}", termios.c_oflag);
    // println!("Control modes: 0x{:08X}", termios.c_cflag);
    // println!("Local modes:   0x{:08X}", termios.c_lflag);
    // println!("Control characters: {:?}", termios.c_cc);

    // termios.c_oflag = 0x00000004;
    // termios.c_cflag = 0x00001CB2;
    // termios.c_lflag = 0x00000A30;
    // termios.c_cc = [3, 28, 127, 21, 4, 0, 1, 0, 17, 19, 26, 0, 18, 15, 23, 22, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    // termios::cfsetspeed(&mut termios, 19200)?;
    termios::cfmakeraw(&mut termios);
    termios::tcsetattr(fd, termios::TCSANOW, &termios)?;

    let file = unsafe {
        File::from_raw_fd(fd)
    };
    Ok(file)
}



#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PollKind {
    ForRead,
    ForWrite,
}


#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PollResult {
    TimedOut,
    ReadReady,
    WriteReady,
    Undocumented,
}


/// Poll the port to check if a read or readwrite can be performed.
/// 
/// If deadline is provided then the call will block and wait until
/// the port becomes ready for either read or write operation.
/// 
/// # Safety
/// 
/// The fd remains open and valid for the duration of the returned BorrowedFd object
/// because we borrow a raw pointer from the `&File` only for the duration of the function.
pub fn port_poll(port: &File, poll: PollKind, deadline: Option<Instant>) -> io::Result<PollResult> {
    let fd = unsafe {
        BorrowedFd::borrow_raw(port.as_raw_fd())
    };
    let timeout = match deadline {
        Some(deadline) => {
            let now = Instant::now();
            let time_left = deadline.saturating_duration_since(now);
            PollTimeout::try_from(time_left).unwrap_or(PollTimeout::ZERO)
        },
        None => PollTimeout::ZERO,
    };
    let input_flags = match poll {
        PollKind::ForRead => {
            PollFlags::POLLIN |
            PollFlags::POLLPRI |
            PollFlags::POLLRDNORM |
            PollFlags::POLLRDBAND
        },
        PollKind::ForWrite => {
            PollFlags::POLLPRI |
            PollFlags::POLLOUT |
            PollFlags::POLLWRNORM |
            PollFlags::POLLWRBAND
        },
    };
    let mut pollfd = [PollFd::new(fd, input_flags)];
    let poll_result = nix::poll::poll(&mut pollfd, timeout);
    match poll_result {
        // Upon failure, poll() shall return -1 and set errno to indicate the error.
        Err(errno) => {
            Err(Error::from(errno))
        },
        // Upon successful completion, poll() shall return a non-negative value.
        Ok(rc) if rc < 0 => {
            // This should never happen but if it does then errno might have info.
            Err(Error::from(Errno::last()))
        },
        // A value of 0 indicates that the call timed out and no file descriptors have been selected.
        Ok(0) => {
            Ok(PollResult::TimedOut)
        },
        // A positive value indicates the total number of pollfd structures that have selected events 
        Ok(_) => {
            let revents = match pollfd[0].revents() {
                Some(flags) => flags,
                None => {
                    // No revent flags provided
                    return Ok(PollResult::TimedOut)
                },
            };

            // Check for device disconnection
            if revents.intersects(PollFlags::POLLHUP) {
                return Err(Error::other("POLLHUP: Device has been disconnected"));
            }

            // Check for invalid file descriptor
            if revents.intersects(PollFlags::POLLNVAL) {
                return Err(Error::other("POLLNVAL: Invalid fd member"));
            }

            // Check for poll errors
            if revents.intersects(PollFlags::POLLERR) {
                return Err(Error::other("POLLERR: An error has occurred"));
            }

            // Success - Write ready
            let pf_write_ready = 
                PollFlags::POLLOUT |    // Normal data may be written without blocking.
                PollFlags::POLLWRNORM | // Equivalent to POLLOUT.
                PollFlags::POLLWRBAND;  // Priority data may be written.
            if revents.intersects(pf_write_ready) {
                return Ok(PollResult::WriteReady);
            }

            // Success - Read ready
            let pf_read_ready = 
                PollFlags::POLLIN |     // Data other than high-priority data may be read without blocking.
                PollFlags::POLLRDNORM | // Normal data may be read without blocking.
                PollFlags::POLLRDBAND | // Priority data may be read without blocking.
                PollFlags::POLLPRI;     // High priority data may be read without blocking.
            if revents.intersects(pf_read_ready) {
                return Ok(PollResult::ReadReady);
            }

            // Success but status undocumented
            Ok(PollResult::Undocumented)
        }
    }
}


/// Read some data from the port. EOF, Interrupt and TimedOut errors are
/// treated as not an error and an Ok variant is returned in such cases.
pub fn port_read(port: &mut File, data: &mut VecDeque<u8>) -> io::Result<()> {
    let mut buf = [0; 1024 * 1024];
    loop {
        match port.read(&mut buf) {
            Ok(0) => {
                // EOF - No more data
                return Ok(())
            }
            Ok(n) => {
                // OK - Data was read
                data.extend(&buf[0..n]);
            }
            Err(err) => match err.kind() {
                io::ErrorKind::Interrupted => {
                    // Read interrupt - Ignored. This is not an error for our use case.
                    return Ok(())
                },
                io::ErrorKind::TimedOut => {
                    // Read timeout - Ignored. This is not an error for our use case.
                    return Ok(())
                },
                io::ErrorKind::WouldBlock => {
                    // Would block - Ignored. This is not an error for our use case.
                    return Ok(())
                }
                _ => {
                    // I/O Error
                    return Err(err)
                },
            },
        }
    }
}


/// Write some data to the port. EOF, Interrupt and TimedOut errors are
/// treated as not an error and an Ok variant is returned in such cases.
pub fn port_write(port: &mut File, data: &mut VecDeque<u8>) -> io::Result<()> {
    let buf = data.make_contiguous();
    match port.write(buf) {
        Ok(0) => {
            // EOF - Ingored. This is not an error for our use case.
            Ok(())
        },
        Ok(n) => {
            // OK - Wrote some data
            let _ = data.drain(0..n);
            Ok(())
        }
        Err(err) => match err.kind() {
            io::ErrorKind::Interrupted => {
                // Write interrupt - Ignored. This is not an error for our use case.
                Ok(())
            },
            io::ErrorKind::TimedOut => {
                // Write timeout - Ignored. This is not an error for our use case.
                Ok(())
            },
            io::ErrorKind::WouldBlock => {
                // Would block - Ignored. This is not an error for our use case.
                Ok(())
            }
            _ => {
                // I/O Error
                Err(err)
            },
        },
    }
}


/// Send all data to the port or timeout
pub fn port_send(port: &mut File, send: &[u8], recv: &mut VecDeque<u8>, deadline: Instant) -> io::Result<()> {
    let mut send = VecDeque::from(send.to_vec());

    loop {
        // Check if the port is ready
        match port_poll(port, PollKind::ForWrite, Some(deadline))? {
            PollResult::TimedOut => {
                // Deadline is reached. Ignore, we will check deadline manually.
                // return Err(io::ErrorKind::TimedOut.into());
            },
            PollResult::ReadReady => {
                // The port has out of band data in rx buffer
                port_read(port, recv)?;
            },
            PollResult::WriteReady => {
                // The port is ready for sending data
                port_write(port, &mut send)?;
            },
            PollResult::Undocumented => {
                // The poll result has an undocumented value
                // eprintln!("WARNING: The result value of the `poll` syscall is unexpected / undocumented");
            }
        }

        // Check if we are done
        if send.is_empty() {
            return Ok(());
        }

        // Check if deadline has passed
        if deadline <= Instant::now() {
            return Err(io::ErrorKind::TimedOut.into());
        }
    }
}


/// Receive data from the port until a given byte or until deadline.
pub fn port_recv(port: &mut File, buff: &mut VecDeque<u8>, until: Option<u8>, deadline: Option<Instant>) -> io::Result<()> {
    loop {
        // Check if the port is ready
        match port_poll(port, PollKind::ForRead, deadline)? {
            PollResult::TimedOut => {
                return Ok(());
            },
            PollResult::ReadReady => {
                port_read(port, buff)?;
            },
            PollResult::WriteReady => {
                // eprintln!("WARNING: PollKind was ForRead but got PollResult WriteReady");
            }
            PollResult::Undocumented => {
                // The poll result has an undocumented value
                // eprintln!("WARNING: The result value of the `poll` syscall is unexpected / undocumented");
            }
        }

        if let Some(delimiter) = until {
            if buff.make_contiguous().contains(&delimiter) {
                return Ok(());
            }
        }
    }
}
