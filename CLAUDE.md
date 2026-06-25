# CLAUDE.md

Guidance for working in this repository.

## Project

`libasterix` is a Rust library for parsing and encoding [ASTERIX](https://www.eurocontrol.int/asterix)
surveillance messages.

- **CAT-048** — Monoradar Target Reports: parse only
- **CAT-062** — System Track Data (SDPS): parse and encode

Source layout:

- `src/asterix/cat048.rs`, `src/asterix/cat062.rs` — per-category parse/encode
- `src/asterix/cursor.rs` — byte reader used by parsers
- `src/asterix/write_cursor.rs` — byte writer used by encoders
- `samples/` — real ASTERIX captures (`.bin`, `.pcap`, `.pcapng`) for testing

## Build & test

```sh
cargo build
cargo test            # Rust unit tests (in cat048.rs, cat062.rs, write_cursor.rs)
```

## Interop tests (independent encoder verification)

We verify our encoder against a **separate, unrelated** reference library:
we encode a message with our Rust code and decode it with the pure-Python
[`libasterix`](https://github.com/zoranbosnjak/asterix-libs) (asterix-libs
project — spec-generated, no shared code with this crate). If the reference
decodes back exactly what we encoded, our encoder is correct.

```
Cat062Record (Rust) --our encoder--> tests/cat062.bin --reference decoder--> assert == input
```

Everything Python-related lives under `tests/`; the Rust side stays at the
repo root.

Files:

- `examples/gen_cat062.rs` — encodes a CAT-062 record, writes `tests/cat062.bin`
- `tests/test_cat062.py` — decodes that file with the reference lib, asserts each field
- `tests/requirements.txt` — pinned Python deps (`libasterix`, `pytest`)
- `tests/README.md` — details and field coverage

### Python environment

The Python runtime is the system interpreter (`/usr/bin/python3`, ≥ 3.10
required by the reference lib). Packages are isolated in a repo-local virtualenv
`tests/.venv/` (gitignored — not committed; recreate per machine). The
reference package installs as `libasterix` on PyPI but imports as `asterix`.

First-time setup (from the repo root):

```sh
python3 -m venv tests/.venv
tests/.venv/bin/pip install -r tests/requirements.txt
```

Run the interop suite:

```sh
cargo run --example gen_cat062          # Rust encodes -> tests/cat062.bin
tests/.venv/bin/pytest tests/         # reference library decodes & verifies
```

`tests/cat062.bin` is a generated artifact (gitignored); regenerate it with
the `cargo run` step above before running pytest.

### Notes

- The interop test currently validates the **encoder** only (bytes flow
  Rust → Python). Testing the parser independently needs the reverse direction
  (reference encodes → our `parse_cat062_block` decodes).
- Field assertion tolerances are set to each CAT-062 item's spec LSB
  (e.g. time 1/128 s, altitude 6.25 ft, velocity 0.25 m/s).
