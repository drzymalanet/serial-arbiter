mod connection;
mod serial_port;

use connection::Connection;
use crossbeam::channel::{bounded, Receiver, RecvTimeoutError, SendError, Sender};
use serial_port::{port_recv, port_send};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{io, mem, thread};

pub const POLLING_INTERVAL: Duration = Duration::from_millis(1);

/// # Serial Port Arbiter
///
/// This is a Linux-only serial port library that offers the following benefits
/// over directly using `/dev/tty`:
/// 1. Opens the `/dev/tty` file with flags for non-blocking access.
/// 2. Sets the `termios` flags to use the TTY in raw mode.
/// 3. Prevents deadlocks caused by input buffer starvation.
/// 4. Prevents data garbling by implementing transaction arbitration.
/// 5. Gracefully handles interrupts and timeout errors.
/// 6. Gracefully handles connection errors and automatically reconnects.
/// 7. Provides a more convenient API than the raw `io::Read` and `io::Write`.
///
/// **This is an "async-less" library**, and it is intended to remain that way.  
/// If you need asynchronous behavior, you can easily make it async-compatible in your own code.
#[derive(Clone)]
pub struct Arbiter {
    conn: Arc<Connection>,
    chan: Sender<Request>,
}

enum Request {
    Clear(Clear),
    Transmit(Transmit),
    Receive(Receive),
}

struct Clear {
    pub response: Sender<io::Result<()>>,
}

struct Transmit {
    pub tx_bytes: Arc<[u8]>,
    pub deadline: Instant,
    pub response: Sender<io::Result<()>>,
}

struct Receive {
    pub until: Option<u8>,
    pub deadline: Option<Instant>,
    pub response: Sender<io::Result<Option<Vec<u8>>>>,
}

struct WorkerThread {
    buff: VecDeque<u8>,
    conn: Arc<Connection>,
    chan: Receiver<Request>,
}

impl Default for Arbiter {
    fn default() -> Self {
        Self::new()
    }
}

impl Arbiter {
    /// Creates a new arbiter which will handle a serial port
    /// connection defined by the given serial port builder.
    pub fn new() -> Self {
        let conn = Arc::new(Connection::new());

        // Setup read and write channels
        let (req_tx, req_rx) = bounded::<Request>(0);

        // Spawn background thread
        let worker = WorkerThread::new(conn.clone(), req_rx);
        worker.spawn();

        Self { conn, chan: req_tx }
    }

    /// Closes the serial port
    pub fn close(&self) {
        self.conn.close();
    }

    /// Returns true if the connection is open
    pub fn is_open(&self) -> bool {
        self.conn.is_open()
    }

    /// Opens the serial port.
    pub fn open(&self, path: impl AsRef<Path>) -> io::Result<()> {
        self.conn.set_path(path);
        self.conn.open().map(|_| ())
    }

    /// Clear the Rx buffer of the serial port.
    pub fn clear_rx_buff(&self) -> io::Result<()> {
        let (response, result_ch) = bounded(1);
        let request = Request::Clear(Clear { response });
        if let Err(SendError { .. }) = self.chan.send(request) {
            return Err(io::Error::other("Internal error"));
        }
        match result_ch.recv() {
            Err(_) => Err(io::Error::other("Internal error")),
            Ok(result) => result,
        }
    }

    /// Transmits data to the serial port.
    pub fn transmit(&self, tx_bytes: Arc<[u8]>, deadline: Instant) -> io::Result<()> {
        let (response, result_ch) = bounded(1);
        let request = Request::Transmit(Transmit {
            tx_bytes,
            deadline,
            response,
        });
        if let Err(SendError { .. }) = self.chan.send(request) {
            return Err(io::Error::other("Internal error"));
        }
        match result_ch.recv() {
            Err(_) => Err(io::Error::other("Internal error")),
            Ok(result) => result,
        }
    }

    /// Transmits a string to the serial port.
    /// Returns any bytes received during transmission.
    pub fn transmit_str(&self, str: impl AsRef<str>, deadline: Instant) -> io::Result<()> {
        let tx_bytes = str.as_ref().as_bytes().into();
        self.transmit(tx_bytes, deadline)
    }

    /// Receives data from the serial port
    pub fn receive(
        &self,
        until: Option<u8>,
        deadline: Option<Instant>,
    ) -> io::Result<Option<Vec<u8>>> {
        let (response, result_ch) = bounded(1);
        let request = Request::Receive(Receive {
            until,
            deadline,
            response,
        });
        if let Err(SendError { .. }) = self.chan.send(request) {
            return Err(io::Error::other("Internal error"));
        }
        match result_ch.recv() {
            Err(_) => Err(io::Error::other("Internal error")),
            Ok(result) => result,
        }
    }

    /// Receives data from the serial port and converts to a String
    pub fn receive_string(
        &self,
        until: Option<u8>,
        deadline: Option<Instant>,
    ) -> io::Result<Option<String>> {
        let result = self.receive(until, deadline)?;
        Ok(result.map(|x| String::from_utf8_lossy(&x).to_string()))
    }

