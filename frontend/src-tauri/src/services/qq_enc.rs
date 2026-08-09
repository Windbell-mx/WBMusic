//! QQ 音乐 musics.fcg 加密接口（sign + AES-GCM）
//!
//! 腾讯 `musics.fcg` 接口要求：
//!   1. `sign` 参数：由混淆 JS（`seg_full.js`）中 `ne._getSecuritySign` 生成
//!   2. POST body：明文 JSON 经 `oe.__cgiEncrypt`（AES-128-GCM + 自定义编码）加密
//!   3. 响应体：需用 `oe.__cgiDecrypt`（同步）解密后才能得到 JSON
//!
//! 实现方式：在专用线程中嵌入 rquickjs（QuickJS）执行腾讯混淆 JS，
//! AES-GCM 加解密由 Rust 侧 `aes-gcm` crate 真实实现，
//! 通过注册的 `__rs*` 桥接函数供 JS 调用（避免在 JS 中做不可靠的 crypto polyfill）。

use std::sync::mpsc;
use std::sync::OnceLock;

use aes_gcm::aead::Aead;
use aes_gcm::{Aes128Gcm, Aes256Gcm, KeyInit};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use log::{debug, error, info, warn};
use rquickjs::function::Func;
use rquickjs::promise::PromiseState;
use rquickjs::{Context, Promise, Runtime};

/// 腾讯 sign 段（混淆 JS），编译时内嵌
const SEG_JS: &str = include_str!("js/seg_full.js");
/// 腾讯 enc/dec 段（混淆 JS），编译时内嵌
const ENC_JS: &str = include_str!("js/enc_full.js");

