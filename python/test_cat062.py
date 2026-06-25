"""Independent verification of libasterix's CAT-062 encoder.

The Rust example `examples/gen_cat062.rs` encodes a CAT-062 message and writes
it to `python/cat062.bin`. This test decodes that file with a completely
independent reference library (`libasterix` from the asterix-libs project,
https://github.com/zoranbosnjak/asterix-libs) and asserts the decoded values
match what the Rust side encoded.

Run:
    cargo run --example gen_cat062
    python/.venv/bin/pytest python/
"""

import pathlib

import pytest
from asterix import base, generated as g

CAT062 = g.Cat_062_1_20
BIN = pathlib.Path(__file__).resolve().parent / "cat062.bin"

# The values encoded by examples/gen_cat062.rs.
EXPECTED = {
    "sac": 1,
    "sic": 1,
    "track_number": 1234,
    "time_of_day": 12345.0,
    "latitude": 47.0,
    "longitude": 8.0,
    "altitude_ft": 35000.0,
    "vx": 150.0,
    "vy": -75.0,
    "callsign": "SWR123",
}


@pytest.fixture(scope="module")
def record():
    assert BIN.exists(), f"{BIN} missing — run `cargo run --example gen_cat062` first"
    raw = BIN.read_bytes()
    datablocks = base.RawDatablock.parse(base.Bits.from_bytes(raw))
    assert not isinstance(datablocks, ValueError), datablocks
    assert len(datablocks) == 1 and datablocks[0].get_category() == 62
    res = CAT062.cv_record.parse(base.ParsingMode.StrictParsing,
                                 datablocks[0].get_raw_records())
    assert not isinstance(res, ValueError), res
    rec, _rest = res
    return rec


def test_data_source_id(record):
    v = record.get_item("010").variation
    assert v.get_item("SAC").variation.content.as_uint() == EXPECTED["sac"]
    assert v.get_item("SIC").variation.content.as_uint() == EXPECTED["sic"]


def test_track_number(record):
    assert record.get_item("040").variation.content.as_uint() == EXPECTED["track_number"]


def test_time_of_track(record):
    t = float(record.get_item("070").variation.content.as_quantity("s"))
    assert t == pytest.approx(EXPECTED["time_of_day"], abs=1 / 128)  # LSB = 1/128 s


def test_position_wgs84(record):
    pos = record.get_item("105").variation
    lat = float(pos.get_item("LAT").variation.content.as_quantity("°"))
    lon = float(pos.get_item("LON").variation.content.as_quantity("°"))
    assert lat == pytest.approx(EXPECTED["latitude"], abs=1e-4)
    assert lon == pytest.approx(EXPECTED["longitude"], abs=1e-4)


def test_geometric_altitude(record):
    alt = float(record.get_item("130").variation.content.as_quantity("ft"))
    assert alt == pytest.approx(EXPECTED["altitude_ft"], abs=6.25)  # LSB = 6.25 ft


def test_velocity(record):
    vel = record.get_item("185").variation
    vx = float(vel.get_item("VX").variation.content.as_quantity("m/s"))
    vy = float(vel.get_item("VY").variation.content.as_quantity("m/s"))
    assert vx == pytest.approx(EXPECTED["vx"], abs=0.25)  # LSB = 0.25 m/s
    assert vy == pytest.approx(EXPECTED["vy"], abs=0.25)


def test_callsign(record):
    cs = record.get_item("245").variation.get_item("CHR").variation.content.as_string()
    assert cs.strip() == EXPECTED["callsign"]
