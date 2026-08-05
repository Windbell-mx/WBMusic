//! 登录会话管理
//!
//! 负责将各音乐源的登录凭据（Cookie / Token）持久化到本地，
//! 重启应用后自动恢复登录态，无需重复登录。
//!
//! 安全设计：
//! - 敏感凭据（Cookie）存入系统凭据库（Windows 凭据管理器 / macOS 钥匙串 / Linux Secret Service），
//!   不再明文写入 session.json
//! - session.json 仅保存非敏感信息（登录状态、昵称、用户 ID）

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::music_provider::MusicProviderKind;

/// 系统凭据库服务名
const KEYRING_SERVICE: &str = "wbmusic";

/// 某个音乐源的持久化会话
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderSession {
    /// 是否已登录
    pub logged_in: bool,
    /// 昵称（展示用）
    pub nickname: Option<String>,
    /// 用户 ID
    pub user_id: Option<String>,
    /// 登录凭据（Cookie / Token 原始字符串）
    ///
    /// 仅保存在内存中；持久化通过系统凭据库（keyring）完成，
    /// 序列化时跳过，避免明文落盘。
    #[serde(skip)]
    pub credential: Option<String>,
}

/// 全部音乐源的会话存储
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionStore {
    pub qq_music: ProviderSession,
    pub netease: ProviderSession,
}

impl SessionStore {
    /// 会话文件路径：{app_data_dir}/session.json
    pub fn file_path(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join("session.json")
    }

