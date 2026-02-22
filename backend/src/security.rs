use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
};
use jsonwebtoken::{encode, get_current_timestamp, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::{Map, Value};

const JWT_ALGORITHM: Algorithm = Algorithm::HS512;

#[derive(Debug, Serialize)]
struct UserClaims {
    pid: String,
    exp: u64,
    #[serde(flatten)]
    claims: Map<String, Value>,
}

pub fn hash_password(pass: &str) -> Result<String, argon2::password_hash::Error> {
    let arg2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        Params::default(),
    );
    let salt = SaltString::generate(&mut OsRng);

    Ok(arg2.hash_password(pass.as_bytes(), &salt)?.to_string())
}

#[must_use]
pub fn verify_password(pass: &str, hashed_password: &str) -> bool {
    let arg2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        Version::V0x13,
        Params::default(),
    );
    let Ok(hash) = PasswordHash::new(hashed_password) else {
        return false;
    };

    arg2.verify_password(pass.as_bytes(), &hash).is_ok()
}

pub fn generate_user_jwt(
    secret: &str,
    expiration: u64,
    pid: String,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = UserClaims {
        pid,
        exp: get_current_timestamp().saturating_add(expiration),
        claims: Map::new(),
    };

    encode(
        &Header::new(JWT_ALGORITHM),
        &claims,
        &EncodingKey::from_base64_secret(secret)?,
    )
}
