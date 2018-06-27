extern crate onvif;
use std::thread;
use std::time::Duration;

fn main() {
  let probe_timeout = Duration::from_secs(3);
  let found_devices = onvif::start_probe(&probe_timeout).unwrap();
  for (i, found_device) in found_devices.iter().enumerate() {
    println!(
      "{}. Found '{}' at {:?}",
      i + 1,
      found_device.name(),
      found_device.xaddrs()
    );
  }
  thread::sleep(Duration::from_secs(3));
}
