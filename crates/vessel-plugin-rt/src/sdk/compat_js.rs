//! Sandbox compatibility runtime shims (`fetch`, `Buffer`, `cheerio`, `CryptoJS`, `axios`, `require`).

pub(crate) const COMPAT_BRIDGE_JS: &str = r##"
(() => {
    const vCrypto = globalThis.vessel && globalThis.vessel.crypto;
    const vHtml = globalThis.vessel && globalThis.vessel.html;
    const vHttp = globalThis.vessel && globalThis.vessel.http;

    // 0. Timers + AbortController / AbortSignal (backed by vessel.__sleep)
    if (typeof globalThis.setTimeout !== "function" && globalThis.vessel && typeof globalThis.vessel.__sleep === "function") {
        let nextTimerId = 1;
        const activeTimers = new Set();
        globalThis.setTimeout = (fn, ms = 0, ...args) => {
            const id = nextTimerId++;
            activeTimers.add(id);
            const delay = Math.max(0, Number(ms) || 0);
            globalThis.vessel.__sleep(delay).then(() => {
                if (activeTimers.delete(id) && typeof fn === "function") {
                    fn(...args);
                }
            });
            return id;
        };
        globalThis.clearTimeout = (id) => {
            activeTimers.delete(id);
        };
        globalThis.setInterval = (fn, ms = 0, ...args) => {
            const id = nextTimerId++;
            activeTimers.add(id);
            const delay = Math.max(10, Number(ms) || 10);
            (async () => {
                while (activeTimers.has(id)) {
                    await globalThis.vessel.__sleep(delay);
                    if (!activeTimers.has(id)) break;
                    if (typeof fn === "function") fn(...args);
                }
            })();
            return id;
        };
        globalThis.clearInterval = (id) => {
            activeTimers.delete(id);
        };
    }

    if (typeof globalThis.FormData !== "function") {
        globalThis.FormData = class FormData {
            constructor() {
                this._entries = [];
            }
            append(k, v) { this._entries.push([String(k), v === undefined || v === null ? "" : String(v)]); }
            set(k, v) {
                const key = String(k);
                this._entries = this._entries.filter(([ek]) => ek !== key);
                this._entries.push([key, v === undefined || v === null ? "" : String(v)]);
            }
            get(k) {
                const found = this._entries.find(([ek]) => ek === String(k));
                return found ? found[1] : null;
            }
            getAll(k) { return this._entries.filter(([ek]) => ek === String(k)).map(([, v]) => v); }
            has(k) { return this._entries.some(([ek]) => ek === String(k)); }
            delete(k) { this._entries = this._entries.filter(([ek]) => ek !== String(k)); }
            entries() { return this._entries[Symbol.iterator](); }
            [Symbol.iterator]() { return this.entries(); }
        };
    }

    if (typeof globalThis.AbortSignal !== "function") {
        class VesselAbortSignal {
            constructor() {
                this.aborted = false;
                this.reason = undefined;
                this._listeners = [];
                this._timeoutMs = undefined;
            }
            addEventListener(type, fn) {
                if (type === "abort" && typeof fn === "function") this._listeners.push(fn);
            }
            removeEventListener(type, fn) {
                if (type === "abort") this._listeners = this._listeners.filter((f) => f !== fn);
            }
            _triggerAbort(reason) {
                if (this.aborted) return;
                this.aborted = true;
                this.reason = reason || new Error("Aborted");
                if (typeof this.onabort === "function") {
                    try { this.onabort(); } catch {}
                }
                for (const fn of this._listeners.slice()) {
                    try { fn({ type: "abort" }); } catch {}
                }
            }
        }
        VesselAbortSignal.timeout = (ms) => {
            const sig = new VesselAbortSignal();
            sig._timeoutMs = Math.max(100, Number(ms) || 10000);
            return sig;
        };
        VesselAbortSignal.abort = (reason) => {
            const sig = new VesselAbortSignal();
            sig._triggerAbort(reason);
            return sig;
        };
        globalThis.AbortSignal = VesselAbortSignal;
    }

    if (typeof globalThis.AbortController !== "function") {
        globalThis.AbortController = class AbortController {
            constructor() {
                this.signal = new globalThis.AbortSignal();
            }
            abort(reason) {
                this.signal._triggerAbort(reason);
            }
        };
    }

    // 1. Web Headers + fetch() over vessel.http.request (SSRF + Domain Policy preserved)
    if (typeof globalThis.Headers !== "function") {
        globalThis.Headers = class Headers {
            constructor(init) {
                this._map = Object.create(null);
                if (Array.isArray(init)) {
                    for (const pair of init) {
                        if (Array.isArray(pair) && pair.length >= 2) {
                            this._map[String(pair[0]).toLowerCase()] = String(pair[1]);
                        }
                    }
                } else if (init && typeof init === "object") {
                    for (const [k, v] of Object.entries(init)) {
                        if (v !== undefined && v !== null) {
                            this._map[String(k).toLowerCase()] = String(v);
                        }
                    }
                }
            }
            get(name) {
                const key = String(name).toLowerCase();
                return Object.prototype.hasOwnProperty.call(this._map, key) ? this._map[key] : null;
            }
            has(name) {
                return Object.prototype.hasOwnProperty.call(this._map, String(name).toLowerCase());
            }
            set(name, value) {
                this._map[String(name).toLowerCase()] = String(value);
            }
            forEach(cb, thisArg) {
                for (const [k, v] of Object.entries(this._map)) {
                    cb.call(thisArg, v, k, this);
                }
            }
        };
    }

    if (typeof globalThis.fetch !== "function" && vHttp) {
        globalThis.fetch = async (input, init = {}) => {
            const url = typeof input === "string" ? input : (input && input.url ? String(input.url) : String(input));
            const opts = init && typeof init === "object" ? init : {};
            let rawHeaders = {};
            if (opts.headers instanceof globalThis.Headers) {
                opts.headers.forEach((v, k) => { rawHeaders[k] = v; });
            } else if (opts.headers && typeof opts.headers === "object") {
                rawHeaders = { ...opts.headers };
            }
            let body = opts.body;
            if (body instanceof globalThis.URLSearchParams) {
                if (!Object.keys(rawHeaders).some((k) => k.toLowerCase() === "content-type")) {
                    rawHeaders["Content-Type"] = "application/x-www-form-urlencoded;charset=UTF-8";
                }
                body = body.toString();
            } else if (body instanceof globalThis.FormData) {
                const boundary = "----VesselFormBoundary" + Math.random().toString(16).slice(2);
                if (!Object.keys(rawHeaders).some((k) => k.toLowerCase() === "content-type")) {
                    rawHeaders["Content-Type"] = `multipart/form-data; boundary=${boundary}`;
                }
                const parts = [];
                for (const [fk, fv] of body.entries()) {
                    parts.push(`--${boundary}\r\nContent-Disposition: form-data; name="${String(fk).replace(/"/g, '\\"')}"\r\n\r\n${String(fv)}\r\n`);
                }
                parts.push(`--${boundary}--\r\n`);
                body = parts.join("");
            }
            const signalTimeout = opts.signal && typeof opts.signal._timeoutMs === "number" ? opts.signal._timeoutMs : undefined;
            const resp = await vHttp.request({
                method: opts.method || "GET",
                url,
                headers: rawHeaders,
                body,
                followRedirects: opts.redirect !== "manual",
                timeoutMs: typeof opts.timeout === "number" ? opts.timeout : signalTimeout,
            });
            const hdrs = new globalThis.Headers(resp.headers || {});
            const textFn = async () => resp.text();
            const jsonFn = async () => resp.json();
            const abFn = async () => {
                const bytes = resp.bytes();
                return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
            };
            const blobFn = async () => ({ text: textFn, arrayBuffer: abFn });
            const baseResp = {
                ok: resp.status >= 200 && resp.status < 300,
                status: resp.status,
                statusText: String(resp.status),
                url: resp.url || url,
                headers: hdrs,
                text: textFn,
                json: jsonFn,
                arrayBuffer: abFn,
                blob: blobFn,
            };
            return {
                ...baseResp,
                clone: () => ({ ...baseResp }),
            };
        };
    }

    // 2. Buffer polyfill
    if (typeof globalThis.Buffer !== "function") {
        class VesselBuffer extends Uint8Array {
            toString(encoding = "utf8") {
                const enc = String(encoding).toLowerCase();
                if (enc === "base64") {
                    return vCrypto.base64Encode(this);
                }
                if (enc === "hex") {
                    return vCrypto.hexEncode(this);
                }
                if (enc === "binary" || enc === "latin1") {
                    let out = "";
                    for (let i = 0; i < this.length; i++) {
                        out += String.fromCharCode(this[i]);
                    }
                    return out;
                }
                const b64 = vCrypto.base64Encode(this);
                return vCrypto.base64Decode(b64);
            }
        }
        VesselBuffer.isBuffer = (v) => v instanceof VesselBuffer || v instanceof Uint8Array;
        VesselBuffer.from = (input, encoding = "utf8") => {
            if (input instanceof Uint8Array || Array.isArray(input)) {
                return new VesselBuffer(input);
            }
            if (typeof input === "string") {
                const enc = String(encoding).toLowerCase();
                if (enc === "base64") {
                    return new VesselBuffer(vCrypto.base64DecodeBytes(input));
                }
                if (enc === "hex") {
                    return new VesselBuffer(vCrypto.hexDecode(input));
                }
                if (enc === "binary" || enc === "latin1") {
                    const bytes = new VesselBuffer(input.length);
                    for (let i = 0; i < input.length; i++) {
                        bytes[i] = input.charCodeAt(i) & 0xff;
                    }
                    return bytes;
                }
                const bytes = new globalThis.TextEncoder().encode(input);
                return new VesselBuffer(bytes);
            }
            if (input && typeof input === "object" && typeof input.byteLength === "number") {
                return new VesselBuffer(new Uint8Array(input));
            }
            return new VesselBuffer(0);
        };
        VesselBuffer.concat = (list) => {
            const arrs = Array.isArray(list) ? list.map((x) => VesselBuffer.from(x)) : [];
            const total = arrs.reduce((sum, a) => sum + a.length, 0);
            const out = new VesselBuffer(total);
            let offset = 0;
            for (const a of arrs) {
                out.set(a, offset);
                offset += a.length;
            }
            return out;
        };
        VesselBuffer.alloc = (len) => new VesselBuffer(Number(len) || 0);
        globalThis.Buffer = VesselBuffer;
    }

    // 3. Rust-backed Cheerio shim over vessel.html
    const createCheerioModule = () => {
        const isHostNode = (v) => Boolean(v && typeof v === "object" && typeof v._nodeKey === "string");
        const dedupeNodes = (nodes) => {
            const seen = Object.create(null);
            const out = [];
            for (const n of nodes) {
                if (!isHostNode(n)) continue;
                if (!seen[n._nodeKey]) {
                    seen[n._nodeKey] = true;
                    out.push(n);
                }
            }
            return out;
        };

        const makeSelection = (nodes, doc) => {
            const list = dedupeNodes(nodes);
            const wrap = (nextNodes) => makeSelection(nextNodes, doc);
            const sel = {
                length: list.length,
                each(fn) {
                    for (let i = 0; i < list.length; i++) {
                        const res = fn.call(list[i], i, list[i]);
                        if (res === false) break;
                    }
                    return sel;
                },
                map(fn) {
                    const out = [];
                    for (let i = 0; i < list.length; i++) {
                        const res = fn.call(list[i], i, list[i]);
                        if (res !== null && res !== undefined) out.push(res);
                    }
                    const wrapped = wrap(out.every(isHostNode) ? out : []);
                    wrapped._mappedValues = out;
                    wrapped.get = (idx) => (idx === undefined ? out : out[idx < 0 ? out.length + idx : idx]);
                    wrapped.toArray = () => out.slice();
                    return wrapped;
                },
                get(idx) {
                    if (idx === undefined) return list.slice();
                    return list[idx < 0 ? list.length + idx : idx];
                },
                toArray() {
                    return list.slice();
                },
                eq(idx) {
                    const i = idx < 0 ? list.length + idx : idx;
                    return wrap(list[i] ? [list[i]] : []);
                },
                slice(start, end) {
                    return wrap(list.slice(start, end));
                },
                first() {
                    return wrap(list.length ? [list[0]] : []);
                },
                last() {
                    return wrap(list.length ? [list[list.length - 1]] : []);
                },
                index() {
                    if (!list.length) return -1;
                    const parent = list[0].parent();
                    if (!parent) return 0;
                    const sibs = parent.children();
                    return sibs.findIndex((c) => c._nodeKey === list[0]._nodeKey);
                },
                is(pred) {
                    if (!list.length) return false;
                    if (typeof pred === "function") return list.some((n, i) => Boolean(pred.call(n, i, n)));
                    const css = String(pred);
                    return list.some((n) => n.matches(css));
                },
                find(query) {
                    const found = [];
                    for (const node of list) {
                        const matches = node.select(String(query));
                        for (const m of matches) found.push(m);
                    }
                    return wrap(found);
                },
                filter(pred) {
                    if (typeof pred === "function") {
                        return wrap(list.filter((n, i) => Boolean(pred.call(n, i, n))));
                    }
                    const css = String(pred);
                    return wrap(list.filter((n) => n.matches(css)));
                },
                not(pred) {
                    if (typeof pred === "function") {
                        return wrap(list.filter((n, i) => !pred.call(n, i, n)));
                    }
                    const css = String(pred);
                    return wrap(list.filter((n) => !n.matches(css)));
                },
                children(css) {
                    const out = [];
                    for (const node of list) {
                        for (const ch of node.children()) {
                            if (!css || ch.matches(String(css))) out.push(ch);
                        }
                    }
                    return wrap(out);
                },
                parent(css) {
                    const out = [];
                    for (const node of list) {
                        const p = node.parent();
                        if (p && (!css || p.matches(String(css)))) out.push(p);
                    }
                    return wrap(out);
                },
                closest(css) {
                    const q = String(css);
                    const out = [];
                    for (const node of list) {
                        let cur = node;
                        while (cur) {
                            if (cur.matches(q)) {
                                out.push(cur);
                                break;
                            }
                            cur = cur.parent();
                        }
                    }
                    return wrap(out);
                },
                next(css) {
                    const out = [];
                    for (const node of list) {
                        const nx = node.next();
                        if (nx && (!css || nx.matches(String(css)))) out.push(nx);
                    }
                    return wrap(out);
                },
                prev(css) {
                    const out = [];
                    for (const node of list) {
                        const pv = node.prev();
                        if (pv && (!css || pv.matches(String(css)))) out.push(pv);
                    }
                    return wrap(out);
                },
                siblings(css) {
                    const out = [];
                    for (const node of list) {
                        const p = node.parent();
                        if (!p) continue;
                        for (const ch of p.children()) {
                            if (ch._nodeKey !== node._nodeKey && (!css || ch.matches(String(css)))) {
                                out.push(ch);
                            }
                        }
                    }
                    return wrap(out);
                },
                hasClass(name) {
                    return list.length ? list[0].hasClass(String(name)) : false;
                },
                attr(name) {
                    if (!list.length) return undefined;
                    if (name === undefined) return list[0].attrs();
                    const v = list[0].attr(String(name));
                    return v === null ? undefined : v;
                },
                prop(name) {
                    return sel.attr(name);
                },
                data(name) {
                    const raw = sel.attr("data-" + String(name));
                    if (raw === undefined) return undefined;
                    try { return JSON.parse(raw); } catch { return raw; }
                },
                text() {
                    return list.map((n) => n.text()).join("");
                },
                html() {
                    return list.length ? list[0].html() : null;
                },
                val() {
                    return sel.attr("value") || "";
                },
                addClass() { return sel; },
                removeClass() { return sel; },
                removeAttr() { return sel; },
                remove() { return sel; },
                empty() { return sel; },
            };
            for (let i = 0; i < list.length; i++) {
                sel[i] = list[i];
            }
            return sel;
        };

        const load = (rawHtml) => {
            const doc = vHtml.parse(String(rawHtml || ""));
            const $ = (selector, context) => {
                if (!selector) return makeSelection([], doc);
                if (typeof selector === "object") {
                    if (Array.isArray(selector)) return makeSelection(selector, doc);
                    if (isHostNode(selector)) return makeSelection([selector], doc);
                    if (typeof selector.get === "function") return selector;
                    return makeSelection([], doc);
                }
                const q = String(selector).trim();
                if (q.startsWith("<")) {
                    const frag = vHtml.parse(q);
                    const children = frag.select("body > *");
                    return makeSelection(children, frag);
                }
                if (context) {
                    return $(context).find(q);
                }
                return makeSelection(doc.select(q), doc);
            };
            $.html = (target) => {
                if (target) return $(target).html() || "";
                return doc.html();
            };
            $.text = (target) => {
                if (target) return $(target).text();
                return "";
            };
            $.load = load;
            return $;
        };

        return { load };
    };

    const cheerioShim = vHtml ? createCheerioModule() : { load: () => () => ({ length: 0 }) };
    if (typeof globalThis.cheerio === "undefined") {
        globalThis.cheerio = cheerioShim;
    }

    // 4. CryptoJS shim backed by vessel.crypto
    const makeWordArray = (text, encoding = "utf8") => ({
        _raw: String(text || ""),
        _enc: encoding,
        toString(enc) {
            if (enc === CryptoJSShim.enc.Base64) {
                return this._enc === "base64" ? this._raw : vCrypto.base64Encode(this._raw);
            }
            if (enc === CryptoJSShim.enc.Hex) {
                return this._enc === "hex" ? this._raw : vCrypto.hexEncode(this._raw);
            }
            if (this._enc === "base64") return vCrypto.base64Decode(this._raw);
            return this._raw;
        },
    });

    const hashWrap = (hexStr) => ({
        toString(enc) {
            if (enc === CryptoJSShim.enc.Base64) {
                return vCrypto.base64Encode(vCrypto.hexDecode(hexStr));
            }
            return hexStr;
        },
    });

    const CryptoJSShim = {
        enc: {
            Utf8: {
                parse: (s) => makeWordArray(s, "utf8"),
                stringify: (w) => (w && typeof w.toString === "function" ? w.toString(CryptoJSShim.enc.Utf8) : String(w || "")),
            },
            Hex: {
                parse: (s) => makeWordArray(s, "hex"),
                stringify: (w) => (w && typeof w.toString === "function" ? w.toString(CryptoJSShim.enc.Hex) : String(w || "")),
            },
            Base64: {
                parse: (s) => makeWordArray(s, "base64"),
                stringify: (w) => (w && typeof w.toString === "function" ? w.toString(CryptoJSShim.enc.Base64) : String(w || "")),
            },
        },
        MD5: (input) => hashWrap(vCrypto.md5(String(input || ""))),
        SHA1: (input) => hashWrap(vCrypto.sha1(String(input || ""))),
        SHA256: (input) => hashWrap(vCrypto.sha256(String(input || ""))),
        HmacSHA256: (msg, key) => hashWrap(vCrypto.hmacSha256(String(key || ""), String(msg || ""))),
        AES: {
            encrypt(plain, key, cfg = {}) {
                const k = key && key._raw !== undefined ? key._raw : String(key || "");
                const iv = cfg && cfg.iv ? (cfg.iv._raw !== undefined ? cfg.iv._raw : String(cfg.iv)) : undefined;
                const cipher = vCrypto.aesEncrypt(String(plain || ""), k, iv, "cbc");
                return { toString: () => cipher };
            },
            decrypt(cipher, key, cfg = {}) {
                const c = cipher && cipher.ciphertext ? cipher.ciphertext : String(cipher || "");
                const k = key && key._raw !== undefined ? key._raw : String(key || "");
                const iv = cfg && cfg.iv ? (cfg.iv._raw !== undefined ? cfg.iv._raw : String(cfg.iv)) : undefined;
                const out = vCrypto.aesDecrypt(c, k, iv, "cbc");
                return makeWordArray(out, "utf8");
            },
        },
    };
    if (typeof globalThis.CryptoJS === "undefined") {
        globalThis.CryptoJS = CryptoJSShim;
    }

    // 5. Callable Axios shim backed by fetch() (supports baseURL, params, headers, timeout, create)
    const buildAxiosUrl = (baseURL, rawUrl, params) => {
        let target = String(rawUrl || "");
        if (baseURL && !/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(target)) {
            target = `${String(baseURL).replace(/\/+$/, "")}/${target.replace(/^\/+/, "")}`;
        }
        if (params && typeof params === "object") {
            const usp = params instanceof globalThis.URLSearchParams ? params : new globalThis.URLSearchParams();
            if (!(params instanceof globalThis.URLSearchParams)) {
                for (const [k, v] of Object.entries(params)) {
                    if (v !== undefined && v !== null) usp.append(k, String(v));
                }
            }
            const qs = usp.toString();
            if (qs) target += (target.includes("?") ? "&" : "?") + qs;
        }
        return target;
    };
    const makeAxiosInstance = (defaults = {}) => {
        const baseDefaults = defaults && typeof defaults === "object" ? { ...defaults } : {};
        const req = async (method, url, body, config = {}) => {
            const cfg = config && typeof config === "object" ? config : {};
            const commonHdrs = (baseDefaults.headers && baseDefaults.headers.common) || {};
            const defaultHdrs = (baseDefaults.headers && !baseDefaults.headers.common) ? baseDefaults.headers : (baseDefaults.headers || {});
            const mergedHeaders = {};
            for (const src of [commonHdrs, defaultHdrs, cfg.headers || {}]) {
                if (src && typeof src === "object") {
                    for (const [hk, hv] of Object.entries(src)) {
                        if (hk !== "common" && hv !== undefined && hv !== null) mergedHeaders[hk] = String(hv);
                    }
                }
            }
            const mergedParams = { ...(baseDefaults.params || {}), ...(cfg.params || {}) };
            const finalUrl = buildAxiosUrl(cfg.baseURL ?? baseDefaults.baseURL, url, Object.keys(mergedParams).length ? mergedParams : undefined);
            const payload = body !== undefined ? body : cfg.data;
            let encodedBody;
            if (payload !== undefined && payload !== null) {
                if (typeof payload === "string" || payload instanceof globalThis.URLSearchParams || payload instanceof globalThis.FormData) {
                    encodedBody = payload;
                } else {
                    encodedBody = JSON.stringify(payload);
                }
            }
            const timeout = typeof cfg.timeout === "number" ? cfg.timeout : (typeof baseDefaults.timeout === "number" ? baseDefaults.timeout : undefined);
            const resp = await globalThis.fetch(finalUrl, {
                method: String(method || "GET").toUpperCase(),
                headers: mergedHeaders,
                body: encodedBody,
                timeout,
                redirect: cfg.maxRedirects === 0 ? "manual" : undefined,
            });
            const text = await resp.text();
            let data = text;
            if (cfg.responseType === "arraybuffer") {
                data = await resp.arrayBuffer();
            } else if (cfg.responseType !== "text") {
                try { data = JSON.parse(text); } catch { data = text; }
            }
            const hdrObj = {};
            if (resp.headers && typeof resp.headers.forEach === "function") {
                resp.headers.forEach((v, k) => { hdrObj[k] = v; });
            }
            return { data, status: resp.status, statusText: resp.statusText, headers: hdrObj, config: cfg };
        };
        const instance = (arg1, arg2) => {
            if (typeof arg1 === "string") {
                const cfg = arg2 && typeof arg2 === "object" ? arg2 : {};
                return req(cfg.method || "GET", arg1, cfg.data, cfg);
            }
            const cfg = arg1 && typeof arg1 === "object" ? arg1 : {};
            return req(cfg.method || "GET", cfg.url, cfg.data, cfg);
        };
        instance.get = (url, cfg) => req("GET", url, undefined, cfg);
        instance.head = (url, cfg) => req("HEAD", url, undefined, cfg);
        instance.post = (url, body, cfg) => req("POST", url, body, cfg);
        instance.put = (url, body, cfg) => req("PUT", url, body, cfg);
        instance.patch = (url, body, cfg) => req("PATCH", url, body, cfg);
        instance.delete = (url, cfg) => req("DELETE", url, cfg && cfg.data, cfg);
        instance.request = (cfg) => req((cfg && cfg.method) || "GET", cfg && cfg.url, cfg && cfg.data, cfg);
        instance.create = (nextDefaults) => makeAxiosInstance({ ...baseDefaults, ...(nextDefaults || {}) });
        instance.defaults = { headers: { common: {}, ...(baseDefaults.headers || {}) }, ...baseDefaults };
        return instance;
    };
    const axiosShim = makeAxiosInstance();

    // 6. Lightweight HTML entity & URL / querystring shims
    const decodeHtmlEntities = (str) => String(str || "")
        .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) => String.fromCodePoint(parseInt(hex, 16)))
        .replace(/&#(\d+);/g, (_, dec) => String.fromCodePoint(parseInt(dec, 10)))
        .replace(/&(quot|apos|amp|lt|gt|nbsp);/g, (_, name) => ({ quot: '"', apos: "'", amp: "&", lt: "<", gt: ">", nbsp: " " }[name] || `&${name};`));
    const htmlEntitiesShim = { decode: decodeHtmlEntities, decodeHTML: decodeHtmlEntities, decodeXML: decodeHtmlEntities };
    const urlModuleShim = {
        URL: globalThis.URL,
        URLSearchParams: globalThis.URLSearchParams,
        parse: (raw) => {
            try {
                const u = new globalThis.URL(String(raw));
                return { href: u.href, protocol: u.protocol, host: u.host, hostname: u.hostname, port: u.port, pathname: u.pathname, search: u.search, hash: u.hash, query: Object.fromEntries(u.searchParams.entries()) };
            } catch {
                return {};
            }
        },
        resolve: (from, to) => {
            try { return new globalThis.URL(String(to), String(from)).toString(); } catch { return String(to); }
        },
    };
    const querystringShim = {
        parse: (qs) => Object.fromEntries(new globalThis.URLSearchParams(String(qs || "")).entries()),
        stringify: (obj) => new globalThis.URLSearchParams(obj || {}).toString(),
        encode: (obj) => new globalThis.URLSearchParams(obj || {}).toString(),
        decode: (qs) => Object.fromEntries(new globalThis.URLSearchParams(String(qs || "")).entries()),
    };

    // 7. Safe read-only path + disabled-disk fs shim
    const pathShim = {
        sep: "/",
        join: (...parts) => parts.filter(Boolean).join("/").replace(/\/+/g, "/"),
        resolve: (...parts) => "/" + parts.filter(Boolean).join("/").replace(/\/+/g, "/").replace(/^\/+/, ""),
        dirname: (p) => String(p || "").replace(/\/[^/]*$/, "") || "/",
        basename: (p, ext) => {
            const b = String(p || "").split("/").pop() || "";
            return ext && b.endsWith(ext) ? b.slice(0, -ext.length) : b;
        },
        extname: (p) => {
            const b = String(p || "").split("/").pop() || "";
            const idx = b.lastIndexOf(".");
            return idx > 0 ? b.slice(idx) : "";
        },
    };
    const fsShim = {
        existsSync: () => false,
        readFileSync: () => { throw new Error("Local filesystem read disabled inside Vessel plugin sandbox"); },
    };

    // 8. Global require() resolver for Nuvio / Node community dependencies
    if (typeof globalThis.require !== "function") {
        globalThis.require = (rawId) => {
            const id = String(rawId || "").replace(/^node:/, "");
            switch (id) {
                case "cheerio":
                case "cheerio-without-node-native":
                case "react-native-cheerio":
                    return cheerioShim;
                case "crypto-js":
                case "crypto":
                    return CryptoJSShim;
                case "axios":
                    return axiosShim;
                case "fs":
                    return fsShim;
                case "path":
                    return pathShim;
                case "url":
                    return urlModuleShim;
                case "querystring":
                    return querystringShim;
                case "buffer":
                    return { Buffer: globalThis.Buffer };
                case "he":
                case "entities":
                    return htmlEntitiesShim;
                default:
                    throw new Error(`Unsupported require('${rawId}') inside Vessel plugin sandbox`);
            }
        };
    }
})();
"##;
