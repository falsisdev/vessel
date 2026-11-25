//! `vessel.unpack(code)` and `vessel.evalWithEnv(script, env?)` helpers for embed extraction.

use std::collections::BTreeMap;

use rquickjs::prelude::Async;
use rquickjs::{CatchResultExt, Ctx, Function, Object, Result as QjsResult, Value};
use serde::Deserialize;
use vessel_domain::{PluginId, VesselError, VesselResult};

use super::HostResult;
use crate::sandbox::{PluginSandbox, SandboxLimits};

const EVAL_ENV_WRAPPER_JS: &str = r#"
(() => {
    globalThis.vessel.evalWithEnv = async (script, env) => {
        const envJson = env ? JSON.stringify(env) : "{}";
        const outJson = await globalThis.__vessel_eval_with_env(String(script), envJson);
        return JSON.parse(outJson);
    };
})();
"#;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvalEnvInput {
    url: Option<String>,
    user_agent: Option<String>,
    cookies: Option<BTreeMap<String, String>>,
}

fn encode_radix(mut num: usize, radix: usize) -> String {
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    if radix < 2 || radix > ALPHABET.len() {
        return num.to_string();
    }
    let mut digits = Vec::new();
    loop {
        digits.push(ALPHABET[num % radix] as char);
        num /= radix;
        if num == 0 {
            break;
        }
    }
    digits.into_iter().rev().collect()
}

fn parse_js_quoted_arg(input: &str) -> Result<(String, &str), String> {
    let trimmed = input.trim_start();
    let quote = trimmed
        .chars()
        .next()
        .filter(|c| *c == '\'' || *c == '"')
        .ok_or_else(|| "expected quoted string argument in packed script".to_owned())?;

    let mut out = String::new();
    let mut chars = trimmed[quote.len_utf8()..].char_indices();
    while let Some((idx, ch)) = chars.next() {
        if ch == '\\' {
            let Some((_, escaped)) = chars.next() else {
                return Err("unterminated escape sequence in packed script".into());
            };
            match escaped {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' => out.push('\\'),
                '\'' => out.push('\''),
                '"' => out.push('"'),
                other => {
                    out.push('\\');
                    out.push(other);
                }
            }
        } else if ch == quote {
            let rest = &trimmed[quote.len_utf8() + idx + ch.len_utf8()..];
            return Ok((out, rest));
        } else {
            out.push(ch);
        }
    }

    Err("unterminated string literal in packed script".into())
}

fn find_packer_header_start(input: &str) -> usize {
    let mut search_from = 0usize;
    while let Some(rel) = input[search_from..].find("function") {
        let fn_start = search_from + rel;
        let after_fn = input[fn_start + "function".len()..].trim_start();
        if let Some(after_paren) = after_fn.strip_prefix('(') {
            let compact: String = after_paren
                .chars()
                .take(32)
                .filter(|c| !c.is_ascii_whitespace())
                .collect();
            if compact.starts_with("p,a,c,k") {
                return fn_start;
            }
        }
        search_from = fn_start + "function".len();
    }
    0
}

fn find_packer_args_start(slice: &str) -> Option<usize> {
    if let Some(idx) = slice.find("}(") {
        return Some(idx + 2);
    }
    let bytes = slice.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'}' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && bytes[j] == b'(' {
                return Some(j + 1);
            }
        }
        i += 1;
    }
    None
}

/// Decodes a Dean Edwards `eval(function(p,a,c,k,e,d){...}(...))` packed JS string safely in Rust.
pub fn unpack_dean_edwards(packed: &str) -> VesselResult<String> {
    let header_start = find_packer_header_start(packed);
    let tail = &packed[header_start..];
    let Some(args_offset) = find_packer_args_start(tail) else {
        return Err(VesselError::InvalidInput(
            "input does not match Dean Edwards packed function signature".into(),
        ));
    };

    let args = &tail[args_offset..];
    let (payload, after_payload) = parse_js_quoted_arg(args).map_err(VesselError::InvalidInput)?;

    let after_comma = after_payload
        .trim_start()
        .strip_prefix(',')
        .ok_or_else(|| VesselError::InvalidInput("missing radix comma in packed script".into()))?;
    let (radix_raw, after_radix) = after_comma
        .split_once(',')
        .ok_or_else(|| VesselError::InvalidInput("missing count comma in packed script".into()))?;
    let radix: usize = radix_raw
        .trim()
        .parse()
        .map_err(|_| VesselError::InvalidInput("invalid packer radix".into()))?;

    let (count_raw, after_count) = after_radix
        .split_once(',')
        .ok_or_else(|| VesselError::InvalidInput("missing symtab comma in packed script".into()))?;
    let count: usize = count_raw
        .trim()
        .parse()
        .map_err(|_| VesselError::InvalidInput("invalid packer symbol count".into()))?;

    let (symtab_raw, _) = parse_js_quoted_arg(after_count).map_err(VesselError::InvalidInput)?;
    let symbols: Vec<&str> = symtab_raw.split('|').collect();
    if symbols.is_empty() {
        return Ok(payload);
    }

    let mut dict = BTreeMap::new();
    for (index, replacement) in symbols.iter().enumerate().take(count) {
        if !replacement.is_empty() {
            dict.insert(encode_radix(index, radix), *replacement);
        }
    }

    let mut result = String::with_capacity(payload.len());
    let mut token = String::new();
    for ch in payload.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            if !token.is_empty() {
                if let Some(rep) = dict.get(&token) {
                    result.push_str(rep);
                } else {
                    result.push_str(&token);
                }
                token.clear();
            }
            result.push(ch);
        }
    }
    if !token.is_empty() {
        if let Some(rep) = dict.get(&token) {
            result.push_str(rep);
        } else {
            result.push_str(&token);
        }
    }

    Ok(result)
}

