mod connection;

use connection::{Connection, ReadRequest, WriteRequest};
use crossbeam::channel::{bounded, unbounded, SendError, TryRecvError};
use serialport::{ClearBuffer, SerialPort, SerialPortBuilder};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{io, thread};

pub const POLLING_INTERVAL: Duration = Duration::from_micros(100);

/// # Serial Port Arbiter
///
/// This is a wrapper for `serialport` lib which adds the following features:
/// 1. Prevents deadlocks caused by input buffer starvation
/// 2. Prevents data garbling by implementing transaction queueing
/// 3. Handles gracefully interrupts and timeout errors
/// 4. Handles gracefully connection errors and automatically reconnects
/// 5. Provides a more convenient API than the raw io::Read and io::Write
///
/// **This is an "async-less" library.**
///
/// There are two plain old background threads. They are running in parallel.
/// One is responsible for sending the data to the serial port and the other one
/// is responsible for the reception of the incomming data from the serial port.
/// This design ensures that there will be no buffer underruns even when sending
/// very big messages to the serial port. I.e deadlocks are solved because the
/// serial port is always ready to accept more data from the slave device.
///
/// ## Example
///
/// ```
/// let cfg = serialport::new("/dev/ttyACM0", 4_000_000)
///     .data_bits(DataBits::Eight)
///     .parity(Parity::None)
///     .stop_bits(StopBits::One)
///     .flow_control(FlowControl::None)
///     .timeout(Duration::from_millis(0));
///
/// let port = Arbiter::new(cfg);
///
/// let request = "Hello!\n";
/// let deadline = Instant::now() + Duration::from_millis(100);
/// println!("Sending request: {request}");
/// port.write_str("Hello!", deadline).unwrap();
///
/// let deadline = Instant::now() + Duration::from_millis(100);
/// println!("\nWaiting for response...");
/// let response = port.read_str(deadline);
/// println!("Response: {response:?}");
/// ```
#[derive(Clone)]
pub struct Arbiter {
    bldr: SerialPortBuilder,
    conn: Arc<Connection>,
}

impl Arbiter {
    /// Creates a new arbiter which will handle a serial port
    /// connection defined by the given serial port builder.
    pub fn new(builder: SerialPortBuilder) -> Self {
        Self {
            bldr: builder,
            conn: Arc::new(Connection::new_closed()),
        }
    }

    /// Closes the serial port
    pub fn close(&self) {
        self.conn.set_closed();
    }

    /// Opens the serial port.
    pub fn open(&self) -> io::Result<()> {
        // Skip if already open
        if self.conn.is_open() {
            return Ok(());
        }

        // Setup read and write channels
        let (write_tx, write_rx) = unbounded::<WriteRequest>();
        let (read_tx, read_rx) = unbounded::<ReadRequest>();

        // Setup two instances of serial port
        let port_builder = self.bldr.clone();
        let mut port_rx = port_builder.open()?;
        let mut port_tx = port_rx
            .try_clone()
            .expect("Cloning the SerialPort is required for all this to work.");

        // Spawn reader thread
        let reader_conn = self.conn.clone();
        thread::spawn(move || loop {
            thread::sleep(POLLING_INTERVAL);
            let request = match read_rx.try_recv() {
                Err(TryRecvError::Empty) => {
                    let _ = port_rx.clear(ClearBuffer::Input);
                    continue;
                }
                Err(TryRecvError::Disconnected) => {
                    return;
                }
                Ok(request) => request,
            };
            match serial_read(&mut port_rx, request.deadline) {
                Ok(data) => {
                    let _ = request.response.try_send(Ok(data));
                }
                Err(err) => {
                    let _ = request.response.try_send(Err(err.kind().into()));
                    reader_conn.set_broken(err);
                    return;
                }
            }
        });

        // Spawn the writer thread
        let writer_conn = self.conn.clone();
        thread::spawn(move || loop {
            thread::sleep(POLLING_INTERVAL);
            let request = match write_rx.recv() {
                Err(_) => {
                    let _ = port_tx.clear(ClearBuffer::Output);
                    return;
                }
                Ok(request) => request,
            };
            match serial_write(&mut port_tx, request.data, request.deadline) {
                Ok(()) => {
                    let _ = request.response.try_send(Ok(()));
                }
                Err(err) => {
                    let _ = request.response.try_send(Err(err.kind().into()));
                    writer_conn.set_broken(err);
                    return;
                }
            }
        });

        self.conn.set_open(write_tx, read_tx);
        Ok(())
    }

