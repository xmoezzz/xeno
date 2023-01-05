use super::{password::Password, *};
use js_sys::*;
use std::io::{Seek, SeekFrom};
use std::{collections::HashMap, io::Cursor};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub fn decompress(mut src: &[u8], pwd: &str, f: &Function) -> Result<(), String> {
    let mut src_reader = Cursor::new(&mut src);
    let pos = src_reader.stream_position().map_err(|e| e.to_string())?;
    let len = src_reader
        .seek(SeekFrom::End(0))
        .map_err(|e| e.to_string())?;
    src_reader
        .seek(SeekFrom::Start(pos))
        .map_err(|e| e.to_string())?;
    let mut seven =
        SevenZReader::new(src_reader, len, Password::from(pwd)).map_err(|e| e.to_string())?;
    seven
        .for_each_entries(|entry, reader| {
            if !entry.is_directory() {
                let path = entry.name();

                if entry.size() > 0 {
                    let mut writer = Vec::new();
                    std::io::copy(reader, &mut writer).map_err(crate::Error::io)?;
                    // result.set(&JsValue::from(path), &Uint8Array::from(&writer[..]));
                    f.call2(
                        &JsValue::NULL,
                        &JsValue::from(path),
                        &Uint8Array::from(&writer[..]),
                    );
                }
            }
            Ok(true)
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
