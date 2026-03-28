use anyhow::{Context, Result, bail, ensure};
use serde::{Serialize, Serializer};
use crate::asterix::AsterixCategory;
use crate::asterix::cursor::Cursor;

fn serialize_fspec_hex<S>(fspec: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let hex: Vec<String> = fspec
        .iter()
        .map(|b| format!("0x{:02X}", b))
        .collect();

    hex.serialize(serializer)
}


#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cat048 {
    pub  category: AsterixCategory,
    pub length: u16,

    #[serde(serialize_with = "serialize_fspec_hex")]
    pub fspec: Vec<u8>,

    // I048/010
    pub data_source_id: Option<DataSourceId>,
    // I048/140
    pub time_of_track: Option<u32>,
    // I048/020
    pub track_type: Option<u8>,
    // I048/040
    pub track_position: Option<TrackPosition>,
    // I048/070
    pub mode_3a: Option<Mode3ACode>,
    // I048/090
    pub flight_level: Option<FlightLevel>,
    // I048/130
    pub radar_plot_characteristics: Option<RadarPlotCharacteristics>,
    // I048/220 (3 octets)
    pub aircraft_address: Option<u32>,
    pub aircraft_identification: Option<String>,
    // I048/250
    pub mode_s_mb_data: Option<ModeSMbData>,
    // I048/161
    pub track_number: Option<u16>,
    // I048/042
    pub calculated_position: Option<CartesianPosition>,
    // I048/200
    pub calculated_track_velocity: Option<TrackVelocityPolar>,
    pub track_status: Option<TrackStatus>,
    pub track_quality: Option<TrackQuality>,
    pub warning_error_conditions: Option<WarningErrorConditions>,
    pub mode_3a_confidence: Option<Mode3AConfidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DataSourceId {
    pub sac: u8,
    pub sic: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TrackPosition {
    pub range_nm: f64,
    pub bearing_deg: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Mode3ACode {
    pub v: bool,       // invalid
    pub g: bool,       // garbled
    pub l: bool,       // smoothed / lost (edition-dependent naming)
    pub code_raw: u16, // 12-bit code (0..0x0FFF)
}

impl Mode3ACode {
    /// Returns squawk as four octal digits packed into a number, e.g. 0406 -> 406.
    pub fn squawk_digits(&self) -> u16 {
        let c = self.code_raw & 0x0FFF;
        let a = ((c >> 9) & 0x7) as u16;
        let b = ((c >> 6) & 0x7) as u16;
        let d = ((c >> 3) & 0x7) as u16;
        let e = (c & 0x7) as u16;
        a * 1000 + b * 100 + d * 10 + e
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct FlightLevel {
    pub v: bool,
    pub g: bool,
    /// Flight level in quarter-units (FL * 4). Common representation in CAT048.
    pub fl_quarter: i16,
}

impl FlightLevel {
    pub fn as_fl(self) -> f32 {
        self.fl_quarter as f32 / 4.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RadarPlotCharacteristics {
    /// “Subfield FSPEC” octets for I048/130 (FX chained).
    pub subfield_fspec: Vec<u8>,

    /// Raw subfield values in the order they appear (one octet per present subfield).
    pub subfields: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeSMbBlock {
    pub data: [u8; 8],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModeSMbData {
    pub blocks: Vec<ModeSMbBlock>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CartesianPosition {
    pub x_nm: f32,
    pub y_nm: f32,
    pub x_raw: i16, // in 1/128 NM units
    pub y_raw: i16, // in 1/128 NM units
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct TrackVelocityPolar {
    pub ground_speed_raw: u16,
    pub heading_raw: u16,
    pub heading_deg: f32,
    pub ground_speed_nm_per_s: f32,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackStatus {
    /// Raw octets (variable length; FX = LSB).
    pub octets: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrackQuality {
    /// Raw octets (fixed length 4).
    pub octets: [u8; 4],

    // Optional convenience decodes (units depend on edition/UAP; keep raw as truth)
    pub sigx_raw: u8,
    pub sigy_raw: u8,
    pub sigv_raw: u8,
    pub siga_raw: u8,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WarningErrorConditions {
    /// Raw octets (variable length; FX = LSB).
    pub octets: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Mode3AConfidence {
    pub raw: u16,

    // Optional convenience flags (1 = low quality, 0 = high quality)
    pub qa4: bool,
    pub qa2: bool,
    pub qa1: bool,

    pub qb4: bool,
    pub qb2: bool,
    pub qb1: bool,

    pub qc4: bool,
    pub qc2: bool,
    pub qc1: bool,

    pub qd4: bool,
    pub qd2: bool,
    pub qd1: bool,
}


fn ia5_6bit_to_char(v: u8) -> char {
    match v {
        0 => ' ',                             // space
        1..=26 => (b'A' + (v - 1)) as char,   // A..Z
        48..=57 => (b'0' + (v - 48)) as char, // 0..9
        _ => ' ',                             // treat unknown as space (or '?' if you prefer)
    }
}

/// Decode I048/240 Aircraft Identification: 6 octets -> 8 IA-5 6-bit chars
fn decode_i048_240(bytes: [u8; 6]) -> String {
    // Concatenate to 48-bit stream, MSB-first
    let mut chars = [0u8; 8];

    // Extract 8 * 6-bit values from the 48 bits
    // bit index 0 is the MSB of bytes[0]
    for i in 0..8 {
        let bit_pos = i * 6;
        let mut val: u8 = 0;
        for j in 0..6 {
            let k = bit_pos + j;
            let byte_idx = k / 8;
            let bit_in_byte = 7 - (k % 8); // MSB-first
            let bit = (bytes[byte_idx] >> bit_in_byte) & 1;
            val = (val << 1) | (bit as u8);
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


// Parses a full CAT048 ASTERIX Data Block (CAT + LEN + payload of 1..N records)
pub fn parse_cat048_block(msg: &[u8]) -> Result<Vec<Cat048>> {
    ensure!(msg.len() >= 3, "CAT048 block too short (<3)");

    let cat = msg[0];
    ensure!(cat == 48, "expected CAT=48, got {cat}");

    let declared = u16::from_be_bytes([msg[1], msg[2]]) as usize;
    ensure!(declared >= 3, "invalid length {declared} (must be >= 3)");
    ensure!(
        msg.len() >= declared,
        "buffer shorter than declared length: declared={declared}, actual={}",
        msg.len()
    );

    let msg = &msg[..declared]; // respect declared length
    let mut payload = &msg[3..]; // records live here

    let mut out = Vec::new();
    while !payload.is_empty() {
        let before = payload.len();

        let (rec, consumed) = parse_cat048_record(payload)
            .with_context(|| format!("CAT048 record parse failed (remaining={})", payload.len()))?;

        ensure!(
            consumed <= payload.len(),
            "CAT048 record consumed beyond payload (consumed={consumed}, remaining={})",
            payload.len()
        );
        ensure!(consumed > 0, "CAT048 record parser made no progress");
        out.push(rec);

        payload = &payload[consumed..];

        // hard safety: must always make progress
        if payload.len() == before {
            bail!("CAT048 record parser made no progress (stuck loop)");
        }
    }
    Ok(out)
}



fn read_fx_chain(cur: &mut Cursor, what: &'static str) -> anyhow::Result<Vec<u8>> {

    let mut out = Vec::with_capacity(2);

    let mut oct = cur.read_u8().with_context(|| format!("{what} octet[1]"))?;
    out.push(oct);

    let mut i = 1usize;
    while (oct & 0x01) != 0 {
        i += 1;
        oct = cur.read_u8().with_context(|| format!("{what} octet[{i}]"))?;
        out.push(oct);

        // defensive cap
        if i > 16 {
            anyhow::bail!("{what}: FX chain too long (>16)");
        }
    }

    Ok(out)
}

// Parses exactly ONE CAT048 record (FSPEC + items) from the beginning of `input`.
// Returns (record, bytes_consumed).
pub fn parse_cat048_record(input: &[u8]) -> anyhow::Result<(Cat048, usize)> {
    use anyhow::{bail, ensure, Context, Result};

    let mut cur = Cursor::new(input);

    // ---- FSPEC chain (FX = LSB) ----
    let mut fspec = Vec::with_capacity(4);
    let mut oct = cur.read_u8().context("FSPEC1")?;
    fspec.push(oct);

    while (oct & 0x01) != 0 {
        oct = cur
            .read_u8()
            .with_context(|| format!("FSPEC{}", fspec.len() + 1))?;
        fspec.push(oct);

        if fspec.len() > 4 {
            bail!("FSPEC chain too long (>4) for this parser");
        }
    }

    let fspec1 = fspec[0];

    // --- your existing fields ---
    let mut data_source_id = None;
    let mut time_of_track = None;
    let mut track_type = None;
    let mut track_position = None;
    let mut mode_3a = None;
    let mut flight_level: Option<FlightLevel> = None;
    let mut radar_plot_characteristics: Option<RadarPlotCharacteristics> = None;
    let mut aircraft_address: Option<u32> = None;
    let mut track_number: Option<u16> = None;
    let mut mode_s_mb_data: Option<ModeSMbData> = None;
    let mut calculated_position: Option<CartesianPosition> = None;
    let mut calculated_track_velocity: Option<TrackVelocityPolar> = None;

    let mut track_status: Option<TrackStatus> = None; // I048/170
    let mut track_quality: Option<TrackQuality> = None; // I048/210
    let mut warning_error_conditions: Option<WarningErrorConditions> = None; // I048/030
    let mut mode_3a_confidence: Option<Mode3AConfidence> = None; // I048/080

    // I048/010
    if (fspec1 & 0x80) != 0 {
        let sac = cur.read_u8().context("I048/010 SAC")?;
        let sic = cur.read_u8().context("I048/010 SIC")?;
        data_source_id = Some(DataSourceId { sac, sic });
    }

    // I048/140
    if (fspec1 & 0x40) != 0 {
        time_of_track = Some(cur.read_u24_be().context("I048/140 time")?);
    }

    // I048/020 (variable length; FX chain in item)
    if (fspec1 & 0x20) != 0 {
        let mut octet = cur.read_u8().context("I048/020 first octet")?;
        track_type = Some(octet >> 5);
        while (octet & 0x01) != 0 {
            octet = cur.read_u8().context("I048/020 extension octet")?;
        }
    }

    // I048/040
    if (fspec1 & 0x10) != 0 {
        let range_raw = cur.read_u16_be().context("I048/040 range")?;
        let bearing_raw = cur.read_u16_be().context("I048/040 bearing")?;
        let range_nm = range_raw as f64 / 256.0;
        let bearing_deg = bearing_raw as f64 * 360.0 / 65536.0;
        track_position = Some(TrackPosition { range_nm, bearing_deg });
    }

    // I048/070
    if (fspec1 & 0x08) != 0 {
        let raw = cur.read_u16_be().context("I048/070 raw")?;
        let v = (raw & 0x8000) != 0;
        let g = (raw & 0x4000) != 0;
        let l = (raw & 0x2000) != 0;
        let code_raw = raw & 0x0FFF;
        mode_3a = Some(Mode3ACode { v, g, l, code_raw });
    }

    // I048/090
    if (fspec1 & 0x04) != 0 {
        let raw = cur.read_u16_be().context("I048/090 raw")?;
        let v = (raw & 0x8000) != 0;
        let g = (raw & 0x4000) != 0;
        let mut val = (raw & 0x3FFF) as i16;
        if (raw & 0x2000) != 0 {
            val |= !0x3FFF;
        }
        flight_level = Some(FlightLevel { v, g, fl_quarter: val });
    }

    // I048/130
    if (fspec1 & 0x02) != 0 {
        let mut subfield_fspec = Vec::with_capacity(2);
        let mut oct = cur.read_u8().context("I048/130 subfield-fspec[1]")?;
        subfield_fspec.push(oct);

        while (oct & 0x01) != 0 {
            oct = cur.read_u8().with_context(|| {
                format!("I048/130 subfield-fspec[{}]", subfield_fspec.len() + 1)
            })?;
            subfield_fspec.push(oct);
            if subfield_fspec.len() > 4 {
                bail!("I048/130 subfield FSPEC chain too long (>4)");
            }
        }

        let mut subfields = Vec::new();
        for (i, &sfs) in subfield_fspec.iter().enumerate() {
            for &mask in &[0x80u8, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02] {
                if (sfs & mask) != 0 {
                    let v = cur.read_u8().with_context(|| {
                        format!("I048/130 subfield value (octet {}, mask 0x{mask:02X})", i + 1)
                    })?;
                    subfields.push(v);
                }
            }
        }

        radar_plot_characteristics = Some(RadarPlotCharacteristics { subfield_fspec, subfields });
    }

    // ---- FSPEC2 ----
    let fspec2 = fspec.get(1).copied().unwrap_or(0);

    if (fspec2 & 0x80) != 0 {
        aircraft_address = Some(cur.read_u24_be().context("I048/220 aircraft address")?);
    }

    let mut aircraft_identification: Option<String> = None;
    if (fspec2 & 0x40) != 0 {
        let mut id = [0u8; 6];
        for i in 0..6 {
            id[i] = cur.read_u8().with_context(|| format!("I048/240 octet {}", i + 1))?;
        }
        aircraft_identification = Some(decode_i048_240(id));
    }

    if (fspec2 & 0x20) != 0 {
        let n = cur.read_u8().context("I048/250 REP")? as usize;
        let mut blocks = Vec::with_capacity(n);
        for i in 0..n {
            let mut b = [0u8; 8];
            for j in 0..8 {
                b[j] = cur.read_u8().with_context(|| {
                    format!("I048/250 block {}, octet {}", i + 1, j + 1)
                })?;
            }
            blocks.push(ModeSMbBlock { data: b });
        }
        mode_s_mb_data = Some(ModeSMbData { blocks });
    }

    if (fspec2 & 0x10) != 0 {
        track_number = Some(cur.read_u16_be().context("I048/161 track_number")?);
    }

    if (fspec2 & 0x08) != 0 {
        let x_raw = cur.read_i16_be().context("I048/042 X")?;
        let y_raw = cur.read_i16_be().context("I048/042 Y")?;
        calculated_position = Some(CartesianPosition {
            x_raw,
            y_raw,
            x_nm: x_raw as f32 / 128.0,
            y_nm: y_raw as f32 / 128.0,
        });
    }

    if (fspec2 & 0x04) != 0 {
        let ground_speed_raw = cur.read_u16_be().context("I048/200 speed")?;
        let heading_raw = cur.read_u16_be().context("I048/200 heading")?;
        let heading_deg = heading_raw as f32 * 360.0 / 65536.0;
        let ground_speed_nm_per_s = ground_speed_raw as f32 / 16384.0;
        calculated_track_velocity = Some(TrackVelocityPolar {
            ground_speed_raw,
            heading_raw,
            heading_deg,
            ground_speed_nm_per_s,
        });
    }

    // ✅ I048/170 Track Status (variable length; FX chain)
    if (fspec2 & 0x02) != 0 {
        let octets = read_fx_chain(&mut cur, "I048/170 Track Status")?;
        track_status = Some(TrackStatus { octets });
    }

    // ---- FSPEC3
    let fspec3 = fspec.get(2).copied().unwrap_or(0);

    // ✅ I048/210 Track Quality (fixed 4 octets)
    if (fspec3 & 0x80) != 0 {
        let mut o = [0u8; 4];
        for i in 0..4 {
            o[i] = cur.read_u8().with_context(|| format!("I048/210 octet {}", i + 1))?;
        }
        track_quality = Some(TrackQuality {
            octets: o,
            sigx_raw: o[0],
            sigy_raw: o[1],
            sigv_raw: o[2],
            siga_raw: o[3],
        });
    }

    // ✅ I048/030 Warning/Error Conditions and Target Classification (variable length; FX chain)
    if (fspec3 & 0x40) != 0 {
        let octets = read_fx_chain(&mut cur, "I048/030 Warning/Error Conditions")?;
        warning_error_conditions = Some(WarningErrorConditions { octets });
    }

    if (fspec3 & 0x20) != 0 {
        let raw = cur.read_u16_be().context("I048/080 raw")?;

        // Bits 16..13 are spare (0), bits 12..1 carry the Q flags. :contentReference[oaicite:2]{index=2}
        let bit = |n: u16| -> bool { (raw & (1u16 << (n - 1))) != 0 };

        mode_3a_confidence = Some(Mode3AConfidence {
            raw,

            // Mapping per spec: QA4 QA2 QA1 QB4 QB2 QB1 QC4 QC2 QC1 QD4 QD2 QD1 :contentReference[oaicite:3]{index=3}
            qa4: bit(12),
            qa2: bit(11),
            qa1: bit(10),

            qb4: bit(9),
            qb2: bit(8),
            qb1: bit(7),

            qc4: bit(6),
            qc2: bit(5),
            qc1: bit(4),

            qd4: bit(3),
            qd2: bit(2),
            qd1: bit(1),
        });
    }





    let consumed = cur.position() as usize;
    ensure!(consumed > 0, "CAT048 record parser made no progress");

    let record_len_u16: u16 = consumed.try_into().context("record too long for u16")?;

    Ok((
        Cat048 {
            category: AsterixCategory::Cat048,
            length: record_len_u16,
            fspec,
            data_source_id,
            time_of_track,
            track_type,
            track_position,
            mode_3a,
            flight_level,
            radar_plot_characteristics,
            aircraft_address,
            aircraft_identification,
            mode_s_mb_data,
            track_number,
            calculated_position,
            calculated_track_velocity,
            track_status,
            track_quality,
            warning_error_conditions,
            mode_3a_confidence,
        },
        consumed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub const CAT048_SAMPLE_1: &[u8; 49] = &[
        0x30, 0x00, 0x31, 0xf7, 0x5f, 0x8f, 0x00, 0x00, 0x01, 0x67, 0x3c, 0x9f, 0x20, 0x0c, 0x93,
        0x53, 0xbb, 0x01, 0x06, 0x08, 0xa7, 0x50, 0x14, 0x87, 0x15, 0x4b, 0x72, 0x00, 0x02, 0x05,
        0x90, 0xfd, 0x13, 0x05, 0x2e, 0x80, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x01, 0x06, 0x80,
        0x03, 0xbc, 0x00, 0x20,
    ];

    /*
    #[test]
    fn cat048_header_category_and_length() {
        let parsed = parse_cat048_block(CAT048_SAMPLE_1).unwrap();
        assert_eq!(parsed.category, AsterixCategory::Cat048);
        assert_eq!(parsed.length, 49);
    }

    #[test]
    fn cat048_fspec_extension_chain() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert_eq!(parsed.fspec.as_slice(), &[0xF7, 0x5f, 0x8f, 0x00]);
    }

    #[test]
    fn cat048_i048_010_data_source_identifier() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert_eq!(parsed.data_source_id, Some(DataSourceId { sac: 0, sic: 1 }));
    }

    #[test]
    fn cat048_i048_070_absent_in_sample() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        let fspec1 = parsed.fspec[0];
        assert_eq!(fspec1 & 0x08, 0);
        assert!(
            parsed.mode_3a.is_none(),
            "mode_3a must be None when FSPEC bit is clear"
        );
    }

    #[test]
    fn cat048_i048_040_measured_position_in_polar_coordinates() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();

        let epsilon = 1e-6;

        let expected = 12.574219_f64;
        assert!(
            (parsed.track_position.unwrap().range_nm - expected).abs() < epsilon,
            "Range mismatch: got {}, expected {}",
            parsed.track_position.unwrap().range_nm,
            expected
        );

        let expected = 117.7459716_f64;
        assert!(
            (parsed.track_position.unwrap().bearing_deg - expected).abs() < epsilon,
            "bearing mismatch: got {}, expected {}",
            parsed.track_position.unwrap().bearing_deg,
            expected
        );
    }

    #[test]
    fn cat048_i048_090_flight_level_binary_representation() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        let fl = parsed.flight_level.expect("I048/090 should be present");



        assert_eq!(fl.v, false);
        assert_eq!(fl.g, false);
        assert_eq!(fl.fl_quarter, 262);

        let expected_flight_level = 65.5;
        assert!((fl.as_fl() - expected_flight_level).abs() < 0.0001);
    }

    #[test]
    fn cat048_i048_220_aircraft_address_absent_in_sample() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert!(parsed.aircraft_address.is_none());
    }

    #[test]
    fn cat048_fspec2_i048_220_bit_clear_in_sample() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        let fspec2 = parsed.fspec.get(1).copied().unwrap_or(0);
        assert_eq!(fspec2 & 0x80, 0);
    }

    #[test]
    fn cat048_i048_240_aircraft_identification_target() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert_eq!(parsed.aircraft_identification.as_deref(), Some("TARGET 2"));
    }

    #[test]
    fn cat048_i048_250_mode_s_mb_data_absent_in_sample() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        let fspec2 = parsed.fspec.get(1).copied().unwrap_or(0);

        assert_eq!(fspec2 & 0x20, 0, "FSPEC2 bit 6 (0x20) must be clear");
        assert!(
            parsed.mode_s_mb_data.is_none(),
            "I048/250 must be None when FSPEC2 bit is clear"
        );
    }

    #[test]
    fn cat048_i048_161_track_number() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert_eq!(parsed.track_number, Some(2));
    }

    #[test]
    fn cat048_i048_042_calculated_position() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert!(parsed.calculated_position.is_some());
        assert_eq!(parsed.calculated_position.unwrap().x_nm, 11.125);
        assert_eq!(parsed.calculated_position.unwrap().y_nm, -5.8515625)
    }

    #[test]
    fn cat048_i048_200_calculated_position() {
        let parsed = parse_cat048_message(CAT048_SAMPLE_1).unwrap();
        assert!(parsed.calculated_track_velocity.is_some());
        assert_eq!(parsed.calculated_track_velocity.unwrap().heading_deg, 180.0);
        assert_eq!(
            parsed
                .calculated_track_velocity
                .unwrap()
                .ground_speed_nm_per_s,
            0.08093262
        );
    }
     */
}