    /// Reads raw data from the serial port.
    pub fn read(&self, deadline: Instant) -> io::Result<Vec<u8>> {
        self.open()?;
        let (response, result) = bounded(1);
        let request = ReadRequest { deadline, response };
        let read_channel = self.conn.get_read_channel()?;
        if let Err(SendError { .. }) = read_channel.send(request) {
            return Err(io::ErrorKind::ConnectionAborted.into());
        }
        match result.recv() {
            Err(_) => Err(io::ErrorKind::ConnectionAborted.into()),
            Ok(result) => result,
        }
    }

    /// Reads a string from the serial port.
    pub fn read_string(&self, deadline: Instant) -> io::Result<Option<String>> {
        let data = self.read(deadline)?;
        if !data.is_empty() {
            let text = String::from_utf8_lossy(&data).to_string();
            Ok(Some(text))
        } else {
            Ok(None)
        }
    }

    /// Writes the given data to the serial port.
    pub fn write(&self, data: impl AsRef<[u8]>, deadline: Instant) -> io::Result<()> {
        self.open()?;
        let data = Vec::from(data.as_ref());
        let (response, result) = bounded(1);
        let request = WriteRequest {
            data,
            deadline,
            response,
        };
        let write_channel = self.conn.get_write_channel()?;
        if let Err(SendError { .. }) = write_channel.send(request) {
            return Err(io::ErrorKind::ConnectionAborted.into());
        }
        match result.recv() {
            Err(_) => Err(io::ErrorKind::ConnectionAborted.into()),
            Ok(result) => result,
        }
    }

    /// Writes the given text to the serial port.
    pub fn write_str(&self, text: impl AsRef<str>, deadline: Instant) -> io::Result<()> {
        self.write(text.as_ref().as_bytes(), deadline)
    }
}

fn serial_read(port: &mut Box<dyn SerialPort>, deadline: Instant) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; 1024];
    let mut cnt = 0;
    loop {
        if cnt == buf.len() {
            // Grow the buffer
            buf.extend([0; 1024]);
        }
        match port.read(&mut buf.as_mut_slice()[cnt..]) {
            Ok(0) => {
                // No more data
                buf.truncate(cnt);
                return Ok(buf);
            }
            Ok(n) => {
                // Got more data
                cnt += n;
                continue;
            }
            Err(err) => match err.kind() {
                io::ErrorKind::Interrupted => {
                    // Interrupted
                    continue;
                }
                io::ErrorKind::TimedOut => {
                    if Instant::now() < deadline {
                        // We ignore the single read timeouts
                        // and instead we check the total deadline
                        continue;
                    } else {
                        // Deadline reached
                        buf.truncate(cnt);
                        return Ok(buf);
                    }
                }
                _ => {
                    // I/O Error
                    return Err(err);
                }
            },
        }
    }
}

fn serial_write(
    port: &mut Box<dyn SerialPort>,
    data: Vec<u8>,
    deadline: Instant,
) -> io::Result<()> {
    let mut cnt = 0;
    loop {
        if cnt == data.len() {
            return Ok(());
        }
        match port.write(&data[cnt..]) {
            Ok(0) => {
                // Could not write
                port.flush()?;
                continue;
            }
            Ok(n) => {
                // Wrote data
                cnt += n;
                port.flush()?;
                continue;
            }
            Err(err) => match err.kind() {
                io::ErrorKind::Interrupted => {
                    // Interrupted
                    continue;
                }
                io::ErrorKind::TimedOut => {
                    if Instant::now() < deadline {
                        // We ignore the single write timeouts
                        // and instead we check the total deadline
                        continue;
                    } else {
                        // The total transaction deadline is reached
                        return Err(io::ErrorKind::TimedOut.into());
                    }
                }
                _ => {
                    // I/O Error
                    return Err(err);
                }
            },
        }
    }
}
