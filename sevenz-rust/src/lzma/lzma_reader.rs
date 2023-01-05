use std::io::{Error, ErrorKind, Read, Result};
use std::ptr::NonNull;

use byteorder::{LittleEndian, ReadBytesExt};

use super::decoder::LZMADecoder;
use super::lz::LZDecoder;
use super::range_codec::RangeDecoder;
use super::*;

pub fn get_memery_usage_by_props(dict_size: u32, props_byte: u8) -> Result<u32> {
    if dict_size > DICT_SIZE_MAX {
        return Err(Error::new(ErrorKind::InvalidInput, "dict size too large"));
    }
    if props_byte > (4 * 5 + 4) * 9 + 8 {
        return Err(Error::new(ErrorKind::InvalidInput, "Invalid props byte"));
    }
    let props = props_byte % (9 * 5);
    let lp = props / 9;
    let lc = props - lp * 9;
    get_memery_usage(dict_size, lc as u32, lp as u32)
}
pub fn get_memery_usage(dict_size: u32, lc: u32, lp: u32) -> Result<u32> {
    if lc > 8 || lp > 4 {
        return Err(Error::new(ErrorKind::InvalidInput, "Invalid lc or lp"));
    }
    return Ok(10 + get_dict_size(dict_size)? / 1024 + ((2 * 0x300) << (lc + lp)) / 1024);
}

fn get_dict_size(dict_size: u32) -> Result<u32> {
    if dict_size > DICT_SIZE_MAX {
        return Err(Error::new(ErrorKind::InvalidInput, "dict size too large"));
    }
    let dict_size = dict_size.max(4096);
    Ok((dict_size + 15) & !15)
}
pub struct LZMAReader<R> {
    reader: UnsafeReader<R>,
    lz: NonNull<LZDecoder>,
    rc: NonNull<RangeDecoder<UnsafeReader<R>>>,
    lzma: LZMADecoder<UnsafeReader<R>>,
    end_reached: bool,
    relaxed_end_cond: bool,
    remaining_size: u64,
}

impl<R> Drop for LZMAReader<R> {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.lz.as_ptr());
            Box::from_raw(self.rc.as_ptr());
            self.reader.clone().release();
        }
    }
}

impl<R: Read> LZMAReader<R> {
    fn construct1(
        reader: R,
        uncomp_size: u64,
        mut props: u8,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        if props > (4 * 5 + 4) * 9 + 8 {
            return Err(Error::new(ErrorKind::InvalidInput, "Invalid props byte"));
        }
        let pb = props / (9 * 5);
        props -= pb * 9 * 5;
        let lp = props / 9;
        let lc = props - lp * 9;
        if dict_size > DICT_SIZE_MAX {
            return Err(Error::new(ErrorKind::InvalidInput, "dict size too large"));
        }
        Self::construct2(
            reader,
            uncomp_size,
            lc as _,
            lp as _,
            pb as _,
            dict_size,
            preset_dict,
        )
    }

