//! `vessel.crypto` host bindings (Base64, Hex, hashes, HMAC, AES-CBC/ECB/GCM, RC4).

use std::fmt::Write as _;

use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockModeDecrypt, BlockModeEncrypt, KeyInit, KeyIvInit};
use aes::{Aes128, Aes192, Aes256};
use aes_gcm::aead::Aead;
use aes_gcm::{Aes128Gcm, Aes256Gcm, Nonce};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64_STD;
use hmac::{Hmac, Mac};
use md5::Md5;
use rquickjs::function::Opt;
use rquickjs::{Ctx, Error as QjsError, Function, Object, Result as QjsResult, TypedArray, Value};
use sha1::Sha1;
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

fn js_err<'js>(ctx: &Ctx<'js>, message: impl Into<String>) -> QjsError {
    let text: String = message.into();
    let val = rquickjs::String::from_str(ctx.clone(), &text)
        .map(|s| s.into_value())
        .unwrap_or_else(|_| Value::new_undefined(ctx.clone()));
    ctx.throw(val)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn hex_decode(hex: &str) -> Result<Vec<u8>, String> {
    let clean = hex.trim();
    if !clean.len().is_multiple_of(2) {
        return Err("hex string must have an even number of digits".into());
    }
    let (chunks, _) = clean.as_bytes().as_chunks::<2>();
    let mut out = Vec::with_capacity(chunks.len());
    for pair in chunks {
        let pair_str = std::str::from_utf8(pair).map_err(|err| err.to_string())?;
        let byte = u8::from_str_radix(pair_str, 16)
            .map_err(|err| format!("invalid hex byte '{pair_str}': {err}"))?;
        out.push(byte);
    }
    Ok(out)
}

fn value_to_bytes<'js>(ctx: &Ctx<'js>, val: Value<'js>) -> Result<Vec<u8>, QjsError> {
    if let Some(s) = val.as_string() {
        return Ok(s.to_string()?.into_bytes());
    }
    if let Ok(ta) = TypedArray::<u8>::from_value(val) {
        let raw = unsafe { ta.as_bytes() }
            .ok_or_else(|| js_err(ctx, "detached Uint8Array passed to vessel.crypto"))?;
        return Ok(raw.to_vec());
    }
    Err(js_err(ctx, "expected string or Uint8Array"))
}

fn aes_encrypt_bytes(
    plain: &[u8],
    key: &[u8],
    iv: Option<&[u8]>,
    mode: &str,
) -> Result<Vec<u8>, String> {
    match mode.to_ascii_lowercase().as_str() {
        "cbc" => {
            let iv_bytes = iv.unwrap_or(&[0u8; 16]);
            let iv_arr: [u8; 16] = iv_bytes.try_into().map_err(|_| {
                format!(
                    "AES-CBC requires a 16-byte IV, got {} bytes",
                    iv_bytes.len()
                )
            })?;
            match key.len() {
                16 => {
                    let key_arr: [u8; 16] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                    Ok(
                        cbc::Encryptor::<Aes128>::new(&key_arr.into(), &iv_arr.into())
                            .encrypt_padded_vec::<Pkcs7>(plain),
                    )
                }
                24 => {
                    let key_arr: [u8; 24] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                    Ok(
                        cbc::Encryptor::<Aes192>::new(&key_arr.into(), &iv_arr.into())
                            .encrypt_padded_vec::<Pkcs7>(plain),
                    )
                }
                32 => {
                    let key_arr: [u8; 32] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                    Ok(
                        cbc::Encryptor::<Aes256>::new(&key_arr.into(), &iv_arr.into())
                            .encrypt_padded_vec::<Pkcs7>(plain),
                    )
                }
                len => Err(format!("AES key must be 16, 24, or 32 bytes; got {len}")),
            }
        }
        "ecb" => match key.len() {
            16 => {
                let key_arr: [u8; 16] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                Ok(ecb::Encryptor::<Aes128>::new(&key_arr.into())
                    .encrypt_padded_vec::<Pkcs7>(plain))
            }
            24 => {
                let key_arr: [u8; 24] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                Ok(ecb::Encryptor::<Aes192>::new(&key_arr.into())
                    .encrypt_padded_vec::<Pkcs7>(plain))
            }
            32 => {
                let key_arr: [u8; 32] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                Ok(ecb::Encryptor::<Aes256>::new(&key_arr.into())
                    .encrypt_padded_vec::<Pkcs7>(plain))
            }
            len => Err(format!("AES key must be 16, 24, or 32 bytes; got {len}")),
        },
        "gcm" => {
            let iv_bytes = iv.ok_or_else(|| "AES-GCM requires a 12-byte IV/nonce".to_owned())?;
            let iv_arr: [u8; 12] = iv_bytes.try_into().map_err(|_| {
                format!(
                    "AES-GCM requires a 12-byte IV/nonce, got {} bytes",
                    iv_bytes.len()
                )
            })?;
            let nonce = Nonce::from(iv_arr);
            match key.len() {
                16 => {
                    let cipher = Aes128Gcm::new_from_slice(key).map_err(|err| err.to_string())?;
                    cipher
                        .encrypt(&nonce, plain)
                        .map_err(|err| format!("AES-GCM encrypt failed: {err}"))
                }
                32 => {
                    let cipher = Aes256Gcm::new_from_slice(key).map_err(|err| err.to_string())?;
                    cipher
                        .encrypt(&nonce, plain)
                        .map_err(|err| format!("AES-GCM encrypt failed: {err}"))
                }
                len => Err(format!("AES-GCM key must be 16 or 32 bytes; got {len}")),
            }
        }
        other => Err(format!(
            "unsupported AES mode '{other}'; expected cbc, ecb, or gcm"
        )),
    }
}

