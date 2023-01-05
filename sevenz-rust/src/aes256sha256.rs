use std::{
    fmt::Write,
    io::{Read, Seek},
};

use aes::cipher::{
    block_padding::{Pkcs7, UnpadError},
    generic_array::GenericArray,
    typenum::U32,
    BlockDecryptMut, Iv, Key, KeyIvInit,
};
use sha2::{digest::FixedOutput, Digest};

use crate::folder::Coder;

type Aes256Cbc = cbc::Decryptor<aes::Aes256>;

pub struct Aes256Sha256Decoder<R> {
    cipher: Cipher,
    input: R,
    done: bool,
    ibuffer: [u8; 512],
    obuffer: Vec<u8>,
    ostart: usize,
    ofinish: usize,
    closed: bool,
    pos: usize,
}

impl<R: Read> Aes256Sha256Decoder<R> {
    pub fn new(input: R, coder: &Coder, passworld: &[u8]) -> Result<Self, crate::Error> {
        if coder.properties.len() < 2 {
            return Err(crate::Error::other("AES256 properties too shart"));
        }
        let b0 = coder.properties[0];
        let num_cycles_power = b0 & 63;
        let b1 = coder.properties[1];
        let iv_size = ((b0 >> 6 & 1) + (b1 & 15)) as usize;
        let salt_size = ((b0 >> 7 & 1) + (b1 >> 4)) as usize;
        if 2 + salt_size + iv_size > coder.properties.len() {
            return Err(crate::Error::other("Salt size + IV size too long"));
        }
        let mut salt = vec![0u8; salt_size as usize];
        salt.copy_from_slice(&coder.properties[2..(2 + salt_size)]);
        let mut iv = [0u8; 16];
        iv[0..iv_size]
            .copy_from_slice(&coder.properties[(2 + salt_size)..(2 + salt_size + iv_size)]);
        if passworld.is_empty() {
            return Err(crate::Error::PasswordRequired);
        }
        let aes_key = if num_cycles_power == 0x3f {
            let mut aes_key = [0u8; 32];
            aes_key.copy_from_slice(&salt[..salt_size]);
            let n = passworld.len().min(aes_key.len() - salt_size);
            aes_key[salt_size..n + salt_size].copy_from_slice(&passworld[0..n]);
            GenericArray::from(aes_key)
        } else {
            let mut sha = sha2::Sha256::default();
            let mut extra = [0u8; 8];
            for _ in 0..(1u32 << num_cycles_power) {
                sha.update(&salt);
                sha.update(passworld);
                sha.update(&extra);
                for i in 0..extra.len() {
                    extra[i] = extra[i].wrapping_add(1);
                    if extra[i] != 0 {
                        break;
                    }
                }
            }
            sha.finalize()
        };
        let cipher = Cipher {
            dec: Aes256Cbc::new(&aes_key, &iv.into()),
            buf: Default::default(),
        };
        Ok(Self {
            input,
            cipher,
            done: false,
            ibuffer: [0; 512],
            obuffer: Default::default(),
            ostart: 0,
            ofinish: 0,
            closed: false,
            pos: 0,
        })
    }

    fn get_more_data(&mut self) -> std::io::Result<usize> {
        if self.done {
            return Ok(0);
        } else {
            self.ofinish = 0;
            self.ostart = 0;
            self.obuffer.clear();
            let mut ibuffer = [0; 512];
            let readin = self.input.read(&mut ibuffer)?;

            if readin == 0 {
                self.done = true;
                self.ofinish = self.cipher.do_final(&mut self.obuffer)?;
                Ok(self.ofinish)
            } else {
                let n = self
                    .cipher
                    .update(&mut ibuffer[..readin], &mut self.obuffer)?;
                self.ofinish = n;
                Ok(n)
            }
        }
    }
}

impl<R: Read> Read for Aes256Sha256Decoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.ostart >= self.ofinish {
            let mut n: usize;
            n = self.get_more_data()?;
            while n == 0 && !self.done {
                n = self.get_more_data()?;
            }
            if n == 0 {
                return Ok(0);
            }
        }

        if buf.len() == 0 {
            return Ok(0);
        }
        let buf_len = self.ofinish - self.ostart;
        let size = buf_len.min(buf.len());
        buf[..size].copy_from_slice(&self.obuffer[self.ostart..self.ostart + size]);
        self.ostart += size;
        self.pos += size;
        Ok(size)
    }
}

impl<R: Read + Seek> Seek for Aes256Sha256Decoder<R> {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        let len = self.ofinish - self.ostart;
        match pos {
            std::io::SeekFrom::Start(p) => {
                let n = (p as i64 - self.pos as i64).min(len as i64);

                if n < 0 {
                    return Ok(0);
                } else {
                    self.ostart = self.ostart + n as usize;
                    return Ok(p);
                }
            }
            std::io::SeekFrom::End(_) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Unsupported,"Aes256 decoder unsupport seek from end"));
            },
            std::io::SeekFrom::Current(n) => {
                let n = n.min(len as i64);
                if n < 0 {
                    return Ok(0);
                } else {
                    self.ostart = self.ostart + n as usize;
                    return Ok(self.pos as u64 + n as u64);
                }
            }
        }
    }
}

struct Cipher {
    dec: Aes256Cbc,
    buf: Vec<u8>,
    // prev_chiper:[u8;16],
}

impl Cipher {
    fn update<W: std::io::Write>(
        &mut self,
        mut data: &mut [u8],
        mut output: W,
    ) -> std::io::Result<usize> {
        let mut n = 0;
        if self.buf.len() > 0 {
            assert!(self.buf.len() < 16);
            let end = 16 - self.buf.len();
            self.buf.extend_from_slice(&data[..end]);
            data = &mut data[end..];
            let block = GenericArray::from_mut_slice(&mut self.buf);
            self.dec.decrypt_block_mut(block);
            let out = block.as_slice();
            output.write_all(out)?;
            n += out.len();
        }

        assert_eq!(data.len() % 16, 0);

        for a in data.chunks_mut(16) {
            if a.len() < 16 {
                self.buf.extend_from_slice(a);
                break;
            }
            let block = GenericArray::from_mut_slice(a);
            self.dec.decrypt_block_mut(block);
            let out = block.as_slice();
            output.write_all(out)?;
            n += out.len();
        }
        Ok(n)
    }

    fn do_final(&mut self, output: &mut Vec<u8>) -> std::io::Result<usize> {
        if self.buf.is_empty() {
            output.clear();
            Ok(0)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "IllegalBlockSize",
            ))
        }
    }
}