/// JS 全局模拟 + polyfill（浏览器环境 shim）
const GLOBAL_POLYFILL: &str = r#"
var window = globalThis;
var self = globalThis;
var navigator = { userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' };
var location = {
  hostname: 'y.qq.com', host: 'y.qq.com', href: 'https://y.qq.com/',
  protocol: 'https:', origin: 'https://y.qq.com', pathname: '/',
  search: '', hash: '', port: ''
};
// setTimeout 用 queueMicrotask 模拟 (同步调度)
globalThis.setTimeout = function(fn, ms) { queueMicrotask(fn); return 0; };
globalThis.clearTimeout = function() {};
globalThis.setInterval = function() { return 0; };
globalThis.clearInterval = function() {};
globalThis.setImmediate = function(fn) { queueMicrotask(fn); return 0; };
globalThis.clearImmediate = function() {};
// 真实 base64（Rust 实现）
globalThis.atob = function(s) {
    var bytes = __rsAtob(s);
    var out = '';
    for (var i = 0; i < bytes.length; i++) out += String.fromCharCode(bytes[i]);
    return out;
};
globalThis.btoa = function(s) {
    var bytes = new Array(s.length);
    for (var i = 0; i < s.length; i++) bytes[i] = s.charCodeAt(i) & 0xff;
    return __rsBtoa(bytes);
};
// TextEncoder / TextDecoder (UTF-8)
globalThis.TextEncoder = function() {};
TextEncoder.prototype.encode = function(str) {
    var utf8 = unescape(encodeURIComponent(str));
    var arr = new Uint8Array(utf8.length);
    for (var i = 0; i < utf8.length; i++) arr[i] = utf8.charCodeAt(i);
    return arr;
};
globalThis.TextDecoder = function() {};
TextDecoder.prototype.decode = function(buf) {
    var bytes = (buf instanceof Uint8Array) ? buf : new Uint8Array(buf);
    var s = '';
    for (var i = 0; i < bytes.length; i++) s += String.fromCharCode(bytes[i]);
    return decodeURIComponent(escape(s));
};
// document (最小)
globalThis.document = {
    createElement: function(tag) { return { tagName: tag, style: {}, getContext: function(){ return null; }, appendChild: function(){}, setAttribute: function(){} }; },
    documentElement: {},
    createTextNode: function(){ return {}; },
    addEventListener: function(){},
    body: { appendChild: function(){}, style: {} }
};
// crypto (AES-GCM via Rust, 真实算法)
globalThis.crypto = {
    getRandomValues: function(arr) {
        for (var i = 0; i < arr.length; i++) arr[i] = (Math.random() * 256) | 0;
        return arr;
    },
    subtle: {
        importKey: function(format, keyData, algo) {
            var bytes = (keyData instanceof Uint8Array) ? Array.from(keyData) : Array.from(new Uint8Array(keyData));
            return Promise.resolve({ __key: bytes, __algo: algo && algo.name });
        },
        encrypt: function(algo, key, data) {
            var iv = (algo.iv instanceof Uint8Array) ? Array.from(algo.iv) : Array.from(new Uint8Array(algo.iv));
            var kb = Array.from(key.__key);
            var db = (data instanceof Uint8Array) ? Array.from(data) : Array.from(new Uint8Array(data));
            var b64 = __rsAesGcmEncrypt(kb, iv, db);
            if (b64.indexOf('__RS_ERR__') === 0) return Promise.reject(new Error(b64.slice(10)));
            var bin = atob(b64);
            var buf = new ArrayBuffer(bin.length);
            var u8 = new Uint8Array(buf);
            for (var i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
            return Promise.resolve(buf);
        },
        decrypt: function(algo, key, data) {
            var iv = (algo.iv instanceof Uint8Array) ? Array.from(algo.iv) : Array.from(new Uint8Array(algo.iv));
            var kb = Array.from(key.__key);
            var db = (data instanceof Uint8Array) ? Array.from(data) : Array.from(new Uint8Array(data));
            var b64 = __rsAesGcmDecrypt(kb, iv, db);
            if (b64.indexOf('__RS_ERR__') === 0) return Promise.reject(new Error(b64.slice(10)));
            var bin = atob(b64);
            var buf = new ArrayBuffer(bin.length);
            var u8 = new Uint8Array(buf);
            for (var i = 0; i < bin.length; i++) u8[i] = bin.charCodeAt(i);
            return Promise.resolve(buf);
        }
    }
};
"#;

/// 请求类型（发送给引擎线程）
enum Request {
    Sign {
        json: String,
        resp: mpsc::SyncSender<Result<String, String>>,
    },
    Encrypt {
        json: String,
        resp: mpsc::SyncSender<Result<String, String>>,
    },
    Decrypt {
        bytes: Vec<u8>,
        resp: mpsc::SyncSender<Result<String, String>>,
    },
}

/// 引擎线程句柄（全局单例，懒启动）
struct QqEncEngine {
    tx: mpsc::Sender<Request>,
}

static ENGINE: OnceLock<QqEncEngine> = OnceLock::new();

impl QqEncEngine {
    fn get() -> &'static QqEncEngine {
        ENGINE.get_or_init(|| {
            let (tx, rx) = mpsc::channel::<Request>();
            let handle = std::thread::Builder::new()
                .name("qq-enc-engine".into())
                .spawn(move || engine_main(rx))
                .expect("启动 QQ 加密引擎线程失败");
            debug!("QQ 加密引擎线程已启动: {:?}", handle.thread().id());
            QqEncEngine { tx }
        })
    }

    fn call(&self, req: Request) -> Result<(), String> {
        self.tx
            .send(req)
            .map_err(|_| "QQ 加密引擎线程已退出".to_string())
    }
}

/// 引擎线程主循环：持有 rquickjs Context（!Send，不能跨线程）
fn engine_main(rx: mpsc::Receiver<Request>) {
    let rt = match Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            error!("创建 QuickJS Runtime 失败: {}", e);
            return;
        }
    };
    let ctx = match Context::full(&rt) {
        Ok(c) => c,
        Err(e) => {
            error!("创建 QuickJS Context 失败: {}", e);
            return;
        }
    };
    ctx.with(|ctx| {
        if let Err(e) = init_ctx(&ctx) {
            error!("QQ 加密引擎初始化失败: {}", e);
            return;
        }
        info!("QQ 加密引擎就绪");
        while let Ok(req) = rx.recv() {
            match req {
                Request::Sign { json, resp } => {
                    let _ = resp.send(do_sign(&ctx, &json));
                }
                Request::Encrypt { json, resp } => {
                    let _ = resp.send(do_encrypt(&ctx, &json));
                }
                Request::Decrypt { bytes, resp } => {
                    let _ = resp.send(do_decrypt(&ctx, &bytes));
                }
            }
        }
        debug!("QQ 加密引擎线程退出");
    });
}

