extern crate quick_xml;

use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashSet;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::Duration;

#[derive(Debug, Default, Hash, Eq, PartialEq)]
pub struct ProbeMatch {
  urn: String,
  name: String,
  hardware: String,
  location: String,
  types: Vec<String>,
  xaddrs: Vec<String>,
  scopes: Vec<String>,
}

fn read_message(socket: &UdpSocket) -> Result<String, io::Error> {
  let mut buf: [u8; 65_535] = [0; 65_535];
  // println!("Reading data");
  let result = socket.recv(&mut buf);
  result.map(move |_| std::str::from_utf8(&buf).unwrap().to_string())
}

fn parse_probe_match(xml: String) -> Result<ProbeMatch, String> {
  let mut reader = Reader::from_str(&xml);
  reader.trim_text(true);
  let mut buf = Vec::new();
  loop {
    match reader.read_event(&mut buf) {
      Ok(Event::End(ref e)) => {
        match e.name() {
          b"SOAP-ENV:Header" => break,
          _ => (),
        };
      }
      Ok(Event::Eof) => return Err("Finished before header".to_string()),
      Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
      _ => (),
    }
  }
  buf.clear();
  let mut buf2 = Vec::new();
  // Header ended
  let mut stack = Vec::<String>::new();
  let mut probe_match = ProbeMatch::default();

  loop {
    match reader.read_event(&mut buf2) {
      Ok(Event::Start(ref e)) => {
        let name = std::str::from_utf8(e.name()).unwrap().to_string();
        // println!("Pushing {}", name);
        stack.push(name);
      }
      Ok(Event::End(ref _e)) => {
        let ended_tag = stack.pop().expect("Stack can't be empty, but it was.");
        if ended_tag == "SOAP-ENV:Body" {
          break;
        }
        // println!("Popped: {}", ended_tag);
        // TODO: Verify what is popped is the tag that has ended
        // (std::str::from_utf8(e.name()).unwrap());
      }
      Ok(Event::Text(e)) => {
        let text = std::str::from_utf8(e.escaped())
          .expect("UTF decode error")
          .to_string();
        let tag = stack.get(stack.len() - 1).expect("Stack can't be empty");
        match tag.as_str() {
          "wsa:Address" => probe_match.urn = text,
          "d:Types" => probe_match.types = text.split(' ').map(|s| s.to_string()).collect(),
          "d:Scopes" => probe_match.scopes = text.split(' ').map(|s| s.to_string()).collect(),
          "d:XAddrs" => probe_match.xaddrs = text.split(' ').map(|s| s.to_string()).collect(),
          _ => {
            // println!("Ignoring text {}", text);
          }
        }
      }
      Ok(Event::Eof) => break,
      Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
      _ => (),
    }
  }
  for scope in &probe_match.scopes {
    if scope.starts_with("onvif://www.onvif.org/hardware/") {
      let parts: Vec<&str> = scope.split('/').collect();
      probe_match.hardware = (parts[parts.len() - 1]).to_string();
    } else if scope.starts_with("onvif://www.onvif.org/location/") {
      let parts: Vec<&str> = scope.split('/').collect();
      probe_match.location = (parts[parts.len() - 1]).to_string();
    } else if scope.starts_with("onvif://www.onvif.org/name/") {
      let parts: Vec<&str> = scope.split('/').collect();
      probe_match.name = (parts[parts.len() - 1]).to_string();
    }
  }
  Ok(probe_match)
}

