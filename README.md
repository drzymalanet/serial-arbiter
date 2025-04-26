# Serial Port Arbiter

This is a Linux-only serial port library that offers the following benefits
over directly using `/dev/tty`:
1. Opens the `/dev/tty` file with flags for non-blocking access.
2. Sets the `termios` flags to use the TTY in raw mode.
3. Prevents deadlocks caused by input buffer starvation.
4. Prevents data garbling by implementing transaction arbitration.
5. Gracefully handles interrupts and timeout errors.
6. Gracefully handles connection errors and automatically reconnects.
7. Provides a more convenient API than the raw `io::Read` and `io::Write`.

**This is an "async-less" library**, and it is intended to remain that way.
If you need asynchronous behavior, you can easily make it async-compatible in your own code.

## Example

```rust
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
```

Go to the examples directory to see how automatic reconnection is working or how to use jsonrpc.