/// 初始化：注册 Rust 函数 + 全局 polyfill + 加载腾讯混淆 JS
fn init_ctx(ctx: &rquickjs::Ctx) -> Result<(), String> {
    // 1. 注册 Rust 桥接函数
    ctx.globals()
        .set(
            "__rsAesGcmEncrypt",
            Func::new(
                |key: Vec<u8>, iv: Vec<u8>, data: Vec<u8>| -> String {
                    match aes_gcm_encrypt_bytes(&key, &iv, &data) {
                        Ok(out) => BASE64.encode(out),
                        Err(e) => format!("__RS_ERR__{}", e),
                    }
                },
            ),
        )
        .map_err(|e| format!("注册 __rsAesGcmEncrypt 失败: {}", e))?;
    ctx.globals()
        .set(
            "__rsAesGcmDecrypt",
            Func::new(
                |key: Vec<u8>, iv: Vec<u8>, data: Vec<u8>| -> String {
                    match aes_gcm_decrypt_bytes(&key, &iv, &data) {
                        Ok(out) => BASE64.encode(out),
                        Err(e) => format!("__RS_ERR__{}", e),
                    }
                },
            ),
        )
        .map_err(|e| format!("注册 __rsAesGcmDecrypt 失败: {}", e))?;
    ctx.globals()
        .set(
            "__rsAtob",
            Func::new(|s: String| -> Vec<u8> {
                match BASE64.decode(s.trim()) {
                    Ok(v) => v,
                    Err(_) => Vec::new(),
                }
            }),
        )
        .map_err(|e| format!("注册 __rsAtob 失败: {}", e))?;
    ctx.globals()
        .set(
            "__rsBtoa",
            Func::new(|bytes: Vec<u8>| -> String { BASE64.encode(bytes) }),
        )
        .map_err(|e| format!("注册 __rsBtoa 失败: {}", e))?;

    // 2. 全局 polyfill
    ctx.eval::<(), _>(GLOBAL_POLYFILL)
        .map_err(|e| format!("polyfill 执行失败: {}", e))?;

    // 3. 加载 sign 段，暴露 _getSecuritySign
    let seg_js = format!("{}; globalThis.__signFn2 = ne._getSecuritySign;", SEG_JS);
    ctx.eval::<(), _>(seg_js)
        .map_err(|e| format!("加载 sign 段失败: {}", e))?;

    // 4. 加载 enc/dec 段
    ctx.eval::<(), _>(ENC_JS)
        .map_err(|e| format!("加载 enc 段失败: {}", e))?;

    Ok(())
}

/// 生成 sign 参数
fn do_sign(ctx: &rquickjs::Ctx, json: &str) -> Result<String, String> {
    ctx.eval::<String, _>(format!("globalThis.__signFn2({:?})", json))
        .map_err(|e| format!("sign 生成失败: {:?}", e))
}

/// 加密明文 JSON（AES-GCM），返回 base64 密文
fn do_encrypt(ctx: &rquickjs::Ctx, json: &str) -> Result<String, String> {
    let cipher: Promise = ctx
        .eval(format!("oe.__cgiEncrypt({:?})", json))
        .map_err(|e| format!("__cgiEncrypt 调用失败: {:?}", e))?;
    drive_promise(ctx, &cipher, "encrypt")
}

/// 解密响应字节（__cgiDecrypt 为同步函数，返回 JSON 字符串）
fn do_decrypt(ctx: &rquickjs::Ctx, bytes: &[u8]) -> Result<String, String> {
    let arr = bytes
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(",");
    ctx.eval::<String, _>(format!("oe.__cgiDecrypt(new Uint8Array([{}]).buffer)", arr))
        .map_err(|e| format!("__cgiDecrypt 失败: {:?}", e))
}

