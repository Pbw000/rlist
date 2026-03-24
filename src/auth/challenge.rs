use std::{
    ops::DerefMut,
    sync::{Arc, atomic::AtomicU64},
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use tokio::sync::RwLock;

use crate::crypto::ChecksumHasher;

#[derive(Debug, Error)]
pub enum ChallengeError {
    #[error("Invalid salt value")]
    InvalidSalt,
    #[error("Failed to validate challenge")]
    ValidationFailed,
    #[error("Challenge expired")]
    Expired,
    #[error("Invalid timestamp")]
    InvalidTimestamp,
}

#[derive(Debug, Clone)]
pub struct ChallengeTask<const DIFFICUITY: usize, const TIMESTAMP_WINDOW_SECS: u64 = 300> {
    pub challenge: Arc<RotatingChallenge>,
}
impl<const DIFFICUITY: usize, const TIMESTAMP_WINDOW_SECS: u64>
    ChallengeTask<DIFFICUITY, TIMESTAMP_WINDOW_SECS>
{
    pub fn new() -> Self {
        Self {
            challenge: Arc::new(RotatingChallenge::new()),
        }
    }
    /// 验证时间戳有效性
    pub fn validate_timestamp(&self, timestamp: u64) -> Result<(), ChallengeError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let diff = now.abs_diff(timestamp);
        if diff > TIMESTAMP_WINDOW_SECS {
            return Err(ChallengeError::InvalidTimestamp);
        }
        Ok(())
    }
    /// 验证 challenge（payload 格式：timestamp + username + password + nonce）
    pub async fn validate<T: Into<String>>(
        &self,
        salt: u64,
        claim: &str,
        payload: T,
    ) -> Result<(), ChallengeError> {
        if claim.len() < DIFFICUITY + 1 {
            eprintln!("Validation failed: claim length too short");
            return Err(ChallengeError::ValidationFailed);
        }
        if !claim[..DIFFICUITY].chars().all(|c| c == '0') {
            eprintln!("Validation failed: claim prefix not all zeros");
            return Err(ChallengeError::ValidationFailed);
        }
        let salt_lock = self.challenge.get_salt(salt).ok_or_else(|| {
            eprintln!("Invalid salt value: {}", salt);
            ChallengeError::InvalidSalt
        })?;
        let mut hasher = ChecksumHasher::new();
        let payload_str = payload.into();
        {
            let salt_hex = salt_lock.read().await;
            hasher.update(salt_hex.as_bytes());
        }
        hasher.update(payload_str.as_bytes());
        let result = hasher.finish_hex();
        if result.as_str() != claim {
            eprintln!("Validation failed: hash result doesn't match claim");
            return Err(ChallengeError::ValidationFailed);
        }

        Ok(())
    }
    pub fn start_rotate(&self, sec: usize) -> tokio::task::JoinHandle<()> {
        let challenge = self.challenge.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(sec as u64));
            loop {
                interval.tick().await;
                challenge.rotate().await;
            }
        })
    }
}
#[derive(Debug)]
pub struct ChallengeSalt {
    pub salt: AtomicU64,
    pub salt_hex: RwLock<String>,
}
#[derive(Debug)]
pub struct RotatingChallenge {
    prev: ChallengeSalt,
    current: ChallengeSalt,
}
impl RotatingChallenge {
    pub fn new() -> Self {
        let salt_dig: u64 = rand::random();
        let salt = AtomicU64::new(salt_dig);
        let salt_hex = RwLock::new(format!("{:x}", salt_dig));
        Self {
            prev: ChallengeSalt {
                salt: AtomicU64::new(salt_dig),
                salt_hex: RwLock::new(format!("{:x}", salt_dig)),
            },
            current: ChallengeSalt { salt, salt_hex },
        }
    }
    pub async fn rotate(&self) {
        let new_salt: u64 = rand::random();
        let prev_salt = self
            .current
            .salt
            .swap(new_salt, std::sync::atomic::Ordering::Relaxed);
        let prev_salt_hex = std::mem::replace(
            self.current.salt_hex.write().await.deref_mut(),
            format!("{:x}", new_salt),
        );
        self.prev
            .salt
            .store(prev_salt, std::sync::atomic::Ordering::Relaxed);
        *self.prev.salt_hex.write().await = prev_salt_hex;
    }
    pub fn get_salt(&self, salt_value: u64) -> Option<&RwLock<String>> {
        if self.current.salt.load(std::sync::atomic::Ordering::Relaxed) == salt_value {
            Some(&self.current.salt_hex)
        } else if self.prev.salt.load(std::sync::atomic::Ordering::Relaxed) == salt_value {
            Some(&self.prev.salt_hex)
        } else {
            None
        }
    }
    pub fn get_current_salt(&self) -> u64 {
        self.current.salt.load(std::sync::atomic::Ordering::Relaxed)
    }
}
