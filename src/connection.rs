use std::{io, sync::Mutex, time::Instant};

use crossbeam::channel::Sender;

pub struct Connection {
    state: Mutex<ConnectionState>,
}

pub struct WriteRequest {
    pub data: Vec<u8>,
    pub deadline: Instant,
    pub response: Sender<io::Result<()>>,
}

pub struct ReadRequest {
    pub deadline: Instant,
    pub response: Sender<io::Result<Vec<u8>>>,
}

enum ConnectionState {
    Closed,
    Open {
        write_channel: Sender<WriteRequest>,
        read_channel: Sender<ReadRequest>,
    },
    Broken(io::Error),
}

impl Connection {
    pub fn new_closed() -> Self {
        let state = ConnectionState::Closed;
        Self {
            state: Mutex::new(state),
        }
    }

    pub fn is_open(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(*state, ConnectionState::Open { .. })
    }

    pub fn set_open(&self, write_channel: Sender<WriteRequest>, read_channel: Sender<ReadRequest>) {
        let mut state = self.state.lock().unwrap();
        *state = ConnectionState::Open {
            write_channel,
            read_channel,
        };
    }

    pub fn set_closed(&self) {
        let mut state = self.state.lock().unwrap();
        *state = ConnectionState::Closed;
    }

    pub fn set_broken(&self, reason: io::Error) {
        let mut state = self.state.lock().unwrap();
        *state = ConnectionState::Broken(reason);
    }

    pub fn get_write_channel(&self) -> io::Result<Sender<WriteRequest>> {
        match &*self.state.lock().unwrap() {
            ConnectionState::Closed => Err(io::ErrorKind::NotConnected.into()),
            ConnectionState::Broken(err) => Err(err.kind().into()),
            ConnectionState::Open { write_channel, .. } => Ok(write_channel.clone()),
        }
    }

    pub fn get_read_channel(&self) -> io::Result<Sender<ReadRequest>> {
        match &*self.state.lock().unwrap() {
            ConnectionState::Closed => Err(io::ErrorKind::NotConnected.into()),
            ConnectionState::Broken(err) => Err(err.kind().into()),
            ConnectionState::Open { read_channel, .. } => Ok(read_channel.clone()),
        }
    }
}
