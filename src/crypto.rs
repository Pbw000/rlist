//! 加密工具模块
//!
//! 提供文件加密、哈希、签名等安全功能

use crate::error::{CryptoError, RlistResult};
use ring::digest::{Context, Digest, SHA256};
use ring::hmac::{self, Key};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use std::num::NonZeroU32;

/// 加密算法
#[derive(Debug, Clone, Copy)]
pub enum Cipher {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

impl Default for Cipher {
    fn default() -> Self {
        Cipher::Aes256Gcm
    }
}

/// 密钥派生配置
#[derive(Debug, Clone)]
pub struct KdfConfig {
    /// 迭代次数
    pub iterations: u32,
    /// 盐长度
    pub salt_len: usize,
}

impl Default for KdfConfig {
    fn default() -> Self {
        Self {
            iterations: 100_000,
            salt_len: 32,
        }
    }
}

/// 生成随机盐
pub fn generate_salt(len: usize) -> RlistResult<Vec<u8>> {
    let rng = SystemRandom::new();
    let mut salt = vec![0u8; len];
    rng.fill(&mut salt)
        .map_err(|_e| crate::error::RlistError::from(CryptoError::KeyDerivation))?;
    Ok(salt)
}

/// 从密码派生密钥
pub fn derive_key(password: &str, salt: &[u8], config: &KdfConfig) -> RlistResult<Vec<u8>> {
    let mut key = vec![0u8; 32]; // 256-bit key

    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        NonZeroU32::new(config.iterations).unwrap(),
        salt,
        password.as_bytes(),
        &mut key,
    );

    Ok(key)
}

/// 计算 SHA256 哈希
pub fn sha256(data: &[u8]) -> Digest {
    let mut context = Context::new(&SHA256);
    context.update(data);
    context.finish()
}

/// 计算 SHA256 十六进制字符串
pub fn sha256_hex(data: &[u8]) -> String {
    let digest = sha256(data);
    hex::encode(digest.as_ref())
}

/// HMAC-SHA256 签名
pub fn hmac_sign(key: &[u8], data: &[u8]) -> RlistResult<Vec<u8>> {
    let hmac_key = Key::new(hmac::HMAC_SHA256, key);
    Ok(hmac::sign(&hmac_key, data).as_ref().to_vec())
}

/// HMAC-SHA256 验证
pub fn hmac_verify(key: &[u8], data: &[u8], signature: &[u8]) -> RlistResult<bool> {
    let expected = hmac_sign(key, data)?;
    Ok(expected == signature)
}

/// 文件校验和
pub struct FileChecksum {
    context: Context,
}

impl FileChecksum {
    pub fn new() -> Self {
        Self {
            context: Context::new(&SHA256),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.context.update(data);
    }

    pub fn finish(self) -> Digest {
        self.context.finish()
    }

    pub fn finish_hex(self) -> String {
        hex::encode(self.finish().as_ref())
    }
}

impl Default for FileChecksum {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成安全随机 token
pub fn generate_token(len: usize) -> RlistResult<String> {
    let rng = SystemRandom::new();
    let mut bytes = vec![0u8; len];
    rng.fill(&mut bytes)
        .map_err(|_e| crate::error::RlistError::from(CryptoError::KeyDerivation))?;
    Ok(hex::encode(bytes))
}

/// Base64 编码
pub fn base64_encode(data: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose};
    general_purpose::STANDARD.encode(data)
}

/// Base64 解码
pub fn base64_decode(data: &str) -> RlistResult<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose};
    Ok(general_purpose::STANDARD
        .decode(data)
        .map_err(|_e| crate::error::RlistError::from(CryptoError::Encryption))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256_hex(data);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_hmac() {
        let key = b"secret_key";
        let data = b"hello world";
        let signature = hmac_sign(key, data).unwrap();
        assert!(hmac_verify(key, data, &signature).unwrap());
        assert!(!hmac_verify(key, b"tampered", &signature).unwrap());
    }

    #[test]
    fn test_derive_key() {
        let password = "my_secure_password";
        let salt = generate_salt(32).unwrap();
        let key1 = derive_key(password, &salt, &KdfConfig::default()).unwrap();
        let key2 = derive_key(password, &salt, &KdfConfig::default()).unwrap();
        assert_eq!(key1, key2);
    }
}