async fn run_eval_with_env(
    plugin_id: &PluginId,
    script: &str,
    env: EvalEnvInput,
) -> VesselResult<String> {
    let sandbox = PluginSandbox::new(plugin_id.clone(), SandboxLimits::default()).await?;

    let cookie_header = env
        .cookies
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ");
    let env_json = serde_json::json!({
        "url": env.url.unwrap_or_else(|| "https://about.blank/".into()),
        "userAgent": env.user_agent.unwrap_or_else(|| "Vessel/0.1".into()),
        "cookieHeader": cookie_header,
    });
    let env_lit = serde_json::to_string(&env_json)
        .map_err(|err| VesselError::InvalidInput(format!("serialize evalWithEnv opts: {err}")))?;

    let bootstrap = format!(
        r#"
        const __env = {env_lit};
        const __parsedUrl = (() => {{
            try {{
                return new URL(__env.url);
            }} catch {{
                return {{ href: __env.url, origin: "", host: "", hostname: "", pathname: "/", search: "" }};
            }}
        }})();
        globalThis.location = __parsedUrl;
        globalThis.navigator = {{ userAgent: __env.userAgent }};
        globalThis.document = {{ cookie: __env.cookieHeader, location: __parsedUrl }};
        globalThis.window = globalThis;
        globalThis.self = globalThis;
        "#
    );

    let sub_script = format!(
        r#"
        {bootstrap}
        const __res = (() => {{
        {script}
        }})();
        JSON.stringify(__res === undefined ? null : __res);
        "#
    );

    let timeout = sandbox.limits().execution_timeout;
    let result = sandbox
        .run_in_context(timeout, async |ctx| {
            let vessel_obj = Object::new(ctx.clone())
                .catch(&ctx)
                .map_err(|err| err.to_string())?;
            super::crypto::attach_crypto(&ctx, &vessel_obj)
                .catch(&ctx)
                .map_err(|err| err.to_string())?;
            ctx.globals()
                .set("vessel", vessel_obj)
                .catch(&ctx)
                .map_err(|err| err.to_string())?;
            ctx.eval::<(), _>(super::POLYFILLS_JS)
                .catch(&ctx)
                .map_err(|err| err.to_string())?;
            ctx.eval::<String, _>(sub_script)
                .catch(&ctx)
                .map_err(|err| err.to_string())
        })
        .await;
    sandbox.shutdown().await;
    result
}

fn unpack_fn<'js>(ctx: Ctx<'js>, code: String) -> QjsResult<String> {
    unpack_dean_edwards(&code).map_err(|err| {
        let msg = err.to_string();
        let val = rquickjs::String::from_str(ctx.clone(), &msg)
            .map(|s| s.into_value())
            .unwrap_or_else(|_| Value::new_undefined(ctx.clone()));
        ctx.throw(val)
    })
}

/// Attaches `vessel.unpack` and `vessel.evalWithEnv` to `globalThis.vessel`.
pub(crate) fn attach_unpack_and_env<'js>(
    ctx: &Ctx<'js>,
    plugin_id: &PluginId,
    vessel_obj: &Object<'js>,
) -> QjsResult<()> {
    vessel_obj.set("unpack", Function::new(ctx.clone(), unpack_fn)?)?;

    let pid = plugin_id.clone();
    ctx.globals().set(
        "__vessel_eval_with_env",
        Function::new(
            ctx.clone(),
            Async(move |script: String, env_json: String| {
                let plugin_id = pid.clone();
                async move {
                    let env: EvalEnvInput = serde_json::from_str(&env_json).unwrap_or_default();
                    HostResult(
                        run_eval_with_env(&plugin_id, &script, env)
                            .await
                            .map_err(|err| err.to_string()),
                    )
                }
            }),
        )?,
    )?;

    ctx.eval::<(), _>(EVAL_ENV_WRAPPER_JS)
        .catch(ctx)
        .map_err(|err| err.throw(ctx))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::unpack_dean_edwards;

    #[test]
    fn unpacks_dean_edwards_packed_script_cleanly() {
        let packed = r#"eval(function(p,a,c,k,e,d){while(c--)if(k[c])p=p.replace(new RegExp('\\b'+c.toString(a)+'\\b','g'),k[c]);return p}('0 1="2://3.4/5.6";',7,7,'const|streamUrl|https|cdn|anthology|master|m3u8'.split('|'),0,{}))"#;
        let unpacked = unpack_dean_edwards(packed).expect("unpack p,a,c,k,e,d");
        assert_eq!(
            unpacked,
            r#"const streamUrl="https://cdn.anthology/master.m3u8";"#
        );
    }

    #[test]
    fn unpacks_when_preceded_by_other_iife_scripts() {
        let html = r#"<script>!function(){return 42}()</script><script>eval(function(p,a,c,k,e,d){while(c--)if(k[c])p=p.replace(new RegExp('\\b'+c.toString(a)+'\\b','g'),k[c]);return p}('0 1="2://3.4/5.6";',7,7,'const|streamUrl|https|cdn|dizibak|master|m3u8'.split('|'),0,{}))</script>"#;
        let unpacked = unpack_dean_edwards(html).expect("skip leading iife script");
        assert_eq!(
            unpacked,
            r#"const streamUrl="https://cdn.dizibak/master.m3u8";"#
        );
    }
}