fn aes_decrypt_bytes(
    cipher_bytes: &[u8],
    key: &[u8],
    iv: Option<&[u8]>,
    mode: &str,
) -> Result<Vec<u8>, String> {
    match mode.to_ascii_lowercase().as_str() {
        "cbc" => {
            let iv_bytes = iv.unwrap_or(&[0u8; 16]);
            let iv_arr: [u8; 16] = iv_bytes.try_into().map_err(|_| {
                format!(
                    "AES-CBC requires a 16-byte IV, got {} bytes",
                    iv_bytes.len()
                )
            })?;
            match key.len() {
                16 => {
                    let key_arr: [u8; 16] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                    cbc::Decryptor::<Aes128>::new(&key_arr.into(), &iv_arr.into())
                        .decrypt_padded_vec::<Pkcs7>(cipher_bytes)
                        .map_err(|err| format!("AES-CBC decrypt failed: {err}"))
                }
                24 => {
                    let key_arr: [u8; 24] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                    cbc::Decryptor::<Aes192>::new(&key_arr.into(), &iv_arr.into())
                        .decrypt_padded_vec::<Pkcs7>(cipher_bytes)
                        .map_err(|err| format!("AES-CBC decrypt failed: {err}"))
                }
                32 => {
                    let key_arr: [u8; 32] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                    cbc::Decryptor::<Aes256>::new(&key_arr.into(), &iv_arr.into())
                        .decrypt_padded_vec::<Pkcs7>(cipher_bytes)
                        .map_err(|err| format!("AES-CBC decrypt failed: {err}"))
                }
                len => Err(format!("AES key must be 16, 24, or 32 bytes; got {len}")),
            }
        }
        "ecb" => match key.len() {
            16 => {
                let key_arr: [u8; 16] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                ecb::Decryptor::<Aes128>::new(&key_arr.into())
                    .decrypt_padded_vec::<Pkcs7>(cipher_bytes)
                    .map_err(|err| format!("AES-ECB decrypt failed: {err}"))
            }
            24 => {
                let key_arr: [u8; 24] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                ecb::Decryptor::<Aes192>::new(&key_arr.into())
                    .decrypt_padded_vec::<Pkcs7>(cipher_bytes)
                    .map_err(|err| format!("AES-ECB decrypt failed: {err}"))
            }
            32 => {
                let key_arr: [u8; 32] = key.try_into().map_err(|_| "invalid key".to_owned())?;
                ecb::Decryptor::<Aes256>::new(&key_arr.into())
                    .decrypt_padded_vec::<Pkcs7>(cipher_bytes)
                    .map_err(|err| format!("AES-ECB decrypt failed: {err}"))
            }
            len => Err(format!("AES key must be 16, 24, or 32 bytes; got {len}")),
        },
        "gcm" => {
            let iv_bytes = iv.ok_or_else(|| "AES-GCM requires a 12-byte IV/nonce".to_owned())?;
            let iv_arr: [u8; 12] = iv_bytes.try_into().map_err(|_| {
                format!(
                    "AES-GCM requires a 12-byte IV/nonce, got {} bytes",
                    iv_bytes.len()
                )
            })?;
            let nonce = Nonce::from(iv_arr);
            match key.len() {
                16 => {
                    let cipher = Aes128Gcm::new_from_slice(key).map_err(|err| err.to_string())?;
                    cipher
                        .decrypt(&nonce, cipher_bytes)
                        .map_err(|err| format!("AES-GCM decrypt failed: {err}"))
                }
                32 => {
                    let cipher = Aes256Gcm::new_from_slice(key).map_err(|err| err.to_string())?;
                    cipher
                        .decrypt(&nonce, cipher_bytes)
                        .map_err(|err| format!("AES-GCM decrypt failed: {err}"))
                }
                len => Err(format!("AES-GCM key must be 16 or 32 bytes; got {len}")),
            }
        }
        other => Err(format!(
            "unsupported AES mode '{other}'; expected cbc, ecb, or gcm"
        )),
    }
}

