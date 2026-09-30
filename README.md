# onvif — Rust ONVIF client library

Work in progress, not even alpha yet.

## What works

- **WS-Discovery device probe** (`discovery::start_probe`): sends an ONVIF
  WS-Discovery probe over UDP multicast and collects `ProbeMatch` responses —
  URN, name, hardware, location, types, XAddrs and scopes — parsed with
  `quick-xml`.
- **Device model** (`device::OnvifDevice`): early builder-based type for
  addressing a device (XAddr + credentials).

## Usage

Discover ONVIF devices on the local network:

```bash
cargo run --example probe
```

```rust
let timeout = std::time::Duration::from_secs(3);
let found = onvif::start_probe(&timeout)?;
for d in &found {
    println!("{} at {:?}", d.name(), d.xaddrs());
}
```

## Development

```bash
cargo test -- --nocapture
```
