//! `vessel.archive` host bindings (`unzipEntries`, `gunzip`, `decodeText`) for subtitle packages.

use std::io::{Cursor, Read};

use encoding_rs::{Encoding, WINDOWS_1254};
use flate2::read::GzDecoder;
use rquickjs::function::Opt;
use rquickjs::{
    Array, Ctx, Error as QjsError, Function, Object, Result as QjsResult, TypedArray, Value,
};
use zip::ZipArchive;

const MAX_ZIP_ENTRIES: usize = 64;
const MAX_ENTRY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 16 * 1024 * 1024;

fn js_err<'js>(ctx: &Ctx<'js>, message: impl Into<String>) -> QjsError {
    let text: String = message.into();
    let val = rquickjs::String::from_str(ctx.clone(), &text)
        .map(|s| s.into_value())
        .unwrap_or_else(|_| Value::new_undefined(ctx.clone()));
    ctx.throw(val)
}

fn value_to_bytes<'js>(ctx: &Ctx<'js>, val: Value<'js>) -> Result<Vec<u8>, QjsError> {
    if let Some(s) = val.as_string() {
        return Ok(s.to_string()?.into_bytes());
    }
    if let Ok(ta) = TypedArray::<u8>::from_value(val) {
        let raw = unsafe { ta.as_bytes() }
            .ok_or_else(|| js_err(ctx, "detached Uint8Array passed to vessel.archive"))?;
        return Ok(raw.to_vec());
    }
    Err(js_err(ctx, "expected Uint8Array or string"))
}

/// Decodes raw bytes using an explicit encoding label, BOM sniffing, strict UTF-8, or CP1254 fallback.
pub(crate) fn decode_text_bytes(bytes: &[u8], encoding_hint: Option<&str>) -> String {
    if let Some(hint) = encoding_hint.map(str::trim).filter(|s| !s.is_empty())
        && !hint.eq_ignore_ascii_case("auto")
        && let Some(enc) = Encoding::for_label(hint.as_bytes())
    {
        let (decoded, _, _) = enc.decode(bytes);
        return decoded.into_owned();
    }

    if let Some((enc, bom_len)) = Encoding::for_bom(bytes) {
        let (decoded, _, _) = enc.decode(&bytes[bom_len..]);
        return decoded.into_owned();
    }

    if let Ok(utf8) = std::str::from_utf8(bytes) {
        return utf8.to_owned();
    }

    let (decoded, _, _) = WINDOWS_1254.decode(bytes);
    decoded.into_owned()
}

fn read_limited<R: Read>(reader: R, max_bytes: u64, label: &str) -> Result<Vec<u8>, String> {
    let mut limited = reader.take(max_bytes + 1);
    let mut buf = Vec::new();
    limited
        .read_to_end(&mut buf)
        .map_err(|err| format!("read '{label}': {err}"))?;
    if buf.len() as u64 > max_bytes {
        return Err(format!(
            "entry '{label}' exceeds maximum allowed size ({max_bytes} bytes)"
        ));
    }
    Ok(buf)
}

fn js_decode_text<'js>(ctx: Ctx<'js>, val: Value<'js>, encoding: Opt<String>) -> QjsResult<String> {
    let bytes = value_to_bytes(&ctx, val)?;
    Ok(decode_text_bytes(&bytes, encoding.0.as_deref()))
}

fn js_gunzip<'js>(ctx: Ctx<'js>, val: Value<'js>) -> QjsResult<TypedArray<'js, u8>> {
    let bytes = value_to_bytes(&ctx, val)?;
    let decoder = GzDecoder::new(Cursor::new(bytes));
    let out = read_limited(decoder, MAX_ENTRY_BYTES * 2, "gzip")
        .map_err(|err| js_err(&ctx, format!("gunzip failed: {err}")))?;
    TypedArray::new_copy(ctx, out)
}

fn js_unzip_entries<'js>(ctx: Ctx<'js>, val: Value<'js>) -> QjsResult<Array<'js>> {
    let bytes = value_to_bytes(&ctx, val)?;
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|err| js_err(&ctx, format!("invalid zip archive: {err}")))?;

    let arr = Array::new(ctx.clone())?;
    let mut total_uncompressed: u64 = 0;
    let mut out_index = 0usize;

    for index in 0..archive.len() {
        if out_index >= MAX_ZIP_ENTRIES {
            break;
        }
        let entry = archive
            .by_index(index)
            .map_err(|err| js_err(&ctx, format!("corrupt zip entry #{index}: {err}")))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .map(|cow| cow.into_owned())
            .unwrap_or_else(|_| format!("entry-{index}"));
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(js_err(
                &ctx,
                format!("zip entry '{name}' exceeds max uncompressed size"),
            ));
        }
        let data = read_limited(entry, MAX_ENTRY_BYTES, &name)
            .map_err(|err| js_err(&ctx, format!("unzip entry '{name}': {err}")))?;
        total_uncompressed = total_uncompressed.saturating_add(data.len() as u64);
        if total_uncompressed > MAX_TOTAL_BYTES {
            return Err(js_err(
                &ctx,
                "zip archive exceeds total uncompressed size limit",
            ));
        }

        let item = Object::new(ctx.clone())?;
        item.set("name", name)?;
        item.set("bytes", TypedArray::new_copy(ctx.clone(), data.as_slice())?)?;

        let text_bytes = std::sync::Arc::new(data);
        item.set(
            "text",
            Function::new(ctx.clone(), move |encoding: Opt<String>| -> String {
                decode_text_bytes(&text_bytes, encoding.0.as_deref())
            })?,
        )?;

        arr.set(out_index, item)?;
        out_index += 1;
    }

    Ok(arr)
}

/// Attaches `vessel.archive` utilities (`unzipEntries`, `gunzip`, `decodeText`) to `vessel`.
pub(crate) fn attach_archive<'js>(ctx: &Ctx<'js>, vessel_obj: &Object<'js>) -> QjsResult<()> {
    let archive_obj = Object::new(ctx.clone())?;
    archive_obj.set(
        "unzipEntries",
        Function::new(ctx.clone(), js_unzip_entries)?,
    )?;
    archive_obj.set("gunzip", Function::new(ctx.clone(), js_gunzip)?)?;
    archive_obj.set("decodeText", Function::new(ctx.clone(), js_decode_text)?)?;
    vessel_obj.set("archive", archive_obj)?;
    Ok(())
}
