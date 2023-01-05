[![Crate](https://img.shields.io/crates/v/sevenz-rust.svg)](https://crates.io/crates/sevenz-rust)
 [![Documentation](https://docs.rs/sevenz-rust/badge.svg)](https://docs.rs/sevenz-rust)
 
This project is a 7z compressor/decompressor written in pure rust.<br/>
And it's very much inspired by the [apache commons-compress](https://commons.apache.org/proper/commons-compress/) project.

## Decompression

Supported codecs:
 - [x] COPY
 - [x] LZMA
 - [x] LZMA2


Supported filters:
 - [x] BCJ X86
 - [x] BCJ PPC
 - [x] BCJ IA64
 - [x] BCJ ARM
 - [x] BCJ ARM_THUMB
 - [x] BCJ SPARC
 - [x] DELTA

The dependence of lzma-rs was removed sine 0.1.3.<br>
The LZMA/LZMA2 decoder and all filters code was ported from [tukaani xz for java](https://tukaani.org/xz/java.html)


### Usage

```
[dependencies]
sevenz-rust={version="0.1.5"}
```

Decompress source file "data/sample.7z" to dest path "data/sample"
```rust
sevenz_rust::decompress_file("data/sample.7z", "data/sample").expect("complete");
```

Decompress a encrypted 7z file

Add 'aes256' feature
```
[dependencies]
sevenz-rust={version="0.1.5", features=["aes256"]}
```

```rust
sevenz_rust::decompress_file_with_password("path/to/encrypted.7z", "path/to/output", "password".into()).expect("complete");
```

## Compression
The compression feature is under development.

## Dependencies
- [crc](https://crates.io/crates/crc)
- [bit-set](https://crates.io/crates/bit-set)
