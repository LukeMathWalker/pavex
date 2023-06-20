use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use pavex::http::HeaderMap;
use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use uuid::Uuid;

const ALGORITHM: Algorithm = Algorithm::EdDSA;

#[derive(Deserialize, Serialize, Debug, Clone)]
/// The claims that are stored in the JWT.
pub struct Claims {
    /// The subject of the token, i.e. the user id.
    sub: Uuid,
    /// The expiry time of the token, in seconds since UNIX_EPOCH.
    exp: u64,
}

impl Claims {
    /// Return the user id encoded in the token.
    pub fn user_id(&self) -> Uuid {
        self.sub
    }
}

/// Create a new token for the given user id.
pub fn encode_token(user_id: Uuid, jwt_key: &EncodingKey) -> Result<Secret<String>, anyhow::Error> {
    let claims = Claims {
        sub: user_id,
        exp: seconds_from_now(3600),
    };
    let header = Header {
        alg: ALGORITHM,
        ..Default::default()
    };
    Ok(Secret::new(encode(&header, &claims, jwt_key)?))
}

fn seconds_from_now(secs: u64) -> u64 {
    let expiry_time =
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap() + Duration::from_secs(secs);
    expiry_time.as_secs()
}

pub fn extract_token(headers: &HeaderMap) -> Option<&str> {
    match headers.get("Authorization") {
        Some(h) => match h.to_str() {
            Ok(hx) => hx.split(' ').nth(1),
            _ => None,
        },
        _ => None,
    }
}

pub fn extract_claims(headers: &HeaderMap, jwt_key: &DecodingKey) -> Option<Claims> {
    let token = extract_token(headers)?;
    match decode_token(token, jwt_key) {
        Ok(claims) => Some(claims),
        Err(e) => {
            info!(
                error.msg = %e, 
                error.error_chain = ?e,
                "Failed to decode token");
            None
        }
    }
}

fn decode_token(token: &str, jwt_key: &DecodingKey) -> Result<Claims, anyhow::Error> {
    let validation = Validation::new(ALGORITHM);
    let decoded = decode::<Claims>(token, jwt_key, &validation)?;
    Ok(decoded.claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use uuid::Uuid;

    #[test]
    fn encode_decode_token() {
        // Arrange
        let sub = Uuid::new_v4();
        let (encoding_key, decoding_key) = generate_keys();

        // Act
        let token = encode_token(sub, &encoding_key).unwrap();
        let decoded = decode_token(&token.expose_secret(), &decoding_key);

        // Assert
        let decoded = decoded.expect("Failed to decode token");
        assert_eq!(decoded.sub, sub);
    }

    fn generate_keys() -> (EncodingKey, DecodingKey) {
        let key_pair = jwt_simple::algorithms::Ed25519KeyPair::generate();
        let encoding_key = EncodingKey::from_ed_pem(key_pair.to_pem().as_bytes()).unwrap();
        let decoding_key =
            DecodingKey::from_ed_pem(key_pair.public_key().to_pem().as_bytes()).unwrap();
        (encoding_key, decoding_key)
    }
}
