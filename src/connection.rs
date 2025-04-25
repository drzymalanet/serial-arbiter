use std::{
    fs::File,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::serial_port::port_open;

const DEFAULT_COOLOFF_DURATION: Duration = Duration::from_secs(1);

pub struct Connection {
    inner: Mutex<ConnectionInner>,
}

struct ConnectionInner {
    path: Option<PathBuf>,
    file: Option<Arc<Mutex<File>>>,
    last_conn_attempt: Option<Instant>,
    cool_time: Option<Duration>,
}

impl Connection {
    pub fn new() -> Self {
        let state = ConnectionInner {
            path: None,
            file: None,
            last_conn_attempt: None,
            cool_time: Some(DEFAULT_COOLOFF_DURATION),
        };
        Self {
            inner: Mutex::new(state),
        }
    }

    pub fn open(&self) -> io::Result<Arc<Mutex<File>>> {
        let mut state = self.inner.lock().unwrap();
        // Skip if already open
        if let Some(file) = &state.file {
            return Ok(file.clone());
        }
        // Skip if cool-off ongoing
        if let Some(cool_time) = state.cool_time {
            if let Some(last_conn) = state.last_conn_attempt {
                if Instant::now() < last_conn + cool_time {
                    return Err(ErrorKind::QuotaExceeded.into());
                }
            }
            state.last_conn_attempt = Some(Instant::now());
        }
        // Try to open
        match &state.path {
            None => Err(ErrorKind::InvalidFilename.into()),
            Some(path) => match port_open(path) {
                Ok(file) => {
                    let file = Arc::new(Mutex::new(file));
                    state.file = Some(file.clone());
                    state.last_conn_attempt = None;
                    Ok(file)
                }
                Err(err) => Err(err),
            },
        }
    }

    pub fn close(&self) {
        let mut state = self.inner.lock().unwrap();
        state.last_conn_attempt = None;
        state.file = None;
    }

    pub fn set_path(&self, path: impl AsRef<Path>) {
        let mut state = self.inner.lock().unwrap();
        state.path = Some(path.as_ref().into());
        state.file = None;
    }

    pub fn is_open(&self) -> bool {
        let state = self.inner.lock().unwrap();
        state.file.is_some()
    }

    /// Change the duration of cooloff after disconnecting due to an error
    /// and before a new connection attempt is made. If set to None then
    /// another connect attepmpt is tried without any artificial delays.
    pub fn set_cooloff_duration(&self, cooloff: Option<Duration>) {
        let mut inner = self.inner.lock().unwrap();
        inner.cool_time = cooloff;
    }
}
