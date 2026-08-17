//! 数据加密模块
//!
//! 基于设计文档 §3.1 实现，提供 AES-256-GCM 数据加密/解密能力。
//! 支持两种密钥派生方式：
//! 1. 设备指纹模式：使用设备硬件信息派生密钥，仅限当前设备
//! 2. 主密码模式：使用用户提供的主密码派生密钥，支持跨设备恢复
//!
//! # 加密方案
//! - 算法：AES-256-GCM（认证加密，同时提供保密性和完整性）
//! - 密钥派生：PBKDF2-SHA256（600,000 次迭代）
//! - IV/Nonce：每次加密随机生成 96-bit nonce
//! - 存储格式：base64(nonce || ciphertext || tag)

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use pbkdf2::pbkdf2_hmac;
use sha2::Sha256;

/// PBKDF2 迭代次数（OWASP 2023 推荐值）
const PBKDF2_ITERATIONS: u32 = 600_000;
/// AES-256 密钥长度（字节）
const KEY_LENGTH: usize = 32;
/// AES-GCM nonce 长度（字节）
const NONCE_LENGTH: usize = 12;

/// 加密模块
///
/// 提供数据加密/解密能力，支持设备指纹和主密码两种密钥来源。
#[derive(Clone)]
pub struct Encryption {
    /// AES-256-GCM 加密器
    cipher: Aes256Gcm,
}

impl Encryption {
    /// 使用原始密钥字节创建加密模块
    ///
    /// # Arguments
    /// * `key` - 32 字节的 AES-256 密钥
    pub fn new(key: &[u8]) -> Result<Self> {
        if key.len() < KEY_LENGTH {
            anyhow::bail!(
                "加密密钥长度不足: 需要 {} 字节，实际 {} 字节",
                KEY_LENGTH,
                key.len()
            );
        }

        let cipher = Aes256Gcm::new_from_slice(&key[..KEY_LENGTH])
            .map_err(|e| anyhow::anyhow!("无法创建 AES-256-GCM 加密器: {}", e))?;

        tracing::debug!("Encryption module initialized");
        Ok(Self { cipher })
    }

    /// 使用主密码创建加密模块（跨设备恢复场景）
    ///
    /// 通过 PBKDF2-SHA256 从主密码派生 AES-256 密钥。
    ///
    /// # Arguments
    /// * `master_password` - 用户提供的主密码
    /// * `salt` - PBKDF2 盐值（通常存储在数据库中）
    pub fn from_master_password(master_password: &str, salt: &[u8]) -> Result<Self> {
        let key = Self::derive_key(master_password, salt);
        Self::new(&key)
    }

    /// 使用 PBKDF2-SHA256 从密码派生密钥
    ///
    /// # Arguments
    /// * `password` - 密码字符串
    /// * `salt` - 盐值
    ///
    /// # Returns
    /// 32 字节的派生密钥
    pub fn derive_key(password: &str, salt: &[u8]) -> Vec<u8> {
        let mut key = vec![0u8; KEY_LENGTH];
        pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);
        key
    }

    /// 生成随机盐值
    ///
    /// # Returns
    /// 32 字节的随机盐值
    pub fn generate_salt() -> Vec<u8> {
        use aes_gcm::aead::OsRng;
        use rand::RngCore;
        let mut salt = vec![0u8; 32];
        OsRng.fill_bytes(&mut salt);
        salt
    }

    /// 加密数据
    ///
    /// # Arguments
    /// * `plaintext` - 待加密的明文数据
    ///
    /// # Returns
    /// base64 编码的密文（格式: nonce || ciphertext || tag）
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String> {
        // 生成随机 nonce
        let nonce_bytes: [u8; NONCE_LENGTH] = rand::random();
        let nonce = Nonce::from_slice(&nonce_bytes);

        // 加密数据
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow::anyhow!("加密失败: {}", e))?;

        // 拼接 nonce + ciphertext，然后 base64 编码
        let mut combined = Vec::with_capacity(NONCE_LENGTH + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// 解密数据
    ///
    /// # Arguments
    /// * `encrypted_b64` - base64 编码的密文
    ///
    /// # Returns
    /// 解密后的明文数据
    pub fn decrypt(&self, encrypted_b64: &str) -> Result<Vec<u8>> {
        // base64 解码
        let combined = BASE64.decode(encrypted_b64).context("base64 解码失败")?;

        if combined.len() < NONCE_LENGTH {
            anyhow::bail!("密文数据太短，无法提取 nonce");
        }

        // 分离 nonce 和 ciphertext
        let (nonce_bytes, ciphertext) = combined.split_at(NONCE_LENGTH);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 解密数据
        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| anyhow::anyhow!("解密失败（密钥可能不正确）: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建测试用的加密模块
    fn create_test_encryption() -> Encryption {
        let key = vec![0u8; KEY_LENGTH];
        Encryption::new(&key).unwrap()
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let enc = create_test_encryption();
        let plaintext = b"Hello, Navis Go!";

        // 加密
        let encrypted = enc.encrypt(plaintext).unwrap();
        assert!(!encrypted.is_empty());

        // 解密
        let decrypted = enc.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_different_results() {
        let enc = create_test_encryption();
        let plaintext = b"same data";

        // 每次加密结果不同（因为随机 nonce）
        let enc1 = enc.encrypt(plaintext).unwrap();
        let enc2 = enc.encrypt(plaintext).unwrap();
        assert_ne!(enc1, enc2);

        // 但都能正确解密
        assert_eq!(enc.decrypt(&enc1).unwrap(), plaintext);
        assert_eq!(enc.decrypt(&enc2).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_with_wrong_key() {
        let enc1 = create_test_encryption();
        let enc2 = Encryption::new(&vec![1u8; KEY_LENGTH]).unwrap();

        let encrypted = enc1.encrypt(b"secret data").unwrap();
        let result = enc2.decrypt(&encrypted);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_invalid_data() {
        let enc = create_test_encryption();
        assert!(enc.decrypt("not-valid-base64!!!").is_err());
        assert!(enc.decrypt(&BASE64.encode(b"short")).is_err());
    }

    #[test]
    fn test_master_password_derivation() {
        let salt = Encryption::generate_salt();
        let enc1 = Encryption::from_master_password("my_secure_password", &salt).unwrap();
        let enc2 = Encryption::from_master_password("my_secure_password", &salt).unwrap();

        let plaintext = b"cross-device data";
        let encrypted = enc1.encrypt(plaintext).unwrap();
        let decrypted = enc2.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_different_passwords_different_keys() {
        let salt = Encryption::generate_salt();
        let enc1 = Encryption::from_master_password("password1", &salt).unwrap();
        let enc2 = Encryption::from_master_password("password2", &salt).unwrap();

        let encrypted = enc1.encrypt(b"data").unwrap();
        assert!(enc2.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_derive_key_deterministic() {
        let salt = b"test_salt_value_1234567890123456";
        let key1 = Encryption::derive_key("password", salt);
        let key2 = Encryption::derive_key("password", salt);
        assert_eq!(key1, key2);
        assert_eq!(key1.len(), KEY_LENGTH);
    }

    #[test]
    fn test_generate_salt_unique() {
        let salt1 = Encryption::generate_salt();
        let salt2 = Encryption::generate_salt();
        assert_ne!(salt1, salt2);
        assert_eq!(salt1.len(), 32);
    }
}
