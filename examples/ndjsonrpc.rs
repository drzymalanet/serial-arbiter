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