/// 驱动 Promise 直到 Resolved（快速 JS 微任务循环）
fn drive_promise(ctx: &rquickjs::Ctx, cipher: &Promise, what: &str) -> Result<String, String> {
    for i in 0..1000 {
        match cipher.state() {
            PromiseState::Resolved => {
                return match cipher.result::<String>() {
                    Some(Ok(v)) => Ok(v),
                    Some(Err(e)) => Err(format!("{} 结果读取失败: {:?}", what, e)),
                    None => Err(format!("{} 结果为 null", what)),
                };
            }
            PromiseState::Rejected => {
                return Err(format!("{} 被拒绝", what));
            }
            PromiseState::Pending => {
                ctx.execute_pending_job();
                if i > 0 && i % 100 == 0 {
                    warn!("{} 等待中 (iter {})", what, i);
                }
            }
        }
    }
    Err(format!("{} 超时（微任务未完成）", what))
}

/// AES-GCM 加密（真实算法，Rust 实现）
fn aes_gcm_encrypt_bytes(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let out = if key.len() == 16 {
        let cipher = Aes128Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
        cipher
            .encrypt(aes_gcm::Nonce::from_slice(iv), data)
            .map_err(|e| e.to_string())?
    } else if key.len() == 32 {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
        cipher
            .encrypt(aes_gcm::Nonce::from_slice(iv), data)
            .map_err(|e| e.to_string())?
    } else {
        return Err(format!("bad key len {}", key.len()));
    };
    Ok(out)
}

/// AES-GCM 解密（真实算法，Rust 实现）
fn aes_gcm_decrypt_bytes(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>, String> {
    let out = if key.len() == 16 {
        let cipher = Aes128Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
        cipher
            .decrypt(aes_gcm::Nonce::from_slice(iv), data)
            .map_err(|e| e.to_string())?
    } else if key.len() == 32 {
        let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
        cipher
            .decrypt(aes_gcm::Nonce::from_slice(iv), data)
            .map_err(|e| e.to_string())?
    } else {
        return Err(format!("bad key len {}", key.len()));
    };
    Ok(out)
}

/// 生成 g_tk（腾讯 hash，用于 comm 参数）
///
/// 与官网 JS 等价：JS 的 `<<` 是 32 位有符号移位（ToInt32 截断），
/// 且 `hash += (hash << 5) + c` 每轮都会把结果截回 32 位。
/// 因此用 u32 wrapping 算术逐位模拟，避免 i64 在 debug 下溢出 panic。
pub fn g_tk(key: &str) -> i64 {
    let mut n: u32 = 5381;
    for b in key.bytes() {
        n = n.wrapping_add(n.wrapping_shl(5).wrapping_add(b as u32));
    }
    (n & 0x7fff_ffff) as i64
}

/// 生成 musics.fcg 的 sign 参数
pub fn sign(json: &str) -> Result<String, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    QqEncEngine::get()
        .call(Request::Sign {
            json: json.to_string(),
            resp: tx,
        })
        .map_err(|e| e)?;
    rx.recv().map_err(|e| format!("sign 响应接收失败: {}", e))?
}

/// 加密明文 JSON，返回 base64 密文
pub fn encrypt(json: &str) -> Result<String, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    QqEncEngine::get()
        .call(Request::Encrypt {
            json: json.to_string(),
            resp: tx,
        })
        .map_err(|e| e)?;
    rx.recv().map_err(|e| format!("encrypt 响应接收失败: {}", e))?
}

/// 解密响应字节，返回明文 JSON 字符串
pub fn decrypt(bytes: &[u8]) -> Result<String, String> {
    let (tx, rx) = mpsc::sync_channel(1);
    QqEncEngine::get()
        .call(Request::Decrypt {
            bytes: bytes.to_vec(),
            resp: tx,
        })
        .map_err(|e| e)?;
    rx.recv().map_err(|e| format!("decrypt 响应接收失败: {}", e))?
}
