/// A cursor for writing binary data to a buffer.
/// Symmetric to the read Cursor.
pub struct WriteCursor {
    buf: Vec<u8>,
}

impl WriteCursor {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }

    pub fn position(&self) -> usize {
        self.buf.len()
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_u16_be(&mut self, v: u16) {
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
    }

    pub fn write_i16_be(&mut self, v: i16) {
        let bytes = v.to_be_bytes();
        self.buf.extend_from_slice(&bytes);
    }

    pub fn write_u24_be(&mut self, v: u32) {
        self.buf.push((v >> 16) as u8);
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
    }

    pub fn write_i24_be(&mut self, v: i32) {
        // Sign-extend to 24 bits
        let bytes = v.to_be_bytes();
        // Take the last 3 bytes (24 bits)
        self.buf.push(bytes[1]);
        self.buf.push(bytes[2]);
        self.buf.push(bytes[3]);
    }

    pub fn write_i32_be(&mut self, v: i32) {
        let bytes = v.to_be_bytes();
        self.buf.extend_from_slice(&bytes);
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Patch a u16 value at a specific position (big-endian).
    /// Used for writing length fields after the content is known.
    pub fn patch_u16_be(&mut self, pos: usize, v: u16) {
        self.buf[pos] = (v >> 8) as u8;
        self.buf[pos + 1] = v as u8;
    }
}

impl Default for WriteCursor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_u8() {
        let mut cursor = WriteCursor::new();
        cursor.write_u8(0x42);
        assert_eq!(cursor.as_slice(), &[0x42]);
    }

    #[test]
    fn test_write_u16_be() {
        let mut cursor = WriteCursor::new();
        cursor.write_u16_be(0x1234);
        assert_eq!(cursor.as_slice(), &[0x12, 0x34]);
    }

    #[test]
    fn test_write_i16_be() {
        let mut cursor = WriteCursor::new();
        cursor.write_i16_be(-1);
        assert_eq!(cursor.as_slice(), &[0xFF, 0xFF]);
    }

    #[test]
    fn test_write_u24_be() {
        let mut cursor = WriteCursor::new();
        cursor.write_u24_be(0x123456);
        assert_eq!(cursor.as_slice(), &[0x12, 0x34, 0x56]);
    }

    #[test]
    fn test_write_i24_be() {
        let mut cursor = WriteCursor::new();
        cursor.write_i24_be(-1);
        assert_eq!(cursor.as_slice(), &[0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn test_write_i32_be() {
        let mut cursor = WriteCursor::new();
        cursor.write_i32_be(0x12345678);
        assert_eq!(cursor.as_slice(), &[0x12, 0x34, 0x56, 0x78]);
    }

    #[test]
    fn test_patch_u16_be() {
        let mut cursor = WriteCursor::new();
        cursor.write_u8(0x3E); // CAT
        cursor.write_u16_be(0x0000); // Placeholder for length
        cursor.write_u8(0xAA); // Some data
        cursor.patch_u16_be(1, 0x0004); // Patch length
        assert_eq!(cursor.as_slice(), &[0x3E, 0x00, 0x04, 0xAA]);
    }
}
