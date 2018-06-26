extern crate onvif;
use std::thread;
use std::time::Duration;

fn main() {
  let probe_timeout = Duration::from_secs(3);
  let found_devices = onvif::start_probe(&probe_timeout).unwrap();
  for found_device in found_devices.iter() {
    println!("{:?}", found_device);
  }
  thread::sleep(Duration::from_secs(3));
}
