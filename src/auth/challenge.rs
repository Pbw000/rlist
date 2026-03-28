use ring::digest::Context;
use std::{
    ops::DerefMut,
    sync::{Arc, atomic::AtomicU64},
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use tokio::sync::RwLock;

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
    #[error("Invalid format")]
    InvalidFormat,
    #[error("Challenge failed")]
    ChallengeFailed,
}
pub trait IntoHashContext {
    fn hash_and_to_context(&self) -> Context;
}
#[derive(Debug, Clone)]
pub struct ChallengeTask<const TIMESTAMP_WINDOW_SECS: u64 = 300> {
    pub challenge: Arc<RotatingChallenge>,
}
impl<const TIMESTAMP_WINDOW_SECS: u64> ChallengeTask<TIMESTAMP_WINDOW_SECS> {
    pub fn new() -> Self {
        Self {
            challenge: Arc::new(RotatingChallenge::new()),
        }
    }
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
    pub async fn validate<T: IntoHashContext, const DIFFICUITY: usize>(
        &self,
        salt: u64,
        claim: &str,
        payload: &T,
    ) -> Result<(), ChallengeError> {
        if claim.len() < DIFFICUITY + 1 {
            tracing::debug!("Challenge validation failed: claim length too short");
            return Err(ChallengeError::InvalidFormat);
        }
        if !claim[..DIFFICUITY].chars().all(|c| c == '0') {
            tracing::debug!("Challenge validation failed: insufficient leading zeros");
            return Err(ChallengeError::ChallengeFailed);
        }
        let salt_lock = self.challenge.get_salt(salt).ok_or_else(|| {
            tracing::debug!("Challenge validation failed: invalid salt value");
            ChallengeError::InvalidSalt
        })?;
        let mut hasher = payload.hash_and_to_context();
        {
            let salt_hex = salt_lock.read().await;
            hasher.update(salt_hex.as_bytes());
        }
        let result = hasher.finish();
        let result = hex::encode(result);
        if result.as_str() != claim {
            tracing::debug!("Challenge validation failed: hash mismatch");
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
