#![no_std]
//! Bounded, allocation-free primitives for game-specific wire decoders.
//!
//! This is the layer that deliberately remains `no_std`. Filesystem access,
//! archive traversal, image/audio decoding, the device runtime, and the strict
//! game transliteration do not inherit that constraint.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    UnexpectedEof { offset: usize, needed: usize },
    LengthOverflow,
    TrailingData { offset: usize, remaining: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub const fn position(&self) -> usize {
        self.offset
    }

    pub const fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    pub fn read_exact(&mut self, length: usize) -> Result<&'a [u8], DecodeError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(DecodeError::LengthOverflow)?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(DecodeError::UnexpectedEof {
                offset: self.offset,
                needed: length,
            })?;
        self.offset = end;
        Ok(value)
    }

    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.read_exact(1)?[0])
    }

    pub fn read_i8(&mut self) -> Result<i8, DecodeError> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_u16_be(&mut self) -> Result<u16, DecodeError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_be(&mut self) -> Result<i16, DecodeError> {
        Ok(self.read_u16_be()? as i16)
    }

    pub fn read_i32_be(&mut self) -> Result<i32, DecodeError> {
        let bytes = self.read_exact(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn finish(self) -> Result<(), DecodeError> {
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(DecodeError::TrailingData {
                offset: self.offset,
                remaining: self.remaining(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_big_endian_values_and_rejects_truncation() {
        let mut reader = Reader::new(&[0x7f, 0x80, 0x12, 0x34, 0xaa]);
        assert_eq!(reader.read_i8(), Ok(127));
        assert_eq!(reader.read_i8(), Ok(-128));
        assert_eq!(reader.read_u16_be(), Ok(0x1234));
        assert_eq!(
            reader.finish(),
            Err(DecodeError::TrailingData {
                offset: 4,
                remaining: 1
            })
        );

        let mut short = Reader::new(&[1]);
        assert_eq!(
            short.read_u16_be(),
            Err(DecodeError::UnexpectedEof {
                offset: 0,
                needed: 2
            })
        );
        assert_eq!(short.position(), 0);
    }
}
