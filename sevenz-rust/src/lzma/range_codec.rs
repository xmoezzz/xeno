use byteorder::{BigEndian, ReadBytesExt};

use super::*;
use std::io::ErrorKind;
use std::io::{Read, Result};
use std::ptr::NonNull;

pub struct RangeDecoder<R> {
    inner: R,
    range: u32,
    code: u32,
}
impl RangeDecoder<RangeDecoderBuffer> {
    pub fn new_buffer(len: usize) -> Self {
        Self {
            inner: RangeDecoderBuffer::new(len - 5),
            code: 0,
            range: 0,
        }
    }
}

impl<R: Read> RangeDecoder<R> {
    pub fn new_stream(mut inner: R) -> Result<Self> {
        let b = inner.read_u8()?;
        if b != 0x00 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, ""));
        }
        let code = inner.read_u32::<BigEndian>()?;
        Ok(Self {
            inner,
            code,
            range: (0xFFFFFFFFu32),
        })
    }

    pub fn is_stream_finished(&self) -> bool {
        self.code == 0
    }
}

impl<R: Read> RangeDecoder<R> {
    pub fn normalize(&mut self) -> Result<()> {
        let mask = TOP_MASK;
        if self.range & mask == 0 {
            let b = self.inner.read_u8()? as u32;
            let code = ((self.code) << SHIFT_BITS) | b;
            self.code = code;
            let range = (self.range) << SHIFT_BITS;
            self.range = range;
        }
        Ok(())
    }

    pub fn decode_bit(&mut self, probs: &mut [u16], index: usize) -> Result<i32> {
        self.normalize()?;
        let prob = probs[index] as u32;
        let bound = (self.range >> (BIT_MODEL_TOTAL_BITS as i32)) * prob;
        let mask = 0x80000000u32;
        let cm = self.code ^ mask;
        let bm = bound ^ mask;
        if (cm as i32) < (bm as i32) {
            self.range = bound;
            probs[index] = (prob + (((BIT_MODEL_TOTAL).wrapping_sub(prob)) >> MOVE_BITS)) as u16;
            Ok(0)
        } else {
            self.range = self.range.wrapping_sub(bound);
            self.code = self.code.wrapping_sub(bound);
            probs[index] = (prob - (prob >> MOVE_BITS)) as u16;
            Ok(1)
        }
    }

    pub fn decode_bit_tree(&mut self, probs: &mut [u16]) -> Result<i32> {
        let mut symbol = 1;
        loop {
            symbol = (symbol << 1) | self.decode_bit(probs, symbol as usize)?;
            if symbol >= probs.len() as i32 {
                break;
            }
        }
        Ok(symbol - probs.len() as i32)
    }

    pub fn decode_reverse_bit_tree(&mut self, probs: &mut [u16]) -> Result<i32> {
        let mut symbol = 1;
        let mut i = 0;
        let mut result = 0;
        loop {
            let bit = self.decode_bit(probs, symbol as usize)?;
            symbol = (symbol << 1) | bit;
            result |= bit << i;
            i += 1;
            if symbol >= probs.len() as i32 {
                break;
            }
        }
        Ok(result as i32)
    }

    pub fn decode_direct_bits(&mut self, mut count: u32) -> Result<i32> {
        let mut result = 0;
        loop {
            self.normalize()?;
            self.range = self.range >> 1;
            let t = (self.code.wrapping_sub(self.range)) >> 31;
            self.code -= self.range & (t.wrapping_sub(1));
            result = (result << 1) | (1u32.wrapping_sub(t));
            count -= 1;
            if count == 0 {
                break;
            }
        }
        Ok(result as _)
    }
}

pub struct RangeDecoderBuffer {
    buf: Vec<u8>,
    pos: usize,
}
impl RangeDecoder<RangeDecoderBuffer> {
    pub fn prepare<R: Read>(&mut self, mut reader: R, len: usize) -> Result<()> {
        if len < 5 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "buffer len must >= 5",
            ));
        }

        let b = reader.read_u8()?;
        if b != 0x00 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "first byte is 0",
            ));
        }
        self.code = reader.read_u32::<BigEndian>()?;

        self.range = 0xFFFFFFFFu32;
        let len = len - 5;
        let pos = self.inner.buf.len() - len;
        let end = pos + len;
        self.inner.pos = pos;
        reader.read_exact(&mut self.inner.buf[pos..end])
    }

    #[inline]
    pub fn is_finished(&self) -> bool {
        self.inner.pos == self.inner.buf.len() && self.code == 0
    }

}

impl RangeDecoderBuffer {
    pub fn new(len: usize) -> Self {
        Self {
            buf: vec![0; len],
            pos: len,
        }
    }
}
impl Read for RangeDecoderBuffer {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if self.pos == self.buf.len() {
            return Ok(0);
        }
        let len = buf.len().min(self.buf.len() - self.pos);
        let range = self.pos..(self.pos + len);
        buf.copy_from_slice(&self.buf[range]);
        self.pos += len;
        Ok(len)
    }
}