    /// 从磁盘加载会话，不存在则返回默认空会话
    pub fn load(app_data_dir: &Path) -> Self {
        let path = Self::file_path(app_data_dir);
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// 保存会话到磁盘（原子写入：先写临时文件再改名）
    pub fn save(&self, app_data_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(app_data_dir)
            .map_err(|e| format!("创建数据目录失败: {}", e))?;
        let path = Self::file_path(app_data_dir);
        let tmp = path.with_extension("json.tmp");
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化会话失败: {}", e))?;
        fs::write(&tmp, content).map_err(|e| format!("写入会话文件失败: {}", e))?;
        fs::rename(&tmp, &path).map_err(|e| format!("保存会话文件失败: {}", e))?;
        Ok(())
    }

    /// Windows 凭据管理器单个条目的容量上限。
    ///
    /// `CRED_MAX_CREDENTIAL_BLOB_SIZE` = 2560 **字节**（非字符），
    /// keyring 按 UTF-16 编码后字节数校验（`encode_utf16().count() * 2`），
    /// 即单块最多 1280 个 UTF-16 代码单元。
    /// 这里取 1200 字符留出余量，超长凭据（如网易云 Cookie 含超长 MUSIC_U）
    /// 会拆分为多个条目分块存储。
    const KEYRING_CHUNK_MAX: usize = 1200;

    /// 凭据库条目名称：wbmusic / {provider}[#{chunk}]
    ///
    /// 第一块用基础名（兼容旧数据），后续块追加 `#1`、`#2` 后缀。
    fn keyring_account(kind: MusicProviderKind, chunk: usize) -> String {
        let base = match kind {
            MusicProviderKind::QqMusic => "qq_music",
            MusicProviderKind::Netease => "netease",
        };
        if chunk == 0 {
            base.to_string()
        } else {
            format!("{}#{}", base, chunk)
        }
    }

    /// 尝试读取单个凭据条目（不存在则返回 Ok(None)）
    fn read_entry(account: &str) -> Result<Option<String>, String> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, account)
            .map_err(|e| format!("创建凭据条目失败: {}", e))?;
        match entry.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("读取凭据失败: {}", e)),
        }
    }

    /// 从系统凭据库读取某音乐源的凭据（自动拼接分块）
    pub fn load_credential(&self, kind: MusicProviderKind) -> Result<Option<String>, String> {
        // 第一块
        let Some(first) = Self::read_entry(&Self::keyring_account(kind, 0))? else {
            return Ok(None);
        };
        // 后续块
        let mut parts = vec![first];
        let mut chunk = 1;
        loop {
            match Self::read_entry(&Self::keyring_account(kind, chunk))? {
                Some(p) => parts.push(p),
                None => break,
            }
            chunk += 1;
        }
        Ok(Some(parts.join("")))
    }

    /// 将凭据写入系统凭据库（超长自动分块）
    pub fn save_credential(&self, kind: MusicProviderKind, credential: &str) -> Result<(), String> {
        // 先清理旧分块，避免残留
        self.delete_credential(kind)?;

        // 按字符边界切分：避免把多字节 UTF-8 字符切成两半
        // （凭据通常为 ASCII，但 Cookie 中可能出现中文等非 ASCII 字符）
        let mut chunks: Vec<&str> = Vec::new();
        let mut start = 0;
        for (idx, _) in credential.char_indices() {
            if idx - start >= Self::KEYRING_CHUNK_MAX {
                chunks.push(&credential[start..idx]);
                start = idx;
            }
        }
        if start < credential.len() {
            chunks.push(&credential[start..]);
        }

        for (i, chunk) in chunks.iter().enumerate() {
            let entry = keyring::Entry::new(KEYRING_SERVICE, &Self::keyring_account(kind, i))
                .map_err(|e| format!("创建凭据条目失败: {}", e))?;
            entry
                .set_password(chunk)
                .map_err(|e| format!("保存凭据失败: {}", e))?;
        }
        Ok(())
    }

    /// 从系统凭据库删除某音乐源的凭据（删除全部块）
    pub fn delete_credential(&self, kind: MusicProviderKind) -> Result<(), String> {
        let mut chunk = 0;
        loop {
            let account = Self::keyring_account(kind, chunk);
            let entry = keyring::Entry::new(KEYRING_SERVICE, &account)
                .map_err(|e| format!("创建凭据条目失败: {}", e))?;
            match entry.delete_credential() {
                Ok(()) => {}
                Err(keyring::Error::NoEntry) => {
                    // 基础块不存在不代表没有孤儿分块（例如并行竞态或异常中断），
                    // 因此继续尝试删除后续分块，直到连续遇到不存在的块为止。
                    // 注意：chunk > 0 且当前块不存在时即可安全停止——
                    // 因为分块编号是连续的，遇到第一个缺失块说明后面也没有。
                    if chunk > 0 {
                        break;
                    }
                }
                Err(e) => return Err(format!("删除凭据失败: {}", e)),
            }
            chunk += 1;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    /// Windows 凭据管理器不保证并发读写同一条目的顺序（keyring 文档明确警告）。
    /// 产品中凭据操作是单线程的（登录/登出/启动恢复串行执行），
    /// 因此测试用全局锁串行化，模拟产品真实场景并避免测试互相污染。
    fn test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// 测试守卫：测试开始前备份该音乐源的真实凭据，测试结束后（Drop）自动恢复。
    /// 防止测试破坏用户已保存的登录凭据（测试与产品共用系统凭据库）。
    struct CredentialGuard {
        kind: MusicProviderKind,
        backup: Option<String>,
    }

    impl CredentialGuard {
        fn new(kind: MusicProviderKind) -> Self {
            let store = SessionStore::default();
            let backup = store.load_credential(kind).unwrap_or(None);
            Self { kind, backup }
        }
    }

    impl Drop for CredentialGuard {
        fn drop(&mut self) {
            let store = SessionStore::default();
            match &self.backup {
                Some(c) => {
                    if let Err(e) = store.save_credential(self.kind, c) {
                        eprintln!("恢复凭据失败: {}", e);
                    }
                }
                None => {
                    if let Err(e) = store.delete_credential(self.kind) {
                        eprintln!("清理测试凭据失败: {}", e);
                    }
                }
            }
        }
    }

    /// 验证超长凭据（超过 Windows 凭据管理器 2560 字符限制）的保存/读取往返
    #[test]
    fn test_long_credential_roundtrip() {
        let _guard = test_lock().lock().unwrap();
        let store = SessionStore::default();
        let kind = MusicProviderKind::Netease;
        let _cred_backup = CredentialGuard::new(kind);

        // 构造超过平台限制的凭据（UTF-16 编码后超 2560 字节，含中文验证字符边界切分）
        let long = format!(
            "MUSIC_U={};{};__csrf=abcd1234",
            "x".repeat(3000),
            "中文Cookie值".repeat(300)
        );
        assert!(long.encode_utf16().count() * 2 > 2560);

        store.save_credential(kind, &long).unwrap();
        let loaded = store.load_credential(kind).unwrap().unwrap();
        assert_eq!(loaded, long);

        // 删除后应返回 None
        store.delete_credential(kind).unwrap();
        assert!(store.load_credential(kind).unwrap().is_none());
    }

    /// 验证短凭据正常存取
    #[test]
    fn test_short_credential_roundtrip() {
        let _guard = test_lock().lock().unwrap();
        let store = SessionStore::default();
        let kind = MusicProviderKind::QqMusic;
        let _cred_backup = CredentialGuard::new(kind);

        store.save_credential(kind, "uin=123;qm_keyst=abc").unwrap();
        let loaded = store.load_credential(kind).unwrap().unwrap();
        assert_eq!(loaded, "uin=123;qm_keyst=abc");

        store.delete_credential(kind).unwrap();
        assert!(store.load_credential(kind).unwrap().is_none());
    }

    /// 验证 delete 会删除所有分块（含基础块），且 load 会拼接所有块
    #[test]
    fn test_delete_removes_all_chunks() {
        let _guard = test_lock().lock().unwrap();
        let store = SessionStore::default();
        let kind = MusicProviderKind::Netease;
        let _cred_backup = CredentialGuard::new(kind);

        let long = format!(
            "MUSIC_U={};{};__csrf=abcd1234",
            "x".repeat(3000),
            "中文Cookie值".repeat(300)
        );
        store.save_credential(kind, &long).unwrap();

        // 保存后：基础块和所有分块都应存在
        let mut chunk = 0;
        loop {
            let account = SessionStore::keyring_account(kind, chunk);
            let entry = keyring::Entry::new(KEYRING_SERVICE, &account).unwrap();
            match entry.get_password() {
                Ok(_) => {
                    assert!(chunk < 20, "分块数量异常");
                    chunk += 1;
                }
                Err(keyring::Error::NoEntry) => break,
                Err(e) => panic!("读取分块失败: {}", e),
            }
        }
        assert!(chunk >= 2, "长凭据应拆分为至少 2 块，实际 {}", chunk);
        eprintln!("分块数量: {}", chunk);

        store.delete_credential(kind).unwrap();

        // 删除后：所有块（0..chunk+1）都应不存在
        for i in 0..chunk + 2 {
            let account = SessionStore::keyring_account(kind, i);
            let entry = keyring::Entry::new(KEYRING_SERVICE, &account).unwrap();
            assert!(
                matches!(entry.get_password(), Err(keyring::Error::NoEntry)),
                "分块 {} 应已被删除",
                i
            );
        }
    }
}
