# libasterix

A Rust library for parsing and encoding [ASTERIX](https://www.eurocontrol.int/asterix) (All Purpose Structured Eurocontrol Surveillance Information Exchange) messages.

## Supported Categories

| Category | Description | Parse | Encode |
|----------|-------------|-------|--------|
| CAT-048  | Monoradar Target Reports | Yes | No |
| CAT-062  | System Track Data (SDPS) | Yes | Yes |

## Usage

```toml
[dependencies]
libasterix = "0.1"
```

### Parsing a CAT-062 message

```rust
use libasterix::asterix::cat062::{parse_cat062_block, Cat062};

let data: &[u8] = &[ /* raw ASTERIX bytes */ ];
let messages: Vec<Cat062> = parse_cat062_block(data)?;

for msg in &messages {
    if let Some(pos) = &msg.position_wgs84 {
        println!("Lat: {}, Lon: {}", pos.latitude, pos.longitude);
    }
    if let Some(id) = &msg.target_id {
        println!("Callsign: {}", id.target_id);
    }
}
```

### Encoding a CAT-062 message

```rust
use libasterix::asterix::cat062::{encode_cat062_block, Cat062Record};

let mut record = Cat062Record::new(1, 1); // SAC=1, SIC=1
record.latitude = 47.0;
record.longitude = 8.0;
record.altitude_ft = Some(35000.0);
record.callsign = Some("SWR123".to_string());

let bytes = encode_cat062_block(&[record]);
// Send `bytes` over UDP...
```

### Parsing a CAT-048 message

```rust
use libasterix::asterix::cat048::parse_cat048_block;

let data: &[u8] = &[ /* raw ASTERIX bytes */ ];
let messages = parse_cat048_block(data)?;

for msg in &messages {
    if let Some(fl) = &msg.flight_level {
        println!("Flight level: {}", fl.flight_level);
    }
}
```

## Sample Data

The `samples/` directory contains ASTERIX captures for testing:

- `cat062_sample.bin` - Raw CAT-062 binary
- `cat_034_048.pcap` - CAT-034/048 packet capture
- `cat_062_065.pcap` - CAT-062/065 packet capture

## License

MIT
