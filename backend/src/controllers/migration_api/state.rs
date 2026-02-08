#[path = "state_events.rs"]
mod state_events;
#[path = "state_profile.rs"]
mod state_profile;
#[path = "state_types.rs"]
mod state_types;
#[path = "state_uploads.rs"]
mod state_uploads;

use axum::http::HeaderMap;
use chrono::{DateTime, Duration, Utc};
use loco_rs::prelude::*;
use std::sync::{LazyLock, Mutex, MutexGuard};
use uuid::Uuid;

use super::{error_response, ErrorSpec};
pub(super) use state_events::*;
pub(super) use state_profile::*;
pub(super) use state_types::*;
pub(super) use state_uploads::*;

const SESSION_DURATION_SECS: i64 = 60 * 60 * 24 * 7;
const SESSION_UPDATE_AGE_SECS: i64 = 60 * 60 * 24;

fn seed_tags() -> std::collections::HashMap<String, TagRecord> {
    let entries: &[(&str, TagScope, &str, &str)] = &[
        // Zainteresowania
        ("Muzyka", TagScope::Interest, "hobby", "1"),
        ("Sport", TagScope::Interest, "hobby", "2"),
        ("Podróże", TagScope::Interest, "hobby", "3"),
        ("Fotografia", TagScope::Interest, "hobby", "4"),
        ("Gry", TagScope::Interest, "hobby", "5"),
        ("Gotowanie", TagScope::Interest, "hobby", "6"),
        ("Czytanie", TagScope::Interest, "hobby", "7"),
        ("Sztuka", TagScope::Interest, "hobby", "8"),
        ("Film", TagScope::Interest, "hobby", "9"),
        ("Taniec", TagScope::Interest, "hobby", "10"),
        ("Fitness", TagScope::Interest, "styl życia", "11"),
        ("Joga", TagScope::Interest, "styl życia", "12"),
        ("Góry", TagScope::Interest, "styl życia", "13"),
        ("Rower", TagScope::Interest, "styl życia", "14"),
        ("Bieganie", TagScope::Interest, "styl życia", "15"),
        ("Programowanie", TagScope::Interest, "tech", "16"),
        ("AI i ML", TagScope::Interest, "tech", "17"),
        ("Startupy", TagScope::Interest, "tech", "18"),
        ("Design", TagScope::Interest, "tech", "19"),
        ("Nauka", TagScope::Interest, "akademickie", "20"),
        ("Filozofia", TagScope::Interest, "akademickie", "21"),
        ("Języki obce", TagScope::Interest, "akademickie", "22"),
        ("Wolontariat", TagScope::Interest, "społeczne", "23"),
        ("Gry planszowe", TagScope::Interest, "społeczne", "24"),
        // Aktywności
        ("Grupa naukowa", TagScope::Activity, "akademickie", "1"),
        ("Kawa i rozmowa", TagScope::Activity, "społeczne", "2"),
        ("Partner treningowy", TagScope::Activity, "fitness", "3"),
        ("Wspólny projekt", TagScope::Activity, "tech", "4"),
        ("Wymiana językowa", TagScope::Activity, "akademickie", "5"),
    ];

    entries
        .iter()
        .map(|(name, scope, category, order)| {
            let id = Uuid::new_v4().to_string();
            (
                id.clone(),
                TagRecord {
                    id,
                    name: (*name).to_string(),
                    scope: *scope,
                    category: Some((*category).to_string()),
                    emoji: None,
                    onboarding_order: Some((*order).to_string()),
                },
            )
        })
        .collect()
}

impl MigrationState {
    fn new() -> Self {
        Self {
            users: std::collections::HashMap::new(),
            users_by_email: std::collections::HashMap::new(),
            sessions_by_token: std::collections::HashMap::new(),
            profiles: std::collections::HashMap::new(),
            profiles_by_user: std::collections::HashMap::new(),
            tags: seed_tags(),
            degrees: vec![
                DegreeRecord {
                    id: Uuid::new_v4().to_string(),
                    name: "Computer Science".to_string(),
                },
                DegreeRecord {
                    id: Uuid::new_v4().to_string(),
                    name: "Data Science".to_string(),
                },
                DegreeRecord {
                    id: Uuid::new_v4().to_string(),
                    name: "Psychology".to_string(),
                },
            ],
            events: std::collections::HashMap::new(),
            event_attendees: std::collections::HashMap::new(),
            uploads: std::collections::HashMap::new(),
            otp_by_email: std::collections::HashMap::new(),
            user_settings: std::collections::HashMap::new(),
        }
    }
}

static STATE: LazyLock<Mutex<MigrationState>> = LazyLock::new(|| Mutex::new(MigrationState::new()));

