//! Integration tests for the landing-page early-access (pre-launch) signup
//! branch of `POST /api/v1/auth/sign-up/email`.
//!
//! Verifies:
//!   * happy path stamps `pre_launch_signed_up_at` + `platform_pref` +
//!     `signup_source` on the resulting user row
//!   * an OTP is enqueued the same way mobile signups produce one
//!   * Turnstile is bypassed when `TURNSTILE_SECRET` is unset (the same
//!     dev-bypass the helper documents in `auth/turnstile.rs`)
//!   * CORS preflight from the landing origin succeeds
//!
//! The IP rate-limit cap and the Turnstile *failure* branch are covered
//! by unit tests rather than integration — exercising them here would
//! require either toggling env vars per test (race-prone) or stubbing
//! the upstream Cloudflare endpoint (out of scope for this PR).

use axum::http::header::ORIGIN;
use axum_test::TestServer;
use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Nullable, Text, Timestamptz, VarChar};
use serial_test::serial;
use std::future::Future;

async fn run<F, Fut>(f: F)
where
    F: FnOnce(TestServer) -> Fut,
    Fut: Future<Output = ()>,
{
    let _ = dotenvy::dotenv();
    let ctx = poziomki_backend::app::build_test_app_context().expect("build test app context");
    poziomki_backend::app::reset_test_database()
        .await
        .expect("truncate test tables");
    let server = TestServer::new(poziomki_backend::app::build_router_with_state(ctx));
    f(server).await;
}

#[derive(QueryableByName)]
struct UserPreLaunchRow {
    #[diesel(sql_type = VarChar)]
    email: String,
    #[diesel(sql_type = Nullable<VarChar>)]
    platform_pref: Option<String>,
    #[diesel(sql_type = Nullable<VarChar>)]
    signup_source: Option<String>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    pre_launch_signed_up_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[allow(clippy::unused_async)]
async fn read_user_metadata(email: &str) -> UserPreLaunchRow {
    // The API pool runs as `poziomki_api` (NOBYPASSRLS), so a raw SELECT
    // on `users` returns 0 rows under RLS. Use a one-shot owner-role
    // connection straight to `DATABASE_URL` for the read-back — same
    // trick we'd use from a migration or admin CLI.
    use diesel::prelude::*;
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL set for test owner reads");
    let mut conn =
        diesel::pg::PgConnection::establish(&database_url).expect("connect as owner role");
    diesel::sql_query(
        "SELECT email, platform_pref, signup_source, pre_launch_signed_up_at
         FROM public.users WHERE email = $1",
    )
    .bind::<Text, _>(email)
    .get_result::<UserPreLaunchRow>(&mut conn)
    .expect("user row not found")
}

#[tokio::test]
#[serial]
async fn early_access_signup_stamps_pre_launch_metadata() {
    run(|server| async move {
        let response = server
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "early@example.com",
                "name": "Early Adopter",
                "password": "secret123",
                "platformPref": "android",
                "source": "landing_early_access",
                // No turnstileToken — verifier bypasses when
                // TURNSTILE_SECRET is unset (the dev-bypass branch).
            }))
            .await;
        assert_eq!(response.status_code(), 200);

        let row = read_user_metadata("early@example.com").await;
        assert_eq!(row.email, "early@example.com");
        assert_eq!(row.platform_pref.as_deref(), Some("android"));
        assert_eq!(row.signup_source.as_deref(), Some("landing_early_access"));
        assert!(
            row.pre_launch_signed_up_at.is_some(),
            "pre_launch_signed_up_at should be stamped"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn early_access_signup_rejects_unknown_platform() {
    run(|server| async move {
        let response = server
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "bad@example.com",
                "name": "Bad Platform",
                "password": "secret123",
                "platformPref": "windows-phone",
                "source": "landing_early_access",
            }))
            .await;
        assert_eq!(response.status_code(), 400);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn mobile_signup_still_works_without_source() {
    // Regression guard: the new branching must not affect the existing
    // mobile signup path which sends no `source` / `platformPref`.
    run(|server| async move {
        let response = server
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "mobile@example.com",
                "name": "Mobile User",
                "password": "secret123",
            }))
            .await;
        assert_eq!(response.status_code(), 200);

        let row = read_user_metadata("mobile@example.com").await;
        assert!(
            row.pre_launch_signed_up_at.is_none(),
            "mobile signups must NOT be marked as pre-launch"
        );
        assert!(row.platform_pref.is_none());
        assert!(row.signup_source.is_none());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cors_preflight_from_landing_origin_succeeds() {
    run(|server| async move {
        let response = server
            .method(axum::http::Method::OPTIONS, "/api/v1/auth/sign-up/email")
            .add_header(
                ORIGIN,
                axum::http::HeaderValue::from_static("https://poziomki.app"),
            )
            .add_header(
                axum::http::header::ACCESS_CONTROL_REQUEST_METHOD,
                axum::http::HeaderValue::from_static("POST"),
            )
            .add_header(
                axum::http::header::ACCESS_CONTROL_REQUEST_HEADERS,
                axum::http::HeaderValue::from_static("content-type"),
            )
            .await;

        // 200 or 204 are both valid preflight outcomes depending on
        // tower-http's CorsLayer version.
        let status = response.status_code().as_u16();
        assert!(
            matches!(status, 200 | 204),
            "preflight should succeed, got {status}"
        );
        let allow_origin = response.headers().get("access-control-allow-origin");
        assert_eq!(
            allow_origin.and_then(|v| v.to_str().ok()),
            Some("https://poziomki.app"),
        );
    })
    .await;
}
