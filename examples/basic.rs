extern crate onvif;
use std::thread;
use std::time::Duration;

fn main() {
  let probe_timeout = Duration::from_secs(3);
  onvif::start_probe(&probe_timeout);
  thread::sleep(Duration::from_secs(3));
}