pub(crate) fn rc4_transform(input: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if key.is_empty() {
        return Err("RC4 key cannot be empty".into());
    }
    let mut s = [0u8; 256];
    for (idx, slot) in s.iter_mut().enumerate() {
        *slot = idx as u8;
    }
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key.len()]);
        s.swap(i, j as usize);
    }

    let mut i: u8 = 0;
    let mut j: u8 = 0;
    let mut out = Vec::with_capacity(input.len());
    for &byte in input {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);
        let k = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
        out.push(byte ^ k);
    }
    Ok(out)
}

fn js_base64_encode<'js>(ctx: Ctx<'js>, val: Value<'js>) -> QjsResult<String> {
    let bytes = value_to_bytes(&ctx, val)?;
    Ok(BASE64_STD.encode(bytes))
}

fn js_base64_decode<'js>(ctx: Ctx<'js>, input: String) -> QjsResult<String> {
    let bytes = BASE64_STD
        .decode(input.trim())
        .map_err(|err| js_err(&ctx, format!("base64Decode failed: {err}")))?;
    String::from_utf8(bytes)
        .map_err(|err| js_err(&ctx, format!("base64Decode UTF-8 failed: {err}")))
}

fn js_base64_decode_bytes<'js>(ctx: Ctx<'js>, input: String) -> QjsResult<TypedArray<'js, u8>> {
    let bytes = BASE64_STD
        .decode(input.trim())
        .map_err(|err| js_err(&ctx, format!("base64DecodeBytes failed: {err}")))?;
    TypedArray::new_copy(ctx, bytes)
}

fn js_hex_encode<'js>(ctx: Ctx<'js>, val: Value<'js>) -> QjsResult<String> {
    let bytes = value_to_bytes(&ctx, val)?;
    Ok(hex_encode(&bytes))
}

fn js_hex_decode<'js>(ctx: Ctx<'js>, hex: String) -> QjsResult<TypedArray<'js, u8>> {
    let bytes = hex_decode(&hex).map_err(|err| js_err(&ctx, err))?;
    TypedArray::new_copy(ctx, bytes)
}

fn js_hmac_sha256<'js>(ctx: Ctx<'js>, key: String, data: String) -> QjsResult<String> {
    let mut mac = <HmacSha256 as KeyInit>::new_from_slice(key.as_bytes())
        .map_err(|err| js_err(&ctx, format!("hmacSha256 failed: {err}")))?;
    mac.update(data.as_bytes());
    Ok(hex_encode(&mac.finalize().into_bytes()))
}

fn js_aes_encrypt<'js>(
    ctx: Ctx<'js>,
    plain: String,
    key: String,
    iv: Opt<String>,
    mode: Opt<String>,
) -> QjsResult<String> {
    let mode_str = mode.0.as_deref().unwrap_or("cbc");
    let iv_bytes = iv.0.as_ref().map(|s| s.as_bytes());
    let cipher = aes_encrypt_bytes(plain.as_bytes(), key.as_bytes(), iv_bytes, mode_str)
        .map_err(|err| js_err(&ctx, err))?;
    Ok(BASE64_STD.encode(cipher))
}

fn js_aes_decrypt<'js>(
    ctx: Ctx<'js>,
    cipher_b64: String,
    key: String,
    iv: Opt<String>,
    mode: Opt<String>,
) -> QjsResult<String> {
    let mode_str = mode.0.as_deref().unwrap_or("cbc");
    let iv_bytes = iv.0.as_ref().map(|s| s.as_bytes());
    let cipher_bytes = BASE64_STD
        .decode(cipher_b64.trim())
        .map_err(|err| js_err(&ctx, format!("aesDecrypt base64 failed: {err}")))?;
    let plain = aes_decrypt_bytes(&cipher_bytes, key.as_bytes(), iv_bytes, mode_str)
        .map_err(|err| js_err(&ctx, err))?;
    String::from_utf8(plain).map_err(|err| js_err(&ctx, format!("aesDecrypt UTF-8 failed: {err}")))
}

