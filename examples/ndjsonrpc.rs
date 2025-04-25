use serde_json::json;
use serial_arbiter::*;
use std::io;
use std::time::*;

fn main() -> io::Result<()> {
    // Connect
    let port = Arbiter::new();
    port.open("/dev/ttyACM0")?;

    // Make a deadline
    let deadline = Instant::now() + Duration::from_millis(10);

    // Transmit request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "hello_world",
        "params": "Hello world!",
        "id": 777,
    }).to_string() + "\n";
    print!("\nSending request:\n{request}");
    port.transmit_str(request, deadline)?;

    // Receive response
    println!("\nWaiting for response...");
    while let Ok(Some(response)) = port.receive_string(Some(0xa), Some(deadline)) {
        println!("Got response:\n{response}");
    }

    Ok(())
}