pub(super) fn lock_state() -> MutexGuard<'static, MigrationState> {
    STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(super) fn reset_state() {
    let mut state = lock_state();
    *state = MigrationState::new();
}

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
        error = Some(validation_error("Email is required"));
    } else if !is_valid_email(&email) {
        error = Some(validation_error("Invalid email address"));
    } else if !(1..=100).contains(&payload.name.trim().chars().count()) {
        error = Some(validation_error(
            "Name must be between 1 and 100 characters",
        ));
    } else if !(8..=128).contains(&payload.password.len()) {
        error = Some(validation_error(
            "Password must be between 8 and 128 characters",
        ));
    }
    let domain = allowed_email_domain();
    if error.is_none() && !email.ends_with(&format!("@{domain}")) {
        error = Some(validation_error(&format!(
            "Only @{domain} emails are allowed"
        )));
    }
    error.map_or(Ok(()), Err)
}

fn validation_error(message: &str) -> ErrorSpec {
    ErrorSpec {
        error: message.to_string(),
        code: "VALIDATION_ERROR",
        details: None,
    }
}

pub(super) fn extract_bearer_token(headers: &HeaderMap) -> Option<String> {
    let header = headers.get("authorization")?.to_str().ok()?;
    let token = header.strip_prefix("Bearer ")?;
    Some(token.to_string())
}

pub(super) fn session_to_view(session: &SessionRecord) -> SessionView {
    SessionView {
        id: session.id.clone(),
        user_id: session.user_id.clone(),
        token: session.token.clone(),
        expires_at: session.expires_at.to_rfc3339(),
        created_at: session.created_at.to_rfc3339(),
        updated_at: session.updated_at.to_rfc3339(),
        ip_address: session.ip_address.clone(),
        user_agent: session.user_agent.clone(),
    }
}

pub(super) fn user_to_view(user: &UserRecord) -> UserView {
    UserView {
        id: user.id.clone(),
        email: user.email.clone(),
        name: user.name.clone(),
        email_verified: user.email_verified,
    }
}

pub(super) fn resolve_session(
    state: &mut MigrationState,
    token: &str,
    now: DateTime<Utc>,
) -> Option<SessionRecord> {
    let maybe_session = state.sessions_by_token.get(token)?.clone();
    if maybe_session.expires_at <= now {
        state.sessions_by_token.remove(token);
        return None;
    }

    let mut session = maybe_session;
    let elapsed = now - session.updated_at;
    if elapsed >= Duration::seconds(SESSION_UPDATE_AGE_SECS) {
        session.updated_at = now;
        session.expires_at = now + Duration::seconds(SESSION_DURATION_SECS);
        state
            .sessions_by_token
            .insert(token.to_string(), session.clone());
    }
    Some(session)
}

pub(super) fn make_session(headers: &HeaderMap, user_id: &str) -> SessionRecord {
    let now = Utc::now();
    let token = Uuid::new_v4().to_string();
    SessionRecord {
        id: Uuid::new_v4().to_string(),
        user_id: user_id.to_string(),
        token,
        created_at: now,
        updated_at: now,
        expires_at: now + Duration::seconds(SESSION_DURATION_SECS),
        ip_address: headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned),
        user_agent: headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .map(ToOwned::to_owned),
    }
}

pub(super) fn require_auth(
    headers: &HeaderMap,
    state: &mut MigrationState,
) -> std::result::Result<(SessionRecord, UserRecord), Box<Response>> {
    extract_bearer_token(headers)
        .and_then(|token| resolve_session(state, &token, Utc::now()))
        .and_then(|session| {
            state
                .users
                .get(&session.user_id)
                .cloned()
                .map(|user| (session, user))
        })
        .ok_or_else(|| Box::new(unauthorized_response(headers)))
}

pub(super) fn require_profile(
    headers: &HeaderMap,
    state: &MigrationState,
    user_id: &str,
) -> std::result::Result<ProfileRecord, Box<Response>> {
    state
        .profiles_by_user
        .get(user_id)
        .and_then(|profile_id| state.profiles.get(profile_id))
        .cloned()
        .ok_or_else(|| {
            Box::new(error_response(
                axum::http::StatusCode::NOT_FOUND,
                headers,
                ErrorSpec {
                    error: "Profile not found. Create a profile first.".to_string(),
                    code: "NOT_FOUND",
                    details: None,
                },
            ))
        })
}

fn unauthorized_response(headers: &HeaderMap) -> Response {
    error_response(
        axum::http::StatusCode::UNAUTHORIZED,
        headers,
        ErrorSpec {
            error: "Authentication required".to_string(),
            code: "UNAUTHORIZED",
            details: None,
        },
    )
}