pub fn start_probe(probe_duration: &Duration) -> Result<Vec<ProbeMatch>, io::Error> {
  let MULTICAST_ADDR: SocketAddr = "239.255.255.250:3702".parse().unwrap();
  println!("Started probe");
  let soap_tmpl = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
  <s:Envelope xmlns:s=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:a=\"http://schemas.xmlsoap.org/ws/2004/08/addressing\">
  <s:Header>
  <a:Action s:mustUnderstand=\"1\">
  http://schemas.xmlsoap.org/ws/2005/04/discovery/Probe</a:Action>
  <a:MessageID>uuid:__uuid__</a:MessageID>
  <a:ReplyTo><a:Address>http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous</a:Address>
  </a:ReplyTo><a:To s:mustUnderstand=\"1\">urn:schemas-xmlsoap-org:ws:2005:04:discovery</a:To>
  </s:Header><s:Body><Probe xmlns=\"http://schemas.xmlsoap.org/ws/2005/04/discovery\">
  <d:Types xmlns:d=\"http://schemas.xmlsoap.org/ws/2005/04/discovery\" xmlns:dp0=\"http://www.onvif.org/ver10/network/wsdl\">
  dp0:__type__</d:Types></Probe></s:Body>
  </s:Envelope>";
  let types = vec!["NetworkVideoTransmitter", "Device", "NetworkVideoDisplay"];
  let soap_reqs: Vec<String> = types
    .iter()
    .map(|device_type| {
      let s: &str = soap_tmpl.clone();
      let s = s.replace("__type__", device_type);
      let s = s.replace("__uuid__", "7c208633-8086-4a83-a9d4-b3fd8673b8f7"); //TODO: Replace hardcoded uuid with a generated one
      s
    })
    .collect();
  let all_interfaces = SocketAddr::from(([0, 0, 0, 0], 0));
  let socket = UdpSocket::bind(&all_interfaces).expect("Could not bind to udp socket");
  let read_socket = socket.try_clone().unwrap();
  read_socket
    .set_read_timeout(Some(Duration::from_secs(1)))
    .expect("set_read_timeout call failed");
  let (thread_stop_tx, thread_stop_rx) = mpsc::channel();
  let (devices_tx, devices_rx) = mpsc::channel();
  let _read_thread_handle = thread::spawn(move || {
    loop {
      if let Ok(message) = read_message(&read_socket) {
        devices_tx
          .send(parse_probe_match(message))
          .expect("Could not send found device over channel");
      }
      match thread_stop_rx.try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => {
          println!("Stopping receive thread");
          break;
        }
        Err(TryRecvError::Empty) => {}
      }
      //thread::sleep(Duration::from_millis(100));
    }
  });
  let _broadcast_thread = thread::spawn(move || {
    for _ in 1..5 {
      for soap_req in &soap_reqs {
        socket
          .send_to(soap_req.as_bytes(), &MULTICAST_ADDR)
          .expect("Could not send req");
        thread::sleep(Duration::from_millis(100));
      }
    }
  });
  //broadcast_thread.join();
  //read_thread_handle.join();
  let mut found_devices = HashSet::new();
  for _ in 1..10 {
    let dev = devices_rx.recv().unwrap().unwrap();
    found_devices.insert(dev);
  }
  thread::sleep(*probe_duration);
  let _ = thread_stop_tx.send(());

  Ok(found_devices.into_iter().collect())
}

#[cfg(test)]
mod tests {
  #[test]
  fn it_works() {
    assert_eq!(2 + 2, 4);
  }

  #[test]
  fn probe_xml_parse() {
    use super::*;
    use std::fs;
    use std::io::prelude::*;

    let mut fl = fs::File::open("src/resources/probe-discovery-response.xml").unwrap();
    let mut probe_discovery_response = String::new();
    fl.read_to_string(&mut probe_discovery_response)
      .expect("something went wrong reading the file");

    let probe_match = parse_probe_match(probe_discovery_response).unwrap();
    let expected = ProbeMatch {
      urn: "urn:uuid:a91b83ca-3388-7688-99aa-101806a776fb".to_string(),
      name: "NVT".to_string(),
      hardware: "IPC-model".to_string(),
      location: "china".to_string(),
      types: vec!["dn:NetworkVideoTransmitter".to_string()],
      xaddrs: vec!["http://192.168.1.70:8899/onvif/device_service".to_string()],
      scopes: vec![
        "onvif://www.onvif.org/type/video_encoder",
        "onvif://www.onvif.org/type/audio_encoder",
        "onvif://www.onvif.org/hardware/IPC-model",
        "onvif://www.onvif.org/location/country/china",
        "onvif://www.onvif.org/name/NVT",
        "onvif://www.onvif.org/Profile/Streaming",
      ].iter()
        .map(|s| s.to_string())
        .collect(),
    };
    assert_eq!(expected, probe_match);
  }
}