    fn construct2(
        reader: R,
        uncomp_size: u64,
        lc: u32,
        lp: u32,
        pb: u32,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        if lc > 8 || lp > 4 || pb > 4 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Invalid lc or lp or pb",
            ));
        }
        let mut dict_size = get_dict_size(dict_size)?;
        if uncomp_size <= u64::MAX / 2 && dict_size as u64 > uncomp_size {
            dict_size = get_dict_size(uncomp_size as u32)?;
        }
        let reader = UnsafeReader::new(reader);
        let rc = RangeDecoder::new_stream(reader.clone());
        let rc = match rc {
            Ok(r) => r,
            Err(e) => {
                reader.release();
                return Err(e);
            }
        };
        let lz = LZDecoder::new(get_dict_size(dict_size)? as _, preset_dict);
        let lz = unsafe {
            let lz = Box::new(lz);
            let ptr = Box::into_raw(lz);
            NonNull::new_unchecked(ptr)
        };
        let rc = unsafe {
            let rc = Box::new(rc);
            let ptr = Box::into_raw(rc);
            NonNull::new_unchecked(ptr)
        };
        let lzma = LZMADecoder::new(lz, rc, lc, lp, pb);
        Ok(Self {
            reader,
            lz,
            rc,
            lzma,
            end_reached: false,
            relaxed_end_cond: true,
            remaining_size: uncomp_size,
        })
    }

    pub fn new_mem_limit(
        mut reader: R,
        mem_limit_kb: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        let props = reader.read_u8()?;
        let mut dict_size = reader.read_u32::<LittleEndian>()?;

        let uncomp_size = reader.read_u64::<LittleEndian>()?;
        let need_mem = get_memery_usage_by_props(dict_size, props)?;
        if mem_limit_kb < need_mem {
            return Err(Error::new(
                ErrorKind::OutOfMemory,
                format!(
                    "{}kb memery needed,but limit was {}kb",
                    need_mem, mem_limit_kb
                ),
            ));
        }
        Self::construct1(reader, uncomp_size, props, dict_size, preset_dict)
    }

    pub fn new_with_props(
        reader: R,
        uncomp_size: u64,
        props: u8,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        Self::construct1(reader, uncomp_size, props, dict_size, preset_dict)
    }

    pub fn new(
        reader: R,
        uncomp_size: u64,
        lc: u32,
        lp: u32,
        pb: u32,
        dict_size: u32,
        preset_dict: Option<&[u8]>,
    ) -> Result<Self> {
        Self::construct2(reader, uncomp_size, lc, lp, pb, dict_size, preset_dict)
    }

    fn read_decode(&mut self, buf: &mut [u8]) -> Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        if self.end_reached {
            return Ok(0);
        }
        let mut size = 0;
        let mut len = buf.len() as u32;
        let mut off = 0u32;
        while len > 0 {
            let mut copy_size_max = len as u32;
            if self.remaining_size <= u64::MAX / 2 && (self.remaining_size as u32) < len {
                copy_size_max = self.remaining_size as u32;
            }
            unsafe {
                self.lz.as_mut().set_limit(copy_size_max as usize);
            }

            match self.lzma.decode() {
                Ok(_) => {}
                Err(e) => {
                    if self.remaining_size != u64::MAX || !self.lzma.end_marker_detected() {
                        return Err(e);
                    }
                    self.end_reached = true;
                    unsafe {
                        self.rc.as_mut().normalize()?;
                    }
                }
            }

            let copied_size = unsafe { self.lz.as_mut() }.flush(buf, off as _) as u32;
            off += copied_size;
            len -= copied_size;
            size += copied_size;
            if self.remaining_size <= u64::MAX / 2 {
                self.remaining_size -= copied_size as u64;
                if self.remaining_size == 0 {
                    self.end_reached = true;
                }
            }

            if self.end_reached {
                let lz = unsafe { self.lz.as_ref() };
                let rc = unsafe { self.rc.as_ref() };
                if lz.has_pending() || (!self.relaxed_end_cond && !rc.is_stream_finished()) {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "end reached but not decoder finished",
                    ));
                }
                return Ok(size as _);
            }
        }
        Ok(size as _)
    }
}

impl<R: Read> Read for LZMAReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.read_decode(buf)
    }
}

struct UnsafeReader<R>(NonNull<R>);
impl<R> UnsafeReader<R> {
    fn new(r: R) -> Self {
        let r = Box::new(r);
        let ptr = Box::into_raw(r);
        unsafe { Self(NonNull::new_unchecked(ptr)) }
    }

    fn release(self) {
        unsafe { Box::from_raw(self.0.as_ptr()) };
    }
}
impl<R> Clone for UnsafeReader<R> {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<R: Read> Read for UnsafeReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        unsafe { self.0.as_mut().read(buf) }
    }
}
