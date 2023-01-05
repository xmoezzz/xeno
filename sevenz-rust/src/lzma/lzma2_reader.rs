use super::{
    decoder::LZMADecoder,
    lz::LZDecoder,
    range_codec::{RangeDecoder, RangeDecoderBuffer},
};
use byteorder::{self, BigEndian, ReadBytesExt};
use std::{
    io::{ErrorKind, Read, Result},
    ptr::NonNull,
};
pub const COMPRESSED_SIZE_MAX: u32 = 1 << 16;

pub struct LZMA2Reader<R> {
    inner: R,
    lz: NonNull<LZDecoder>,
    rc: NonNull<RangeDecoder<RangeDecoderBuffer>>,
    lzma: Option<LZMADecoder<RangeDecoderBuffer>>,
    uncompressed_size: usize,
    is_lzma_chunk: bool,
    need_dict_reset: bool,
    need_props: bool,
    end_reached: bool,
    error: Option<std::io::Error>,
}
#[inline]
pub fn get_memery_usage(dict_size: u32) -> u32 {
    40 + COMPRESSED_SIZE_MAX / 1024 + get_dict_size(dict_size) / 1024
}

#[inline]
fn get_dict_size(dict_size: u32) -> u32 {
    dict_size + 15 & !15
}

impl<R: Read> LZMA2Reader<R> {
    pub fn new(inner: R, dict_size: u32, preset_dict: Option<&[u8]>) -> Self {
        let has_preset = preset_dict.as_ref().map(|a| a.len() > 0).unwrap_or(false);
        let lz = unsafe {
            let lz = LZDecoder::new(get_dict_size(dict_size) as _, preset_dict);
            let ptr = Box::into_raw(Box::new(lz));
            NonNull::new_unchecked(ptr)
        };
        let rc = unsafe {
            let lz = RangeDecoder::new_buffer(COMPRESSED_SIZE_MAX as _);
            let ptr = Box::into_raw(Box::new(lz));
            NonNull::new_unchecked(ptr)
        };
        Self {
            inner,
            lz,
            rc,
            lzma: None,
            uncompressed_size: 0,
            is_lzma_chunk: false,
            need_dict_reset: !has_preset,
            need_props: true,
            end_reached: false,
            error: None,
        }
    }

    fn decode_chunk_header(&mut self) -> Result<()> {
        let control = self.inner.read_u8()?;
        if control == 0x00 {
            self.end_reached = true;
            return Ok(());
        }

        if control >= 0xE0 || control == 0x01 {
            self.need_props = true;
            self.need_dict_reset = false;
            unsafe {
                self.lz.as_mut().reset();
            }
        } else if self.need_dict_reset {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Corrupted input data (LZMA2:0)",
            ));
        }
        if control >= 0x80 {
            self.is_lzma_chunk = true;
            self.uncompressed_size = ((control & 0x1F) as usize) << 16;
            self.uncompressed_size += self.inner.read_u16::<BigEndian>()? as usize + 1;
            let compressed_size = self.inner.read_u16::<BigEndian>()? as usize + 1;
            if control >= 0xC0 {
                self.need_props = false;
                self.decode_props()?;
            } else if self.need_props {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidInput,
                    "Corrupted input data (LZMA2:1)",
                ));
            } else if control >= 0xA0 {
                self.lzma.as_mut().map(|l| l.reset());
            }
            unsafe { &mut *self.rc.as_ptr() }.prepare(&mut self.inner, compressed_size)?;
        } else if control > 0x02 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "Corrupted input data (LZMA2:2)"));
        } else {
            self.is_lzma_chunk = false;
            self.uncompressed_size = (self.inner.read_u16::<BigEndian>()? + 1) as _;
        }
        Ok(())
    }

    fn decode_props(&mut self) -> std::io::Result<()> {
        let props = self.inner.read_u8()?;
        if props > (4 * 5 + 4) * 9 + 8 {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                "Corrupted input data (LZMA2:3)",
            ));
        }
        let pb = props / (9 * 5);
        let props = props - pb * 9 * 5;
        let lp = props / 9;
        let lc = props - lp * 9;
        if lc + lp > 4 {
            return Err(std::io::Error::new(ErrorKind::InvalidInput, "Corrupted input data (LZMA2:4)"));
        }
        self.lzma = Some(LZMADecoder::new(
            self.lz, self.rc, lc as _, lp as _, pb as _,
        ));

        Ok(())
    }

    fn read_decode(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if let Some(e) = &self.error {
            return Err(std::io::Error::new(e.kind(), e.to_string()));
        }

        if self.end_reached {
            return Ok(0);
        }
        let mut size = 0;
        let mut len = buf.len();
        let mut off = 0;
        while len > 0 {
            if self.uncompressed_size == 0 {
                self.decode_chunk_header()?;
                if self.end_reached {
                    return Ok(size);
                }
            }

            let copy_size_max = self.uncompressed_size.min(len);
            if !self.is_lzma_chunk {
                unsafe { &mut *self.lz.as_ptr() }
                    .copy_uncompressed(&mut self.inner, copy_size_max)?;
            } else {
                unsafe { self.lz.as_mut() }.set_limit(copy_size_max);
                if let Some(lzma) = self.lzma.as_mut() {
                    lzma.decode()?;
                }
            }

            unsafe {
                let copied_size = self.lz.as_mut().flush(buf, off);
                off += copied_size;
                len -= copied_size;
                size += copied_size;
                self.uncompressed_size -= copied_size;
                if self.uncompressed_size == 0 {
                    if !self.rc.as_ref().is_finished() || self.lz.as_ref().has_pending() {
                        return Err(std::io::Error::new(
                            ErrorKind::InvalidInput,
                            "rc not finished or lz has pending",
                        ));
                    }
                }
            }
        }
        Ok(size)
    }
}

impl<R: Read> Read for LZMA2Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self.read_decode(buf) {
            Ok(size) => Ok(size),
            Err(e) => {
                let error = std::io::Error::new(e.kind(), e.to_string());
                self.error = Some(e);
                return Err(error);
            }
        }
    }
}

impl<R> Drop for LZMA2Reader<R> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.lz.as_ptr());
            Box::from_raw(self.rc.as_ptr());
        }
    }
}
