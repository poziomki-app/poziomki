use std::sync::LazyLock;

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

/// A valid Argon2id hash under the same `Params::default()` the
/// login path uses, pre-computed on first access.
///
/// Exposed via `run_dummy_password_verify` so the login handler can
/// run an Argon2 verify on the "user not found" branch. Without this,
/// login latency leaks whether an email is registered.
static DUMMY_PASSWORD_HASH: LazyLock<String> = LazyLock::new(|| {
    // Fall back to a syntactically valid hash if the (static-string)
    // hash computation somehow fails — `verify_password` will return
    // false but still incur the Argon2 work, which is what we want.
    hash_password("dummy-for-constant-time-login")
        .unwrap_or_else(|_| String::from("$argon2id$v=19$m=19456,t=2,p=1$AAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"))
});

/// Run a password verify against the cached dummy hash.
///
/// Used by the login path when the email lookup returned `None` so
/// the request takes the same wall-clock time as a valid-email
/// wrong-password attempt. The return value is discarded by the
/// caller — callers treat the "user not found" branch as an
/// authentication failure regardless of the outcome.
#[must_use]
pub fn run_dummy_password_verify(pass: &str) -> bool {
    verify_password(pass, DUMMY_PASSWORD_HASH.as_str())
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

#[cfg(test)]
mod tests {
    use super::{hash_password, run_dummy_password_verify, verify_password, DUMMY_PASSWORD_HASH};
    use std::time::Instant;

    #[test]
    fn dummy_password_verify_rejects_arbitrary_inputs() {
        // Any password other than the sentinel must fail. The caller
        // discards the bool regardless, but this pins the obvious
        // failure mode.
        assert!(!run_dummy_password_verify("anything"));
        assert!(!run_dummy_password_verify(""));
        assert!(!run_dummy_password_verify("password123"));
    }

    #[test]
    fn dummy_hash_parses_and_verifies_under_same_params() {
        // If DUMMY_PASSWORD_HASH isn't a valid Argon2 hash, verify_password
        // returns early via `PasswordHash::new` failure, which would make
        // the dummy path faster than the real path — defeating the whole
        // purpose. Lock that invariant here.
        let verify_accepts_it = verify_password(
            "dummy-for-constant-time-login",
            DUMMY_PASSWORD_HASH.as_str(),
        );
        assert!(
            verify_accepts_it,
            "DUMMY_PASSWORD_HASH must parse + verify the static sentinel under Params::default()"
        );
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn dummy_verify_latency_matches_real_verify() {
        // Timing sanity: the dummy verify should cost roughly the same
        // as a real wrong-password verify. If the dummy hash ever gets
        // swapped for cheaper params, this test catches the regression
        // by flagging an order-of-magnitude divergence. The threshold
        // is deliberately loose (5x) so CI noise doesn't flake it.
        let real_hash =
            hash_password("real-secret").unwrap_or_else(|_| unreachable!("static-string hash"));

        // Warm any LazyLock work.
        let _ = run_dummy_password_verify("warm");
        let _ = verify_password("warm", &real_hash);

        let n = 5;
        let t_real = {
            let start = Instant::now();
            for _ in 0..n {
                let _ = verify_password("wrong-pw", &real_hash);
            }
            start.elapsed()
        };
        let t_dummy = {
            let start = Instant::now();
            for _ in 0..n {
                let _ = run_dummy_password_verify("wrong-pw");
            }
            start.elapsed()
        };

        let real_ns = t_real.as_nanos().max(1) as f64;
        let dummy_ns = t_dummy.as_nanos().max(1) as f64;
        let ratio = if real_ns >= dummy_ns {
            real_ns / dummy_ns
        } else {
            dummy_ns / real_ns
        };
        assert!(
            ratio < 5.0,
            "dummy verify and real verify latencies must be within 5x (t_real={t_real:?}, t_dummy={t_dummy:?}, ratio={ratio})"
        );
    }
}
