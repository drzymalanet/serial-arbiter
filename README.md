# Serial Port Arbiter

This is a wrapper for `serialport` lib which adds the following features:
1. Prevents deadlocks caused by input buffer starvation
2. Prevents data garbling by implementing transaction queueing
3. Handles gracefully interrupts and timeout errors
4. Handles gracefully connection errors and automatically reconnects
5. Provides a more convenient API than the raw io::Read and io::Write

**This is an "async-less" library.**

There are two plain old background threads. They are running in parallel.
One is responsible for sending the data to the serial port and the other one
is responsible for the reception of the incomming data from the serial port.
This design ensures that there will be no buffer underruns even when sending
very big messages to the serial port. I.e deadlocks are solved because the
serial port is always ready to accept more data from the slave device.

## Example

```rust
use std::io;
use serialport::*;
use serial_arbiter::*;
use std::time::*;

fn main() -> io::Result<()> {
    let cfg = serialport::new("/dev/ttyACM0", 115200)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(0));

    let port = Arbiter::new(cfg);

    let request = format!("{}\n", r#"{"jsonrpc":"2.0","id":777,"method":"hello_world","params":"Hello World!"}"#);
    let deadline = Instant::now() + Duration::from_millis(10);
    print!("\nSending request:\n{request}");
    port.write_str(request, deadline)?;

    let deadline = Instant::now() + Duration::from_millis(10);
    println!("\nWaiting for response...");
    let response = port.read_string(deadline)?.unwrap_or("<EMPTY>".to_string());
    println!("Response: {response}\n");

    Ok(())
}
```

You should be seeing something like this:

```text
Sending request:
{"jsonrpc":"2.0","id":777,"method":"hello_world","params":"Hello World!"}

Waiting for response...
Response: {"jsonrpc":"2.0","error":{"code":32935,"message":"Unknown method"},"id":777}
```
