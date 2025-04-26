use std::{
    io,
    time::{Duration, Instant},
};

use serial_arbiter::*;

fn main() -> io::Result<()> {
    let deadline = Instant::now() + Duration::from_secs(3);

    // Connect
    let port = Arbiter::new();
    port.open("/dev/ttyACM0")?;

    // Transmit request
    port.transmit_str("Hello world\n", deadline)?;
    println!("Request sent. Waiting for response...");

    // Receive response
    let response = port.receive_string(None, Some(deadline))?;
    println!("Got response: {response:?}");

    Ok(())
}
