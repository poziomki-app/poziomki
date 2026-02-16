#[path = "state_auth.rs"]
mod state_auth;
#[path = "state_types.rs"]
mod state_types;
#[path = "state_uploads.rs"]
mod state_uploads;

use chrono::Utc;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

use super::ErrorSpec;
pub(super) use state_auth::*;
pub(super) use state_types::*;
pub(super) use state_uploads::*;

// --- OTP in-memory state (sole remaining in-memory data) ---

const OTP_STATE_MAX_ENTRIES: usize = 5_000;

#[derive(Default)]
pub(super) struct OtpState {
    pub(super) otp_by_email: HashMap<String, OtpEntry>,
}

impl OtpState {
    /// Evict expired entries and enforce maximum capacity.
    pub(super) fn cleanup(&mut self) {
        let now = Utc::now();
        self.otp_by_email.retain(|_, entry| entry.expires_at > now);

        // If still over capacity, evict oldest entries by last_sent_at
        if self.otp_by_email.len() > OTP_STATE_MAX_ENTRIES {
            let mut entries: Vec<(String, chrono::DateTime<Utc>)> = self
                .otp_by_email
                .iter()
                .map(|(k, v)| (k.clone(), v.last_sent_at))
                .collect();
            entries.sort_by_key(|(_, ts)| *ts);
            let to_remove = self.otp_by_email.len() - OTP_STATE_MAX_ENTRIES;
            for (key, _) in entries.into_iter().take(to_remove) {
                self.otp_by_email.remove(&key);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct OtpEntry {
    pub(super) code: String,
    pub(super) expires_at: chrono::DateTime<Utc>,
    pub(super) attempts: u8,
    pub(super) last_sent_at: chrono::DateTime<Utc>,
}

static OTP_STATE: LazyLock<Mutex<OtpState>> = LazyLock::new(|| Mutex::new(OtpState::default()));

pub(super) fn lock_otp_state() -> MutexGuard<'static, OtpState> {
    OTP_STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn reset_otp_state() {
    let mut state = lock_otp_state();
    *state = OtpState::default();
}

pub(super) fn reset_state() {
    reset_otp_state();
}

// --- Auth validation helpers ---

pub(super) fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

pub(super) fn is_valid_email(email: &str) -> bool {
    let mut split = email.split('@');
    let local = split.next();
    let domain = split.next();
    local.is_some_and(|part| !part.is_empty())
        && domain.is_some_and(|part| part.contains('.'))
        && split.next().is_none()
}

pub(super) fn allowed_email_domain() -> String {
    std::env::var("ALLOWED_EMAIL_DOMAIN").unwrap_or_else(|_| "example.com".to_string())
}

pub(super) fn validate_signup_payload(payload: &SignUpBody) -> std::result::Result<(), ErrorSpec> {
    let email = normalize_email(&payload.email);
    let mut error: Option<ErrorSpec> = None;
    if email.is_empty() {
        error = Some(validation_error_spec("Email is required"));
    } else if !is_valid_email(&email) {
        error = Some(validation_error_spec("Invalid email address"));
    } else if !(1..=100).contains(&payload.name.trim().chars().count()) {
        error = Some(validation_error_spec(
            "Name must be between 1 and 100 characters",
        ));
    } else if !(8..=128).contains(&payload.password.len()) {
        error = Some(validation_error_spec(
            "Password must be between 8 and 128 characters",
        ));
    }
    let domain = allowed_email_domain();
    if error.is_none() && !email.ends_with(&format!("@{domain}")) {
        error = Some(validation_error_spec(&format!(
            "Only @{domain} emails are allowed"
        )));
    }
    error.map_or(Ok(()), Err)
}

fn validation_error_spec(message: &str) -> ErrorSpec {
    ErrorSpec {
        error: message.to_string(),
        code: "VALIDATION_ERROR",
        details: None,
    }
}

pub(super) fn validate_profile_name(name: &str) -> std::result::Result<(), &'static str> {
    if name.trim().is_empty() {
        Err("Name is required")
    } else if name.chars().count() > 100 {
        Err("Name must be at most 100 characters")
    } else if name.contains("http://") || name.contains("https://") || name.contains("www.") {
        Err("Imie nie moze zawierac linkow ani adresow email")
    } else {
        Ok(())
    }
}

pub(super) fn validate_profile_age(age: u8) -> std::result::Result<(), &'static str> {
    if !(15..=67).contains(&age) {
        return Err("Age must be between 15 and 67");
    }
    Ok(())
}
