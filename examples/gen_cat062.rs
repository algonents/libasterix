//! Encode a CAT-062 message with libasterix and write it to `python/cat062.bin`.
//!
//! The companion test `python/test_cat062.py` decodes this file with an
//! independent reference library (`libasterix` from the asterix-libs project)
//! and asserts the values match what we encode here.
//!
//! Run with: `cargo run --example gen_cat062`

use libasterix::asterix::cat062::{encode_cat062_block, Cat062Record};

fn main() {
    let record = Cat062Record {
        sac: 1,
        sic: 1,
        track_number: 1234,
        time_of_day: 12345.0,
        latitude: 47.0,
        longitude: 8.0,
        altitude_ft: Some(35000),
        vx: Some(150.0),
        vy: Some(-75.0),
        icao_address: None,
        callsign: Some("SWR123".to_string()),
        track_status: 0,
    };

    let bytes = encode_cat062_block(&[record]);
    std::fs::write("python/cat062.bin", &bytes).expect("write python/cat062.bin");
    println!("wrote python/cat062.bin ({} bytes)", bytes.len());
}