    /// Change the duration of cooloff after disconnecting due to an error
    /// and before a new connection attempt is made. If set to None then
    /// another connect attepmpt is tried without any artificial delays.
    pub fn set_cooloff_duration(&self, cooloff: Option<Duration>) {
        self.conn.set_cooloff_duration(cooloff);
    }
}

impl WorkerThread {
    fn new(connection: Arc<Connection>, requests: Receiver<Request>) -> Self {
        Self {
            buff: VecDeque::new(),
            conn: connection,
            chan: requests,
        }
    }

    fn spawn(mut self) {
        thread::spawn(move || loop {
            self.process();
        });
    }

    fn process(&mut self) {
        loop {
            let request_recv = self.chan.recv_timeout(POLLING_INTERVAL);
            match request_recv {
                Err(RecvTimeoutError::Disconnected) => {
                    // Stop signal
                    return;
                }
                Err(RecvTimeoutError::Timeout) => {
                    // Collect incomming data to avoid RX buffer starvation
                    let _ = self.receive_from_port(None, None);
                }
                Ok(request) => match request {
                    Request::Clear(tx) => {
                        let result = if self.conn.is_open() {
                            self.receive_from_port(None, None)
                        } else {
                            Ok(())
                        };
                        self.buff.clear();
                        let _ = tx.response.try_send(result);
                    }
                    Request::Transmit(tx) => {
                        let result = self.transmit_to_port(tx.tx_bytes, tx.deadline);
                        let _ = tx.response.try_send(result);
                    }
                    Request::Receive(rx) => {
                        // Check if we can skip reading from port
                        if let Some(delimiter) = rx.until {
                            // If we have all needed data
                            let colltype = CollectKind::UntilOrNothing(delimiter);
                            if let Some(data) = self.collect_from_buff(colltype) {
                                // Return the data immediately
                                let _ = rx.response.try_send(Ok(Some(data)));
                                continue;
                            }
                        }

                        // Receive all new available data from the port
                        if let Err(err) = self.receive_from_port(rx.until, rx.deadline) {
                            // Error when receiving data
                            let _ = rx.response.try_send(Err(err));
                            continue;
                        }

                        // Return collected data
                        let colltype = match rx.until {
                            None => CollectKind::Everything,
                            Some(delimiter) => CollectKind::UntilOrEverything(delimiter),
                        };
                        let data = self.collect_from_buff(colltype);
                        let _ = rx.response.try_send(Ok(data));
                    }
                },
            };
        }
    }

    fn receive_from_port(
        &mut self,
        until: Option<u8>,
        deadline: Option<Instant>,
    ) -> io::Result<()> {
        let file_mutex = self.conn.open()?;
        let mut file = file_mutex.lock().unwrap();
        let result = port_recv(&mut file, &mut self.buff, until, deadline);
        if result.is_err() {
            self.conn.close();
        }
        result
    }

    fn transmit_to_port(&mut self, data: Arc<[u8]>, deadline: Instant) -> io::Result<()> {
        let file_mutex = self.conn.open()?;
        let mut file = file_mutex.lock().unwrap();
        let result = port_send(&mut file, &data, &mut self.buff, deadline);
        if result.is_err() {
            self.conn.close();
        }
        result
    }

    /// Collect data from the RX FIFO buffer.
    fn collect_from_buff(&mut self, collect: CollectKind) -> Option<Vec<u8>> {
        if self.buff.is_empty() {
            return None;
        }
        match collect {
            CollectKind::Everything => self.collect_from_buff_everything(),
            CollectKind::UntilOrEverything(delimiter) => {
                if let Some(pos) = self.buff.iter().position(|x| x == &delimiter) {
                    self.collect_from_buff_count(pos + 1)
                } else {
                    self.collect_from_buff_everything()
                }
            }
            CollectKind::UntilOrNothing(delimiter) => {
                if let Some(pos) = self.buff.iter().position(|x| x == &delimiter) {
                    self.collect_from_buff_count(pos + 1)
                } else {
                    None
                }
            }
        }
    }

    /// Collect the given count of elements from the RX FIFO buffer
    fn collect_from_buff_count(&mut self, count: usize) -> Option<Vec<u8>> {
        if self.buff.is_empty() {
            // Return nothing
            return None;
        }
        if self.buff.len() <= count {
            return self.collect_from_buff_everything();
        }
        // Return part of the buffer
        let mut data = self.buff.split_off(count);
        mem::swap(&mut self.buff, &mut data);
        Some(data.into())
    }

    /// Collect all data from the RX FIFO buffer
    fn collect_from_buff_everything(&mut self) -> Option<Vec<u8>> {
        if self.buff.is_empty() {
            return None;
        }
        let mut data = VecDeque::new();
        mem::swap(&mut self.buff, &mut data);
        Some(data.into())
    }
}

enum CollectKind {
    /// Consume all data from the buffer
    Everything,
    /// Consume all data from the buffer but only until the given byte.
    /// If the byte is not found then consume the whole buffer.
    UntilOrEverything(u8),
    /// Consume data from the buffer but only until the given byte.
    /// If the byte is not found then do not consume any data from the buffer.
    UntilOrNothing(u8),
}
