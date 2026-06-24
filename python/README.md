# Python interop tests

Independent verification of `libasterix`'s ASTERIX encoder against a separate
reference implementation. Everything Python-related lives in this `python/`
folder; the Rust side stays at the repo root.

We encode a message with **our** library (Rust) and decode it with an
**unrelated** reference library — [`libasterix`](https://github.com/zoranbosnjak/asterix-libs)
from the asterix-libs project (pure Python, spec-generated, no shared code with
this crate). If the reference decodes back the exact values we encoded, our
encoder is correct.

```
Cat062Record (Rust)  --our encoder-->  cat062.bin  --reference decoder-->  fields
                                                                              |
                                                            asserted == input values
```

## Setup

Run from the repo root:

```sh
python3 -m venv python/.venv
python/.venv/bin/pip install -r python/requirements.txt
```

## Run

```sh
cargo run --example gen_cat062          # Rust encodes -> python/cat062.bin
python/.venv/bin/pytest python/         # reference library decodes & verifies
```

## Coverage

`test_cat062.py` checks CAT-062 items the encoder emits: I062/010 (SAC/SIC),
070 (time of track), 105 (WGS-84 position), 130 (geometric altitude),
185 (velocity), 040 (track number), 245 (callsign).