fn js_rc4<'js>(ctx: Ctx<'js>, input: String, key: String) -> QjsResult<String> {
    let bytes: Vec<u8> = input
        .chars()
        .map(|ch| u8::try_from(ch as u32).map_err(|_| "non-byte char in rc4 input"))
        .collect::<Result<Vec<u8>, _>>()
        .map_err(|err| js_err(&ctx, err))?;
    let out = rc4_transform(&bytes, key.as_bytes()).map_err(|err| js_err(&ctx, err))?;
    Ok(out.into_iter().map(char::from).collect())
}

/// Attaches `vessel.crypto` utilities to the `vessel` host object.
pub(crate) fn attach_crypto<'js>(ctx: &Ctx<'js>, vessel_obj: &Object<'js>) -> QjsResult<()> {
    let crypto_obj = Object::new(ctx.clone())?;

    crypto_obj.set(
        "base64Encode",
        Function::new(ctx.clone(), js_base64_encode)?,
    )?;
    crypto_obj.set(
        "base64Decode",
        Function::new(ctx.clone(), js_base64_decode)?,
    )?;
    crypto_obj.set(
        "base64DecodeBytes",
        Function::new(ctx.clone(), js_base64_decode_bytes)?,
    )?;
    crypto_obj.set("hexEncode", Function::new(ctx.clone(), js_hex_encode)?)?;
    crypto_obj.set("hexDecode", Function::new(ctx.clone(), js_hex_decode)?)?;
    crypto_obj.set(
        "md5",
        Function::new(ctx.clone(), |input: String| -> String {
            hex_encode(&Md5::digest(input.as_bytes()))
        })?,
    )?;
    crypto_obj.set(
        "sha1",
        Function::new(ctx.clone(), |input: String| -> String {
            hex_encode(&Sha1::digest(input.as_bytes()))
        })?,
    )?;
    crypto_obj.set(
        "sha256",
        Function::new(ctx.clone(), |input: String| -> String {
            hex_encode(&Sha256::digest(input.as_bytes()))
        })?,
    )?;
    crypto_obj.set("hmacSha256", Function::new(ctx.clone(), js_hmac_sha256)?)?;
    crypto_obj.set("aesEncrypt", Function::new(ctx.clone(), js_aes_encrypt)?)?;
    crypto_obj.set("aesDecrypt", Function::new(ctx.clone(), js_aes_decrypt)?)?;
    crypto_obj.set("rc4", Function::new(ctx.clone(), js_rc4)?)?;

    vessel_obj.set("crypto", crypto_obj)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{aes_decrypt_bytes, aes_encrypt_bytes, hex_decode, hex_encode, rc4_transform};

    #[test]
    fn supports_aes_modes_hex_and_rc4_round_trip() {
        let key16 = b"0123456789abcdef";
        let iv16 = b"fedcba9876543210";
        let iv12 = b"123456789012";
        let plain = b"anthology-stream-secret";

        let cbc_ct = aes_encrypt_bytes(plain, key16, Some(iv16), "cbc").expect("cbc encrypt");
        assert_eq!(
            aes_decrypt_bytes(&cbc_ct, key16, Some(iv16), "cbc").expect("cbc decrypt"),
            plain
        );

        let ecb_ct = aes_encrypt_bytes(plain, key16, None, "ecb").expect("ecb encrypt");
        assert_eq!(
            aes_decrypt_bytes(&ecb_ct, key16, None, "ecb").expect("ecb decrypt"),
            plain
        );

        let gcm_ct = aes_encrypt_bytes(plain, key16, Some(iv12), "gcm").expect("gcm encrypt");
        assert_eq!(
            aes_decrypt_bytes(&gcm_ct, key16, Some(iv12), "gcm").expect("gcm decrypt"),
            plain
        );

        let hex = hex_encode(plain);
        assert_eq!(hex_decode(&hex).expect("hex decode"), plain);

        let cipher = rc4_transform(plain, b"vessel-key").expect("rc4 encrypt");
        assert_eq!(
            rc4_transform(&cipher, b"vessel-key").expect("rc4 decrypt"),
            plain
        );
    }
}
