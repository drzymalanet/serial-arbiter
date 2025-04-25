use std::{thread, time::Duration};

use serial_arbiter::*;

fn main() {
    let port = Arbiter::new();

    while port.open("/dev/ttyACM0").is_err() {
        println!("Waiting for connection... Please plug in the device.");
        thread::sleep(Duration::from_secs(1));
    }

    while port.receive(None, None).is_ok() {
        println!("Connected! Now unplug the device...");
        thread::sleep(Duration::from_secs(1));
    }

    while port.receive(None, None).is_err() {
        println!("Disconnected. Now plug the device back in...");
        thread::sleep(Duration::from_secs(1));
    }

    println!("Device detected. Test finished.");
}
