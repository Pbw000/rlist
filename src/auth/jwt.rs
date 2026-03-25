use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token")]
    InvalidToken,
    #[error("Expired token")]
    ExpiredToken,
    #[error("Token generation failed")]
    TokenGenerationFailed,
    #[error("Token verification failed")]
    TokenVerificationFailed,
}

pub fn generate_token<T: Serialize + DeserializeOwned>(
    claim: T,
    secret: &[u8],
    expires_in_seconds: usize,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let exp = now + expires_in_seconds;
    let sub = serde_json::to_string(&claim)?;
    let claims = Claims { sub, exp, iat: now };
    encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(secret),
    )
}
pub fn verify_token<T: Serialize + DeserializeOwned>(
    token: &str,
    secret: &[u8],
) -> Result<T, AuthError> {
    let claim = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|_| AuthError::TokenVerificationFailed)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System Time Exception")
        .as_secs() as usize;
    if claim.claims.exp > now {
        serde_json::from_str(&claim.claims.sub).map_err(|_| AuthError::InvalidToken)
    } else {
        Err(AuthError::ExpiredToken)
    }
}
