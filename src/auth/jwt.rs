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
) -> Result<String, AuthError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let exp = now + expires_in_seconds;
    let sub = postcard::to_allocvec(&claim).map_err(|_| AuthError::TokenGenerationFailed)?;
    let sub = hex::encode(sub);
    let claims = Claims { sub, exp, iat: now };
    encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|_| AuthError::TokenGenerationFailed)
}
pub fn verify_token<T: Serialize + DeserializeOwned>(
    token: &str,
    secret: &[u8],
) -> Result<T, AuthError> {
    let claim = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret),
        &Validation::new(Algorithm::HS512),
    ) {
        Ok(claim) => claim,
        Err(e) => match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                return Err(AuthError::ExpiredToken);
            }
            _ => return Err(AuthError::InvalidToken),
        },
    };
    hex::decode(&claim.claims.sub)
        .map_err(|_| AuthError::InvalidToken)
        .and_then(|bytes| postcard::from_bytes(&bytes).map_err(|_| AuthError::InvalidToken))
}
