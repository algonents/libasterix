use anyhow::{ensure, Result};

pub struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        ensure!(self.remaining() >= 1, "unexpected EOF (need 1 byte)");
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }
    pub fn read_i16_be(&mut self) -> anyhow::Result<i16> {
        let hi = self.read_u8()?;
        let lo = self.read_u8()?;
        Ok(i16::from_be_bytes([hi, lo]))
    }

    pub fn read_u16_be(&mut self) -> Result<u16> {
        ensure!(self.remaining() >= 2, "unexpected EOF (need 2 bytes)");
        let b0 = self.buf[self.pos] as u16;
        let b1 = self.buf[self.pos + 1] as u16;
        self.pos += 2;
        Ok((b0 << 8) | b1)
    }

    pub fn read_u24_be(&mut self) -> Result<u32> {
        ensure!(self.remaining() >= 3, "unexpected EOF (need 3 bytes)");
        let b0 = self.buf[self.pos] as u32;
        let b1 = self.buf[self.pos + 1] as u32;
        let b2 = self.buf[self.pos + 2] as u32;
        self.pos += 3;
        Ok((b0 << 16) | (b1 << 8) | b2)
    }
}
