//! ASTERIX CAT062 encoder and decoder
//!
//! CAT062 is used for SDPS (System Track Data) messages.
//! This module provides encoding and decoding for aircraft surveillance data.

use anyhow::{bail, ensure, Context, Result};
use serde::{Serialize, Serializer};

use super::cursor::Cursor;
use super::write_cursor::WriteCursor;
use super::AsterixCategory;

/// CAT062 category byte
pub const CAT062: u8 = 0x3E; // 62 decimal

fn serialize_fspec_hex<S>(fspec: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex: Vec<String> = fspec.iter().map(|b| format!("0x{:02X}", b)).collect();
    hex.serialize(serializer)
}

// ============================================================================
// Decoded CAT062 message structure
// ============================================================================

/// Decoded CAT062 message
#[derive(Debug, Clone, Serialize)]
pub struct Cat062 {
    pub category: AsterixCategory,
    pub length: u16,

    #[serde(serialize_with = "serialize_fspec_hex")]
    pub fspec: Vec<u8>,

    // I062/010 - Data Source Identifier
    pub data_source_id: Option<DataSourceId>,
    // I062/015 - Service Identification
    pub service_id: Option<u8>,
    // I062/070 - Time of Track Information
    pub time_of_track: Option<u32>,
    // I062/105 - Calculated Position in WGS-84
    pub position_wgs84: Option<PositionWGS84>,
    // I062/100 - Calculated Position Cartesian
    pub position_cartesian: Option<PositionCartesian>,
    // I062/185 - Calculated Track Velocity Cartesian
    pub velocity_cartesian: Option<VelocityCartesian>,
    // I062/210 - Calculated Acceleration
    pub acceleration: Option<Acceleration>,
    // I062/060 - Track Mode 3/A Code
    pub mode_3a: Option<Mode3ACode>,
    // I062/245 - Target Identification
    pub target_id: Option<TargetIdentification>,
    // I062/380 - Aircraft Derived Data (compound - stored as raw bytes)
    pub aircraft_derived_data: Option<Vec<u8>>,
    // I062/040 - Track Number
    pub track_number: Option<u16>,
    // I062/080 - Track Status
    pub track_status: Option<TrackStatus>,
    // I062/290 - System Track Update Ages (compound - stored as raw bytes)
    pub system_track_update_ages: Option<Vec<u8>>,
    // I062/200 - Mode of Movement
    pub mode_of_movement: Option<u8>,
    // I062/295 - Track Data Ages (compound - stored as raw bytes)
    pub track_data_ages: Option<Vec<u8>>,
    // I062/136 - Measured Flight Level
    pub measured_flight_level: Option<i16>,
    // I062/130 - Calculated Track Geometric Altitude
    pub geometric_altitude: Option<i16>,
    // I062/135 - Calculated Track Barometric Altitude
    pub barometric_altitude: Option<BarometricAltitude>,
    // I062/220 - Calculated Rate of Climb/Descent
    pub rate_of_climb: Option<i16>,
    // I062/390 - Flight Plan Related Data (compound - stored as raw bytes)
    pub flight_plan_data: Option<Vec<u8>>,
    // I062/270 - Target Size & Orientation
    pub target_size: Option<Vec<u8>>,
    // I062/300 - Vehicle Fleet Identification
    pub vehicle_fleet_id: Option<u8>,
    // I062/110 - Mode 5 Data / Extended Mode 1 Code (compound)
    pub mode_5_data: Option<Vec<u8>>,
    // I062/120 - Track Mode 2 Code
    pub mode_2_code: Option<u16>,
    // I062/510 - Composed Track Number (variable)
    pub composed_track_number: Option<Vec<u8>>,
    // I062/500 - Estimated Accuracies (compound)
    pub estimated_accuracies: Option<Vec<u8>>,
    // I062/340 - Measured Information (compound)
    pub measured_info: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DataSourceId {
    pub sac: u8,
    pub sic: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PositionWGS84 {
    pub latitude_raw: i32,
    pub longitude_raw: i32,
    pub latitude_deg: f64,
    pub longitude_deg: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PositionCartesian {
    pub x_raw: i32, // 24-bit signed, extended to i32
    pub y_raw: i32,
    pub x_m: f64,
    pub y_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct VelocityCartesian {
    pub vx_raw: i16,
    pub vy_raw: i16,
    pub vx_ms: f64,
    pub vy_ms: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Acceleration {
    pub ax_raw: i8,
    pub ay_raw: i8,
    pub ax_ms2: f64,
    pub ay_ms2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Mode3ACode {
    pub v: bool,      // Validated
    pub g: bool,      // Garbled
    pub ch: bool,     // Changed
    pub code_raw: u16, // 12-bit octal code
}

impl Mode3ACode {
    pub fn squawk_octal(&self) -> u16 {
        let c = self.code_raw & 0x0FFF;
        let a = ((c >> 9) & 0x7) as u16;
        let b = ((c >> 6) & 0x7) as u16;
        let d = ((c >> 3) & 0x7) as u16;
        let e = (c & 0x7) as u16;
        a * 1000 + b * 100 + d * 10 + e
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TargetIdentification {
    pub sti: u8, // Source of Target Identification
    pub callsign: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrackStatus {
    pub octets: Vec<u8>,
    // First octet decoded flags
    pub mon: bool,  // Monosensor track
    pub spi: bool,  // SPI present
    pub mrh: bool,  // Most Reliable Height
    pub src: u8,    // Source of altitude (3 bits)
    pub cnf: bool,  // Confirmed vs Tentative
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BarometricAltitude {
    pub qnh: bool,  // QNH correction applied
    pub altitude_raw: i16,
    pub altitude_fl: f64, // Flight Level (raw * 0.25)
}

// ============================================================================
// CAT062 record for encoding (simplified structure)
// ============================================================================

/// CAT062 record containing fields for encoding
#[derive(Debug, Clone)]
pub struct Cat062Record {
    /// I062/010 - Data Source Identifier (SAC/SIC)
    pub sac: u8,
    pub sic: u8,

    /// I062/040 - Track Number (12-bit)
    pub track_number: u16,

    /// I062/070 - Time of Track Information (seconds since midnight UTC)
    pub time_of_day: f64,

    /// I062/105 - Calculated Position in WGS-84 (degrees)
    pub latitude: f64,
    pub longitude: f64,

    /// I062/130 - Calculated Track Altitude (feet)
    pub altitude_ft: Option<i32>,

    /// I062/185 - Calculated Track Velocity (m/s, cartesian)
    pub vx: Option<f64>,
    pub vy: Option<f64>,

    /// I062/245 - Target Identification
    pub icao_address: Option<u32>, // 24-bit Mode-S address
    pub callsign: Option<String>,  // Up to 8 characters

    /// I062/080 - Track Status
    pub track_status: u8,
}

impl Cat062Record {
    pub fn new(sac: u8, sic: u8) -> Self {
        Self {
            sac,
            sic,
            track_number: 0,
            time_of_day: 0.0,
            latitude: 0.0,
            longitude: 0.0,
            altitude_ft: None,
            vx: None,
            vy: None,
            icao_address: None,
            callsign: None,
            track_status: 0,
        }
    }
}

// Conversion helpers

/// Convert latitude to CAT062 raw format
/// LSB = 180/2^25 degrees ≈ 5.364e-6 degrees
pub fn lat_to_raw(lat: f64) -> i32 {
    const LSB: f64 = 180.0 / (1u32 << 25) as f64;
    (lat / LSB) as i32
}

/// Convert longitude to CAT062 raw format
/// LSB = 180/2^25 degrees ≈ 5.364e-6 degrees
pub fn lon_to_raw(lon: f64) -> i32 {
    const LSB: f64 = 180.0 / (1u32 << 25) as f64;
    (lon / LSB) as i32
}

/// Convert altitude in feet to CAT062 raw format
/// LSB = 6.25 feet
pub fn altitude_to_raw(alt_ft: i32) -> i16 {
    const LSB: f64 = 6.25;
    (alt_ft as f64 / LSB) as i16
}

/// Convert velocity from m/s to CAT062 raw format
/// LSB = 0.25 m/s
pub fn velocity_to_raw(v_ms: f64) -> i16 {
    const LSB: f64 = 0.25;
    (v_ms / LSB) as i16
}

/// Convert polar velocity (speed m/s, heading degrees) to cartesian (vx, vy)
/// heading: 0 = North, 90 = East
pub fn velocity_to_cartesian(speed_ms: f64, heading_deg: f64) -> (f64, f64) {
    let heading_rad = heading_deg.to_radians();
    let vx = speed_ms * heading_rad.sin(); // East component
    let vy = speed_ms * heading_rad.cos(); // North component
    (vx, vy)
}

/// Convert time of day to CAT062 raw format
/// LSB = 1/128 second
pub fn time_to_raw(seconds_since_midnight: f64) -> u32 {
    (seconds_since_midnight * 128.0) as u32
}

/// Encode callsign to IA-5 6-bit format (up to 8 characters)
/// Returns 6 bytes (48 bits for 8 characters at 6 bits each)
pub fn encode_callsign(callsign: &str) -> [u8; 6] {
    let mut result = [0u8; 6];
    let chars: Vec<u8> = callsign
        .chars()
        .take(8)
        .map(|c| char_to_ia5(c))
        .collect();

    // Pack 8 6-bit values into 6 bytes
    // Byte 0: char0[5:0] << 2 | char1[5:4]
    // Byte 1: char1[3:0] << 4 | char2[5:2]
    // etc.

    let mut bits: u64 = 0;
    for (i, &ch) in chars.iter().enumerate() {
        bits |= (ch as u64) << (42 - i * 6);
    }
    // Pad remaining with spaces (0x20 in IA-5 = 32, but in 6-bit = 32)
    for i in chars.len()..8 {
        bits |= 32u64 << (42 - i * 6);
    }

    result[0] = (bits >> 40) as u8;
    result[1] = (bits >> 32) as u8;
    result[2] = (bits >> 24) as u8;
    result[3] = (bits >> 16) as u8;
    result[4] = (bits >> 8) as u8;
    result[5] = bits as u8;

    result
}

/// Convert ASCII character to IA-5 6-bit encoding
fn char_to_ia5(c: char) -> u8 {
    // IA-5 6-bit uses lower 6 bits of ASCII
    // Space = 0x20 = 32, A = 0x41 = 1, 0 = 0x30 = 48
    (c as u8) & 0x3F
}

/// Hash ICAO24 hex string to 12-bit track number
pub fn icao_to_track_number(icao24: &str) -> u16 {
    let addr = u32::from_str_radix(icao24, 16).unwrap_or(0);
    (addr & 0x0FFF) as u16
}

/// Parse ICAO24 hex string to 24-bit address
pub fn parse_icao_address(icao24: &str) -> Option<u32> {
    u32::from_str_radix(icao24, 16).ok()
}

/// Encode a single CAT062 record
///
/// FSPEC layout (items used):
/// FSPEC1: [I062/010][spare][spare][I062/070][I062/105][spare][I062/185][FX]
/// FSPEC2: [spare][spare][I062/245][spare][I062/040][I062/080][spare][FX]
/// FSPEC3: [spare][spare][spare][I062/130][spare][spare][spare][0]
pub fn encode_cat062_record(record: &Cat062Record) -> Vec<u8> {
    let mut cursor = WriteCursor::new();

    // Build FSPEC based on available data
    let has_velocity = record.vx.is_some() && record.vy.is_some();
    let has_target_id = record.icao_address.is_some() || record.callsign.is_some();
    let has_altitude = record.altitude_ft.is_some();
    // FSPEC byte 1: [I062/010][0][0][I062/070][I062/105][0][I062/185][FX]
    //               bit7      6  5  bit4      bit3      2  bit1      bit0
    let mut fspec1: u8 = 0;
    fspec1 |= 0x80; // I062/010 - always present (SAC/SIC)
    fspec1 |= 0x10; // I062/070 - Time of Track Information
    fspec1 |= 0x08; // I062/105 - Position
    if has_velocity {
        fspec1 |= 0x02; // I062/185 - Velocity
    }
    fspec1 |= 0x01; // FX - extension to FSPEC2

    // FSPEC byte 2: [0][0][I062/245][0][I062/040][I062/080][0][FX]
    //               7  6  bit5      4  bit3      bit2      1  bit0
    let mut fspec2: u8 = 0;
    if has_target_id {
        fspec2 |= 0x20; // I062/245 - Target Identification
    }
    fspec2 |= 0x08; // I062/040 - Track Number (always present)
    fspec2 |= 0x04; // I062/080 - Track Status (always present)
    if has_altitude {
        fspec2 |= 0x01; // FX - extension to FSPEC3
    }

    // FSPEC byte 3: [0][0][0][I062/130][0][0][0][0]
    //               7  6  5  bit4      3  2  1  0
    let fspec3: u8 = if has_altitude { 0x10 } else { 0x00 };

    // Write FSPEC
    cursor.write_u8(fspec1);
    cursor.write_u8(fspec2);
    if has_altitude {
        cursor.write_u8(fspec3);
    }

    // I062/010 - Data Source Identifier
    cursor.write_u8(record.sac);
    cursor.write_u8(record.sic);

    // I062/070 - Time of Track Information (3 bytes)
    let time_raw = time_to_raw(record.time_of_day) & 0xFFFFFF;
    cursor.write_u24_be(time_raw);

    // I062/105 - Calculated Position in WGS-84 (8 bytes)
    cursor.write_i32_be(lat_to_raw(record.latitude));
    cursor.write_i32_be(lon_to_raw(record.longitude));

    // I062/185 - Calculated Track Velocity Cartesian (4 bytes)
    if has_velocity {
        cursor.write_i16_be(velocity_to_raw(record.vx.unwrap()));
        cursor.write_i16_be(velocity_to_raw(record.vy.unwrap()));
    }

    // I062/245 - Target Identification (7 bytes)
    if has_target_id {
        // First byte: STI (bits 7-6) + spare (bits 5-0 are part of target id encoding)
        // STI = 00 (callsign from downlink)
        let sti = 0x00u8;
        cursor.write_u8(sti);

        // Next 6 bytes: Target identification (IA-5 encoded callsign)
        let callsign_bytes = encode_callsign(record.callsign.as_deref().unwrap_or(""));
        cursor.write_bytes(&callsign_bytes);
    }

    // I062/040 - Track Number (2 bytes)
    cursor.write_u16_be(record.track_number & 0x0FFF);

    // I062/080 - Track Status (variable, minimum 1 byte)
    // Simple status: just first byte with FX=0
    cursor.write_u8(record.track_status & 0xFE); // Clear FX bit

    // I062/130 - Calculated Track Geometric Altitude (2 bytes)
    if let Some(alt_ft) = record.altitude_ft {
        cursor.write_i16_be(altitude_to_raw(alt_ft));
    }

    cursor.into_inner()
}

/// Encode multiple CAT062 records into an ASTERIX data block
///
/// Format: CAT (1 byte) + LEN (2 bytes) + records
pub fn encode_cat062_block(records: &[Cat062Record]) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(1024);

    // Write CAT
    cursor.write_u8(CAT062);

    // Write placeholder for LEN (will patch later)
    let len_pos = cursor.position();
    cursor.write_u16_be(0);

    // Write all records
    for record in records {
        let record_bytes = encode_cat062_record(record);
        cursor.write_bytes(&record_bytes);
    }

    // Patch the length field
    let total_len = cursor.position() as u16;
    cursor.patch_u16_be(len_pos, total_len);

    cursor.into_inner()
}

// ============================================================================
// Decoder functions
// ============================================================================

/// Read FX-chained bytes (variable length field where bit 0 = extension indicator)
fn read_fx_chain(cur: &mut Cursor, what: &'static str) -> Result<Vec<u8>> {
    let mut out = Vec::with_capacity(2);
    let mut oct = cur.read_u8().with_context(|| format!("{what} octet[1]"))?;
    out.push(oct);

    let mut i = 1usize;
    while (oct & 0x01) != 0 {
        i += 1;
        oct = cur.read_u8().with_context(|| format!("{what} octet[{i}]"))?;
        out.push(oct);
        if i > 16 {
            bail!("{what}: FX chain too long (>16)");
        }
    }
    Ok(out)
}

/// Read a compound field with its own subfield presence indicators
/// (Generic version - kept for potential future use)
#[allow(dead_code)]
fn read_compound_field(cur: &mut Cursor, what: &'static str) -> Result<Vec<u8>> {
    let mut result = Vec::new();

    // Read primary subfield presence indicator(s)
    let indicators = read_fx_chain(cur, what)?;
    result.extend_from_slice(&indicators);

    // For each set bit in the indicators (excluding FX bits), read the subfield
    // This is a simplified approach - we just capture raw bytes
    // A full implementation would need subfield size definitions
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 {
                // Subfield present - but we don't know its size without the spec
                // For now, we'll just mark that it exists
            }
        }
    }

    Ok(result)
}

/// Decode IA-5 6-bit encoded callsign (6 bytes -> 8 characters)
fn decode_callsign(bytes: &[u8; 6]) -> String {
    let mut chars = [0u8; 8];

    // Extract 8 * 6-bit values from the 48 bits
    for i in 0..8 {
        let bit_pos = i * 6;
        let mut val: u8 = 0;
        for j in 0..6 {
            let k = bit_pos + j;
            let byte_idx = k / 8;
            let bit_in_byte = 7 - (k % 8);
            let bit = (bytes[byte_idx] >> bit_in_byte) & 1;
            val = (val << 1) | bit;
        }
        chars[i] = val;
    }

    chars
        .into_iter()
        .map(ia5_6bit_to_char)
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// Convert 6-bit IA-5 value to ASCII character
fn ia5_6bit_to_char(v: u8) -> char {
    match v {
        0 => ' ',
        1..=26 => (b'A' + (v - 1)) as char,
        48..=57 => (b'0' + (v - 48)) as char,
        _ => ' ',
    }
}

/// Read a signed 24-bit big-endian value, sign-extended to i32
fn read_i24_be(cur: &mut Cursor) -> Result<i32> {
    let raw = cur.read_u24_be().context("reading i24")?;
    // Sign extend from 24-bit to 32-bit
    if (raw & 0x800000) != 0 {
        Ok((raw | 0xFF000000) as i32)
    } else {
        Ok(raw as i32)
    }
}

/// Parse a CAT062 data block (CAT + LEN + 1..N records)
pub fn parse_cat062_block(msg: &[u8]) -> Result<Vec<Cat062>> {
    ensure!(msg.len() >= 3, "CAT062 block too short (<3)");

    let cat = msg[0];
    ensure!(cat == CAT062, "expected CAT=62 (0x3E), got 0x{cat:02X}");

    let declared = u16::from_be_bytes([msg[1], msg[2]]) as usize;
    ensure!(declared >= 3, "invalid length {declared} (must be >= 3)");
    ensure!(
        msg.len() >= declared,
        "buffer shorter than declared length: declared={declared}, actual={}",
        msg.len()
    );

    let msg = &msg[..declared];
    let mut payload = &msg[3..];

    let mut out = Vec::new();
    while !payload.is_empty() {
        let before = payload.len();

        let (rec, consumed) = parse_cat062_record(payload)
            .with_context(|| format!("CAT062 record parse failed (remaining={})", payload.len()))?;

        ensure!(
            consumed <= payload.len(),
            "CAT062 record consumed beyond payload"
        );
        ensure!(consumed > 0, "CAT062 record parser made no progress");

        out.push(rec);
        payload = &payload[consumed..];

        if payload.len() == before {
            bail!("CAT062 record parser stuck in loop");
        }
    }

    Ok(out)
}

/// Parse exactly ONE CAT062 record from the beginning of input.
/// Returns (record, bytes_consumed).
///
/// CAT062 UAP (User Application Profile):
/// FRN 1: I062/010 - Data Source Identifier (2 bytes)
/// FRN 2: spare
/// FRN 3: I062/015 - Service Identification (1 byte)
/// FRN 4: I062/070 - Time of Track Information (3 bytes)
/// FRN 5: I062/105 - Calculated Position WGS-84 (8 bytes)
/// FRN 6: I062/100 - Calculated Position Cartesian (6 bytes)
/// FRN 7: I062/185 - Calculated Track Velocity Cartesian (4 bytes)
/// FX
/// FRN 8: I062/210 - Calculated Acceleration (2 bytes)
/// FRN 9: I062/060 - Track Mode 3/A Code (2 bytes)
/// FRN 10: I062/245 - Target Identification (7 bytes)
/// FRN 11: I062/380 - Aircraft Derived Data (compound)
/// FRN 12: I062/040 - Track Number (2 bytes)
/// FRN 13: I062/080 - Track Status (1+ bytes, FX)
/// FRN 14: I062/290 - System Track Update Ages (compound)
/// FX
/// FRN 15: I062/200 - Mode of Movement (1 byte)
/// FRN 16: I062/295 - Track Data Ages (compound)
/// FRN 17: I062/136 - Measured Flight Level (2 bytes)
/// FRN 18: I062/130 - Calculated Track Geometric Altitude (2 bytes)
/// FRN 19: I062/135 - Calculated Track Barometric Altitude (2 bytes)
/// FRN 20: I062/220 - Calculated Rate of Climb/Descent (2 bytes)
/// FRN 21: I062/390 - Flight Plan Related Data (compound)
/// FX
/// ... and more
pub fn parse_cat062_record(input: &[u8]) -> Result<(Cat062, usize)> {
    let mut cur = Cursor::new(input);

    // Read FSPEC chain
    let mut fspec = Vec::with_capacity(4);
    let mut oct = cur.read_u8().context("FSPEC1")?;
    fspec.push(oct);

    while (oct & 0x01) != 0 {
        oct = cur
            .read_u8()
            .with_context(|| format!("FSPEC{}", fspec.len() + 1))?;
        fspec.push(oct);
        if fspec.len() > 7 {
            bail!("FSPEC chain too long (>7)");
        }
    }

    let fspec1 = fspec[0];
    let fspec2 = fspec.get(1).copied().unwrap_or(0);
    let fspec3 = fspec.get(2).copied().unwrap_or(0);
    let fspec4 = fspec.get(3).copied().unwrap_or(0);
    let _fspec5 = fspec.get(4).copied().unwrap_or(0);

    // Initialize all fields as None
    let mut data_source_id = None;
    let mut service_id = None;
    let mut time_of_track = None;
    let mut position_wgs84 = None;
    let mut position_cartesian = None;
    let mut velocity_cartesian = None;
    let mut acceleration = None;
    let mut mode_3a = None;
    let mut target_id = None;
    let mut aircraft_derived_data = None;
    let mut track_number = None;
    let mut track_status = None;
    let mut system_track_update_ages = None;
    let mut mode_of_movement = None;
    let mut track_data_ages = None;
    let mut measured_flight_level = None;
    let mut geometric_altitude = None;
    let mut barometric_altitude = None;
    let mut rate_of_climb = None;
    let mut flight_plan_data = None;
    let mut target_size = None;
    let mut vehicle_fleet_id = None;
    let mut mode_5_data = None;
    let mut mode_2_code = None;
    let mut composed_track_number = None;
    let mut estimated_accuracies = None;
    let mut measured_info = None;

    // ---- FSPEC1 ----

    // FRN 1: I062/010 - Data Source Identifier (2 bytes)
    if (fspec1 & 0x80) != 0 {
        let sac = cur.read_u8().context("I062/010 SAC")?;
        let sic = cur.read_u8().context("I062/010 SIC")?;
        data_source_id = Some(DataSourceId { sac, sic });
    }

    // FRN 2: spare (bit 0x40)

    // FRN 3: I062/015 - Service Identification (1 byte)
    if (fspec1 & 0x20) != 0 {
        service_id = Some(cur.read_u8().context("I062/015")?);
    }

    // FRN 4: I062/070 - Time of Track Information (3 bytes)
    if (fspec1 & 0x10) != 0 {
        time_of_track = Some(cur.read_u24_be().context("I062/070")?);
    }

    // FRN 5: I062/105 - Calculated Position WGS-84 (8 bytes)
    if (fspec1 & 0x08) != 0 {
        let lat_raw = cur.read_i16_be().context("I062/105 lat hi")?;
        let lat_lo = cur.read_u16_be().context("I062/105 lat lo")?;
        let lon_raw_hi = cur.read_i16_be().context("I062/105 lon hi")?;
        let lon_lo = cur.read_u16_be().context("I062/105 lon lo")?;

        let latitude_raw = ((lat_raw as i32) << 16) | (lat_lo as i32);
        let longitude_raw = ((lon_raw_hi as i32) << 16) | (lon_lo as i32);

        const LSB: f64 = 180.0 / (1u32 << 25) as f64;
        position_wgs84 = Some(PositionWGS84 {
            latitude_raw,
            longitude_raw,
            latitude_deg: latitude_raw as f64 * LSB,
            longitude_deg: longitude_raw as f64 * LSB,
        });
    }

    // FRN 6: I062/100 - Calculated Position Cartesian (6 bytes)
    if (fspec1 & 0x04) != 0 {
        let x_raw = read_i24_be(&mut cur).context("I062/100 X")?;
        let y_raw = read_i24_be(&mut cur).context("I062/100 Y")?;
        // LSB = 0.5 meters
        position_cartesian = Some(PositionCartesian {
            x_raw,
            y_raw,
            x_m: x_raw as f64 * 0.5,
            y_m: y_raw as f64 * 0.5,
        });
    }

    // FRN 7: I062/185 - Calculated Track Velocity Cartesian (4 bytes)
    if (fspec1 & 0x02) != 0 {
        let vx_raw = cur.read_i16_be().context("I062/185 Vx")?;
        let vy_raw = cur.read_i16_be().context("I062/185 Vy")?;
        // LSB = 0.25 m/s
        velocity_cartesian = Some(VelocityCartesian {
            vx_raw,
            vy_raw,
            vx_ms: vx_raw as f64 * 0.25,
            vy_ms: vy_raw as f64 * 0.25,
        });
    }

    // ---- FSPEC2 ----

    // FRN 8: I062/210 - Calculated Acceleration (2 bytes)
    if (fspec2 & 0x80) != 0 {
        let ax_raw = cur.read_u8().context("I062/210 Ax")? as i8;
        let ay_raw = cur.read_u8().context("I062/210 Ay")? as i8;
        // LSB = 0.25 m/s^2
        acceleration = Some(Acceleration {
            ax_raw,
            ay_raw,
            ax_ms2: ax_raw as f64 * 0.25,
            ay_ms2: ay_raw as f64 * 0.25,
        });
    }

    // FRN 9: I062/060 - Track Mode 3/A Code (2 bytes)
    if (fspec2 & 0x40) != 0 {
        let raw = cur.read_u16_be().context("I062/060")?;
        mode_3a = Some(Mode3ACode {
            v: (raw & 0x8000) != 0,
            g: (raw & 0x4000) != 0,
            ch: (raw & 0x2000) != 0,
            code_raw: raw & 0x0FFF,
        });
    }

    // FRN 10: I062/245 - Target Identification (7 bytes)
    if (fspec2 & 0x20) != 0 {
        let sti_byte = cur.read_u8().context("I062/245 STI")?;
        let sti = (sti_byte >> 6) & 0x03;
        let mut callsign_bytes = [0u8; 6];
        for i in 0..6 {
            callsign_bytes[i] = cur.read_u8().with_context(|| format!("I062/245 byte {}", i + 1))?;
        }
        target_id = Some(TargetIdentification {
            sti,
            callsign: decode_callsign(&callsign_bytes),
        });
    }

    // FRN 11: I062/380 - Aircraft Derived Data (compound, variable)
    if (fspec2 & 0x10) != 0 {
        aircraft_derived_data = Some(read_compound_i062_380(&mut cur)?);
    }

    // FRN 12: I062/040 - Track Number (2 bytes)
    if (fspec2 & 0x08) != 0 {
        track_number = Some(cur.read_u16_be().context("I062/040")?);
    }

    // FRN 13: I062/080 - Track Status (1+ bytes, FX chain)
    if (fspec2 & 0x04) != 0 {
        let octets = read_fx_chain(&mut cur, "I062/080")?;
        let first = octets[0];
        track_status = Some(TrackStatus {
            octets: octets.clone(),
            mon: (first & 0x80) != 0,
            spi: (first & 0x40) != 0,
            mrh: (first & 0x20) != 0,
            src: (first >> 2) & 0x07,
            cnf: (first & 0x02) != 0,
        });
    }

    // FRN 14: I062/290 - System Track Update Ages (compound)
    if (fspec2 & 0x02) != 0 {
        system_track_update_ages = Some(read_compound_i062_290(&mut cur)?);
    }

    // ---- FSPEC3 ----

    // FRN 15: I062/200 - Mode of Movement (1 byte)
    if (fspec3 & 0x80) != 0 {
        mode_of_movement = Some(cur.read_u8().context("I062/200")?);
    }

    // FRN 16: I062/295 - Track Data Ages (compound)
    if (fspec3 & 0x40) != 0 {
        track_data_ages = Some(read_compound_i062_295(&mut cur)?);
    }

    // FRN 17: I062/136 - Measured Flight Level (2 bytes)
    if (fspec3 & 0x20) != 0 {
        measured_flight_level = Some(cur.read_i16_be().context("I062/136")?);
    }

    // FRN 18: I062/130 - Calculated Track Geometric Altitude (2 bytes)
    if (fspec3 & 0x10) != 0 {
        geometric_altitude = Some(cur.read_i16_be().context("I062/130")?);
    }

    // FRN 19: I062/135 - Calculated Track Barometric Altitude (2 bytes)
    if (fspec3 & 0x08) != 0 {
        let raw = cur.read_u16_be().context("I062/135")?;
        let qnh = (raw & 0x8000) != 0;
        let _alt_raw = ((raw & 0x7FFF) as i16) - if (raw & 0x4000) != 0 { 0x4000 } else { 0 };
        barometric_altitude = Some(BarometricAltitude {
            qnh,
            altitude_raw: (raw & 0x7FFF) as i16,
            altitude_fl: (raw & 0x3FFF) as f64 * 0.25,
        });
    }

    // FRN 20: I062/220 - Calculated Rate of Climb/Descent (2 bytes)
    if (fspec3 & 0x04) != 0 {
        rate_of_climb = Some(cur.read_i16_be().context("I062/220")?);
    }

    // FRN 21: I062/390 - Flight Plan Related Data (compound)
    if (fspec3 & 0x02) != 0 {
        flight_plan_data = Some(read_compound_i062_390(&mut cur)?);
    }

    // ---- FSPEC4 ----

    // FRN 22: I062/270 - Target Size & Orientation (1+ bytes)
    if (fspec4 & 0x80) != 0 {
        target_size = Some(read_fx_chain(&mut cur, "I062/270")?);
    }

    // FRN 23: I062/300 - Vehicle Fleet Identification (1 byte)
    if (fspec4 & 0x40) != 0 {
        vehicle_fleet_id = Some(cur.read_u8().context("I062/300")?);
    }

    // FRN 24: I062/110 - Mode 5 Data / Extended Mode 1 Code (compound)
    if (fspec4 & 0x20) != 0 {
        mode_5_data = Some(read_compound_i062_110(&mut cur)?);
    }

    // FRN 25: I062/120 - Track Mode 2 Code (2 bytes)
    if (fspec4 & 0x10) != 0 {
        mode_2_code = Some(cur.read_u16_be().context("I062/120")?);
    }

    // FRN 26: I062/510 - Composed Track Number (variable, 3+ bytes)
    if (fspec4 & 0x08) != 0 {
        composed_track_number = Some(read_compound_i062_510(&mut cur)?);
    }

    // FRN 27: I062/500 - Estimated Accuracies (compound)
    if (fspec4 & 0x04) != 0 {
        estimated_accuracies = Some(read_compound_i062_500(&mut cur)?);
    }

    // FRN 28: I062/340 - Measured Information (compound)
    if (fspec4 & 0x02) != 0 {
        measured_info = Some(read_compound_i062_340(&mut cur)?);
    }

    // ---- FSPEC5+ ----
    // Additional items can be added as needed

    let consumed = cur.position();
    ensure!(consumed > 0, "CAT062 record parser made no progress");

    Ok((
        Cat062 {
            category: AsterixCategory::Cat062,
            length: consumed as u16,
            fspec,
            data_source_id,
            service_id,
            time_of_track,
            position_wgs84,
            position_cartesian,
            velocity_cartesian,
            acceleration,
            mode_3a,
            target_id,
            aircraft_derived_data,
            track_number,
            track_status,
            system_track_update_ages,
            mode_of_movement,
            track_data_ages,
            measured_flight_level,
            geometric_altitude,
            barometric_altitude,
            rate_of_climb,
            flight_plan_data,
            target_size,
            vehicle_fleet_id,
            mode_5_data,
            mode_2_code,
            composed_track_number,
            estimated_accuracies,
            measured_info,
        },
        consumed,
    ))
}

// ============================================================================
// Compound field readers
// ============================================================================

/// I062/380 - Aircraft Derived Data (compound with many subfields)
fn read_compound_i062_380(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();

    // Read primary subfield indicators (FX-chained)
    let indicators = read_fx_chain(cur, "I062/380 indicators")?;
    result.extend_from_slice(&indicators);

    // Subfield sizes for I062/380 (in order of FRN):
    // SF1: ADR (3), SF2: ID (6), SF3: MHG (2), SF4: IAS (2), SF5: TAS (2), SF6: SAL (2), SF7: FSS (2)
    // SF8: TIS (2+), SF9: TID (variable), SF10: COM (2), SF11: SAB (2+), SF12: ACS (7), SF13: BVR (2)
    // SF14: GVR (2), SF15: RAN (2), SF16: TAR (2), SF17: TAN (2), SF18: GSP (2), SF19: VUN (1)
    // SF20: MET (8), SF21: EMC (1), SF22: POS (6), SF23: GAL (2), SF24: PUN (1), SF25: MB (variable)
    // SF26: IAR (2), SF27: MAC (2), SF28: BPS (2)

    let subfield_sizes: &[Option<usize>] = &[
        Some(3),  // SF1: ADR
        Some(6),  // SF2: ID
        Some(2),  // SF3: MHG
        Some(2),  // SF4: IAS
        Some(2),  // SF5: TAS
        Some(2),  // SF6: SAL
        Some(2),  // SF7: FSS
        None,     // SF8: TIS (variable - 1+)
        None,     // SF9: TID (variable - REP)
        Some(2),  // SF10: COM
        None,     // SF11: SAB (variable - 2+)
        Some(7),  // SF12: ACS
        Some(2),  // SF13: BVR
        Some(2),  // SF14: GVR
        Some(2),  // SF15: RAN
        Some(2),  // SF16: TAR
        Some(2),  // SF17: TAN
        Some(2),  // SF18: GSP
        Some(1),  // SF19: VUN
        Some(8),  // SF20: MET
        Some(1),  // SF21: EMC
        Some(6),  // SF22: POS
        Some(2),  // SF23: GAL
        Some(1),  // SF24: PUN
        None,     // SF25: MB (variable - REP)
        Some(2),  // SF26: IAR
        Some(2),  // SF27: MAC
        Some(2),  // SF28: BPS
    ];

    let mut sf_idx = 0;
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 && sf_idx < subfield_sizes.len() {
                match subfield_sizes[sf_idx] {
                    Some(size) => {
                        for _ in 0..size {
                            result.push(cur.read_u8().with_context(|| {
                                format!("I062/380 SF{} byte", sf_idx + 1)
                            })?);
                        }
                    }
                    None => {
                        // Variable length subfield - read FX chain or REP field
                        match sf_idx {
                            7 => {
                                // TIS: 1+ bytes, FX
                                let data = read_fx_chain(cur, "I062/380/TIS")?;
                                result.extend_from_slice(&data);
                            }
                            8 => {
                                // TID: REP + items
                                let rep = cur.read_u8().context("I062/380/TID REP")?;
                                result.push(rep);
                                for _ in 0..rep {
                                    for _ in 0..15 {
                                        result.push(cur.read_u8().context("I062/380/TID item")?);
                                    }
                                }
                            }
                            10 => {
                                // SAB: 2+ bytes, FX
                                let data = read_fx_chain(cur, "I062/380/SAB")?;
                                // SAB has fixed 2 bytes then FX chain
                                result.push(cur.read_u8().context("I062/380/SAB byte2")?);
                                result.extend_from_slice(&data);
                            }
                            24 => {
                                // MB: REP + 8-byte blocks
                                let rep = cur.read_u8().context("I062/380/MB REP")?;
                                result.push(rep);
                                for _ in 0..rep {
                                    for _ in 0..8 {
                                        result.push(cur.read_u8().context("I062/380/MB block")?);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            sf_idx += 1;
        }
    }

    Ok(result)
}

/// I062/290 - System Track Update Ages (compound)
fn read_compound_i062_290(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let indicators = read_fx_chain(cur, "I062/290 indicators")?;
    result.extend_from_slice(&indicators);

    // All subfields in I062/290 are 1 byte each
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 {
                result.push(cur.read_u8().context("I062/290 subfield")?);
            }
        }
    }

    Ok(result)
}

/// I062/295 - Track Data Ages (compound)
fn read_compound_i062_295(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let indicators = read_fx_chain(cur, "I062/295 indicators")?;
    result.extend_from_slice(&indicators);

    // All subfields in I062/295 are 1 byte each
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 {
                result.push(cur.read_u8().context("I062/295 subfield")?);
            }
        }
    }

    Ok(result)
}

/// I062/390 - Flight Plan Related Data (compound)
fn read_compound_i062_390(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let indicators = read_fx_chain(cur, "I062/390 indicators")?;
    result.extend_from_slice(&indicators);

    // Subfield sizes for I062/390:
    let subfield_sizes: &[usize] = &[
        2,  // SF1: TAG
        7,  // SF2: CSN
        4,  // SF3: IFI
        1,  // SF4: FCT
        4,  // SF5: TAC
        4,  // SF6: WTC
        4,  // SF7: DEP
        4,  // SF8: DST
        3,  // SF9: RDS
        2,  // SF10: CFL
        2,  // SF11: CTL
        6,  // SF12: TOD (variable - this is simplified)
        4,  // SF13: AST
        1,  // SF14: STS
        7,  // SF15: STD
        7,  // SF16: STA
        2,  // SF17: PEM
        6,  // SF18: PEC
    ];

    let mut sf_idx = 0;
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 && sf_idx < subfield_sizes.len() {
                for _ in 0..subfield_sizes[sf_idx] {
                    result.push(cur.read_u8().with_context(|| {
                        format!("I062/390 SF{} byte", sf_idx + 1)
                    })?);
                }
            }
            sf_idx += 1;
        }
    }

    Ok(result)
}

/// I062/110 - Mode 5 Data / Extended Mode 1 Code (compound)
fn read_compound_i062_110(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let indicators = read_fx_chain(cur, "I062/110 indicators")?;
    result.extend_from_slice(&indicators);

    // Subfield sizes for I062/110:
    let subfield_sizes: &[usize] = &[
        1,  // SF1: SUM
        4,  // SF2: PMN
        6,  // SF3: POS
        2,  // SF4: GA
        2,  // SF5: EM1
        2,  // SF6: TOS
        2,  // SF7: XP
    ];

    let mut sf_idx = 0;
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 && sf_idx < subfield_sizes.len() {
                for _ in 0..subfield_sizes[sf_idx] {
                    result.push(cur.read_u8().with_context(|| {
                        format!("I062/110 SF{} byte", sf_idx + 1)
                    })?);
                }
            }
            sf_idx += 1;
        }
    }

    Ok(result)
}

/// I062/510 - Composed Track Number (variable, 3+ bytes)
fn read_compound_i062_510(cur: &mut Cursor) -> Result<Vec<u8>> {
    // First 3 bytes are master track, then FX-extended system track numbers
    let mut result = Vec::new();

    let b1 = cur.read_u8().context("I062/510 byte 1")?;
    let b2 = cur.read_u8().context("I062/510 byte 2")?;
    let b3 = cur.read_u8().context("I062/510 byte 3")?;
    result.push(b1);
    result.push(b2);
    result.push(b3);

    // If FX bit is set, read more 3-byte blocks
    while (result.last().copied().unwrap_or(0) & 0x01) != 0 {
        for _ in 0..3 {
            result.push(cur.read_u8().context("I062/510 extension")?);
        }
        if result.len() > 30 {
            bail!("I062/510 too long");
        }
    }

    Ok(result)
}

/// I062/500 - Estimated Accuracies (compound)
fn read_compound_i062_500(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let indicators = read_fx_chain(cur, "I062/500 indicators")?;
    result.extend_from_slice(&indicators);

    // Subfield sizes for I062/500:
    let subfield_sizes: &[usize] = &[
        4,  // SF1: APC
        2,  // SF2: COV
        4,  // SF3: APW
        1,  // SF4: AGA
        2,  // SF5: ABA
        2,  // SF6: ATV
        2,  // SF7: AA
        2,  // SF8: ARC
    ];

    let mut sf_idx = 0;
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 && sf_idx < subfield_sizes.len() {
                for _ in 0..subfield_sizes[sf_idx] {
                    result.push(cur.read_u8().with_context(|| {
                        format!("I062/500 SF{} byte", sf_idx + 1)
                    })?);
                }
            }
            sf_idx += 1;
        }
    }

    Ok(result)
}

/// I062/340 - Measured Information (compound)
fn read_compound_i062_340(cur: &mut Cursor) -> Result<Vec<u8>> {
    let mut result = Vec::new();
    let indicators = read_fx_chain(cur, "I062/340 indicators")?;
    result.extend_from_slice(&indicators);

    // Subfield sizes for I062/340 (per EUROCONTROL CAT062 spec):
    let subfield_sizes: &[usize] = &[
        2,  // SF1: SID (Sensor Identification)
        4,  // SF2: POS (Measured Position)
        2,  // SF3: HEI (Measured 3-D Height)
        2,  // SF4: MDC (Last Measured Mode C Code)
        2,  // SF5: MDA (Last Measured Mode 3/A Code)
        1,  // SF6: TYP (Report Type)
    ];

    let mut sf_idx = 0;
    for &ind in &indicators {
        for bit in (1..8).rev() {
            if (ind & (1 << bit)) != 0 && sf_idx < subfield_sizes.len() {
                for _ in 0..subfield_sizes[sf_idx] {
                    result.push(cur.read_u8().with_context(|| {
                        format!("I062/340 SF{} byte", sf_idx + 1)
                    })?);
                }
            }
            sf_idx += 1;
        }
    }

    Ok(result)
}

// ============================================================================
// Conversion helpers for decoded values
// ============================================================================

/// Convert raw time of track (LSB = 1/128 s) to seconds since midnight
pub fn raw_to_time_seconds(raw: u32) -> f64 {
    raw as f64 / 128.0
}

/// Convert raw geometric altitude (LSB = 6.25 ft) to feet
pub fn raw_to_altitude_feet(raw: i16) -> f64 {
    raw as f64 * 6.25
}

/// Convert raw rate of climb (LSB = 6.25 ft/min) to ft/min
pub fn raw_to_roc_fpm(raw: i16) -> f64 {
    raw as f64 * 6.25
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lat_to_raw() {
        // Equator
        assert_eq!(lat_to_raw(0.0), 0);
        // ~45 degrees
        let raw = lat_to_raw(45.0);
        assert!(raw > 0);
    }

    #[test]
    fn test_lon_to_raw() {
        assert_eq!(lon_to_raw(0.0), 0);
        let raw = lon_to_raw(-122.0); // West longitude
        assert!(raw < 0);
    }

    #[test]
    fn test_altitude_to_raw() {
        // 35000 feet / 6.25 = 5600
        assert_eq!(altitude_to_raw(35000), 5600);
    }

    #[test]
    fn test_velocity_to_cartesian() {
        // 100 m/s heading north
        let (vx, vy) = velocity_to_cartesian(100.0, 0.0);
        assert!(vx.abs() < 0.01);
        assert!((vy - 100.0).abs() < 0.01);

        // 100 m/s heading east
        let (vx, vy) = velocity_to_cartesian(100.0, 90.0);
        assert!((vx - 100.0).abs() < 0.01);
        assert!(vy.abs() < 0.01);
    }

    #[test]
    fn test_time_to_raw() {
        // 1 second = 128 LSB
        assert_eq!(time_to_raw(1.0), 128);
        // Noon = 43200 seconds = 5529600
        assert_eq!(time_to_raw(43200.0), 5529600);
    }

    #[test]
    fn test_encode_callsign() {
        let encoded = encode_callsign("BAW123");
        // Should be 6 bytes
        assert_eq!(encoded.len(), 6);
    }

    #[test]
    fn test_icao_to_track_number() {
        let track = icao_to_track_number("ABC123");
        assert!(track <= 0x0FFF);
    }

    #[test]
    fn test_encode_cat062_record() {
        let mut record = Cat062Record::new(0x01, 0x02);
        record.track_number = 123;
        record.time_of_day = 43200.0; // Noon
        record.latitude = 51.5;
        record.longitude = -0.1;
        record.altitude_ft = Some(35000);
        record.vx = Some(100.0);
        record.vy = Some(50.0);
        record.callsign = Some("TEST123".to_string());

        let bytes = encode_cat062_record(&record);

        // Check it starts with FSPEC
        assert!(!bytes.is_empty());
        // FSPEC1 should have FX set
        assert_eq!(bytes[0] & 0x01, 0x01);
    }

    #[test]
    fn test_encode_cat062_block() {
        let record = Cat062Record::new(0x01, 0x02);
        let bytes = encode_cat062_block(&[record]);

        // Should start with CAT062 (0x3E)
        assert_eq!(bytes[0], CAT062);

        // Length should be at least header (3) + minimal record
        let len = ((bytes[1] as u16) << 8) | (bytes[2] as u16);
        assert!(len >= 3);
        assert_eq!(len as usize, bytes.len());
    }

    #[test]
    fn test_decode_callsign() {
        // Test round-trip encoding/decoding
        let original = "TEST123";
        let encoded = encode_callsign(original);
        let decoded = decode_callsign(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_decode_callsign_padding() {
        // Short callsign should be padded and trimmed
        let original = "BA";
        let encoded = encode_callsign(original);
        let decoded = decode_callsign(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_raw_to_time_seconds() {
        // 128 raw = 1 second
        assert!((raw_to_time_seconds(128) - 1.0).abs() < 0.001);
        // Noon
        assert!((raw_to_time_seconds(5529600) - 43200.0).abs() < 0.001);
    }

    #[test]
    fn test_raw_to_altitude_feet() {
        // 5600 raw = 35000 feet
        assert!((raw_to_altitude_feet(5600) - 35000.0).abs() < 0.1);
    }

    #[test]
    fn test_encode_decode_roundtrip() {
        // Create a record, encode it, decode it, verify fields match
        let mut record = Cat062Record::new(0x01, 0x02);
        record.track_number = 123;
        record.time_of_day = 43200.5;
        record.latitude = 51.5074;
        record.longitude = -0.1278;

        let block = encode_cat062_block(&[record.clone()]);
        let decoded = parse_cat062_block(&block).expect("decode should succeed");

        assert_eq!(decoded.len(), 1);
        let dec = &decoded[0];

        assert_eq!(dec.data_source_id, Some(DataSourceId { sac: 0x01, sic: 0x02 }));
        assert_eq!(dec.track_number, Some(123));

        // Time should be close (encoding loses some precision)
        let time_sec = raw_to_time_seconds(dec.time_of_track.unwrap());
        assert!((time_sec - 43200.5).abs() < 0.01);

        // Position should be close
        let pos = dec.position_wgs84.unwrap();
        assert!((pos.latitude_deg - 51.5074).abs() < 0.0001);
        assert!((pos.longitude_deg - (-0.1278)).abs() < 0.0001);
    }

    #[test]
    fn test_parse_real_cat062_sample() {
        // Load sample extracted from cat_062_065.pcap
        let sample = include_bytes!("../../samples/cat062_sample.bin");

        // Verify header
        assert_eq!(sample[0], CAT062, "Expected CAT062 (0x3E)");
        let declared_len = u16::from_be_bytes([sample[1], sample[2]]) as usize;
        assert_eq!(declared_len, sample.len(), "Declared length should match file size");

        // Parse the block
        let records = parse_cat062_block(sample).expect("Failed to parse CAT062 sample");

        // Should have parsed at least one record
        assert!(!records.is_empty(), "Expected at least one record");

        println!("Parsed {} CAT062 records from sample", records.len());

        // Print details of each record for inspection
        for (i, rec) in records.iter().enumerate() {
            println!("\n--- Record {} ---", i + 1);
            if let Some(ref ds) = rec.data_source_id {
                println!("  Data Source: SAC={} SIC={}", ds.sac, ds.sic);
            }
            if let Some(t) = rec.time_of_track {
                println!("  Time of Track: {} seconds", raw_to_time_seconds(t));
            }
            if let Some(ref pos) = rec.position_wgs84 {
                println!("  Position WGS84: lat={:.6} lon={:.6}", pos.latitude_deg, pos.longitude_deg);
            }
            if let Some(ref vel) = rec.velocity_cartesian {
                println!("  Velocity: vx={:.2} m/s, vy={:.2} m/s", vel.vx_ms, vel.vy_ms);
            }
            if let Some(tn) = rec.track_number {
                println!("  Track Number: {}", tn);
            }
            if let Some(ref tid) = rec.target_id {
                println!("  Target ID: '{}' (STI={})", tid.callsign, tid.sti);
            }
            if let Some(ref m3a) = rec.mode_3a {
                println!("  Mode 3/A: {:04} (V={} G={} CH={})",
                    m3a.squawk_octal(), m3a.v, m3a.g, m3a.ch);
            }
            if let Some(alt) = rec.geometric_altitude {
                println!("  Geometric Altitude: {:.0} ft", raw_to_altitude_feet(alt));
            }
            if let Some(fl) = rec.measured_flight_level {
                println!("  Measured FL: {:.2}", fl as f64 * 0.25);
            }
        }

        // Verify record count
        assert_eq!(records.len(), 2, "Expected 2 records in sample");

        // ===== Verify Record 1 =====
        let r1 = &records[0];

        // Data Source ID
        assert_eq!(r1.data_source_id, Some(DataSourceId { sac: 25, sic: 100 }));

        // Service ID (I062/015)
        assert_eq!(r1.service_id, Some(1));

        // Time of Track (I062/070) - raw value 5865907 = 45827.3984375 seconds
        assert_eq!(r1.time_of_track, Some(5865907));

        // Position WGS-84 (I062/105)
        let pos1 = r1.position_wgs84.as_ref().expect("Record 1 should have position");
        assert!((pos1.latitude_deg - 41.167123).abs() < 0.0001, "Record 1 latitude mismatch");
        assert!((pos1.longitude_deg - 15.708867).abs() < 0.0001, "Record 1 longitude mismatch");

        // Velocity Cartesian (I062/185) - vx=915 raw (228.75 m/s), vy=-189 raw (-47.25 m/s)
        let vel1 = r1.velocity_cartesian.as_ref().expect("Record 1 should have velocity");
        assert_eq!(vel1.vx_raw, 915);
        assert_eq!(vel1.vy_raw, -189);

        // Track Number (I062/040)
        assert_eq!(r1.track_number, Some(4713));

        // Mode 3/A (I062/060) - raw 701 = squawk 1275 octal
        let m3a1 = r1.mode_3a.as_ref().expect("Record 1 should have Mode 3/A");
        assert_eq!(m3a1.code_raw, 701);

        // Geometric Altitude (I062/130) - raw 5837 = 36481.25 ft
        assert_eq!(r1.geometric_altitude, Some(5837));

        // Measured Flight Level (I062/136) - raw 1560 = FL390
        assert_eq!(r1.measured_flight_level, Some(1560));

        // ===== Verify Record 2 =====
        let r2 = &records[1];

        // Data Source ID
        assert_eq!(r2.data_source_id, Some(DataSourceId { sac: 25, sic: 100 }));

        // Position WGS-84 (I062/105)
        let pos2 = r2.position_wgs84.as_ref().expect("Record 2 should have position");
        assert!((pos2.latitude_deg - 41.416939).abs() < 0.0001, "Record 2 latitude mismatch");
        assert!((pos2.longitude_deg - 19.389136).abs() < 0.0001, "Record 2 longitude mismatch");

        // Velocity Cartesian (I062/185)
        let vel2 = r2.velocity_cartesian.as_ref().expect("Record 2 should have velocity");
        assert_eq!(vel2.vx_raw, -835);
        assert_eq!(vel2.vy_raw, -15);

        // Track Number (I062/040)
        assert_eq!(r2.track_number, Some(6831));

        // Geometric Altitude (I062/130) - raw 6773 = 42331.25 ft
        assert_eq!(r2.geometric_altitude, Some(6773));

        // Measured Flight Level (I062/136) - raw 1520 = FL380
        assert_eq!(r2.measured_flight_level, Some(1520));
    }
}
