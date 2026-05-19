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
use axum::http::{HeaderName, HeaderValue};
use axum_test::TestServer;
use diesel::deserialize::QueryableByName;
use diesel::sql_types::{Bool, Nullable, Text, Timestamptz, VarChar};
use serial_test::serial;
use std::future::Future;

fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
    (HeaderName::from_static("authorization"), value)
}

async fn fetch_otp(email: &str) -> String {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use poziomki_backend::db::models::otp_codes::OtpCode;
    use poziomki_backend::db::schema::otp_codes;
    let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
    otp_codes::table
        .filter(otp_codes::email.eq(email))
        .first::<OtpCode>(&mut conn)
        .await
        .expect("OTP row exists")
        .code
}

/// Reads `profiles.is_pre_launch` for the given email via an owner-role
/// connection — the API pool (`poziomki_api`) can't see other users' rows
/// under RLS, so this can't go through the normal pool.
fn read_is_pre_launch(email: &str) -> bool {
    use diesel::prelude::*;
    #[derive(QueryableByName)]
    struct Row {
        #[diesel(sql_type = Bool)]
        is_pre_launch: bool,
    }
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL set");
    let mut conn = diesel::pg::PgConnection::establish(&url).expect("owner conn");
    diesel::sql_query(
        "SELECT p.is_pre_launch
         FROM public.profiles p JOIN public.users u ON u.id = p.user_id
         WHERE u.email = $1",
    )
    .bind::<Text, _>(email)
    .get_result::<Row>(&mut conn)
    .expect("profile row")
    .is_pre_launch
}

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

/// Full end-to-end flow that the landing page drives the user through:
///
///   sign-up (source=`landing_early_access`)
///   → verify-otp
///   → POST /profiles with name + program + bio + tagIds
///   → assert `profile.is_pre_launch` = TRUE  (hidden from mobile)
///   → POST /profiles/me/finalize-pre-launch
///   → assert `profile.is_pre_launch` = FALSE (now visible)
#[tokio::test]
#[serial]
async fn early_access_full_flow_to_finalize() {
    run(|server| async move {
        let email = "e2e@example.com";

        // 1. landing-page signup
        let signup = server
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": email,
                "name": "E2E",
                "password": "secret123",
                "platformPref": "android",
                "source": "landing_early_access",
            }))
            .await;
        assert_eq!(signup.status_code(), 200);

        // 2. verify OTP → bearer token
        let otp = fetch_otp(email).await;
        let verify = server
            .post("/api/v1/auth/verify-otp")
            .json(&serde_json::json!({ "email": email, "otp": otp }))
            .await;
        assert_eq!(verify.status_code(), 200);
        let body: serde_json::Value = verify.json();
        let token = body["data"]["token"].as_str().expect("token").to_owned();
        let (k, v) = auth_header(&token);

        // 3. create the profile with the full payload — what the
        //    landing's "potwierdź" click sends in one shot.
        let create = server
            .post("/api/v1/profiles")
            .add_header(k.clone(), v.clone())
            .json(&serde_json::json!({
                "name": "Ada",
                "program": "informatyka",
                "bio": "hej",
            }))
            .await;
        assert!(
            matches!(create.status_code().as_u16(), 200 | 201),
            "profile create should succeed, got {}",
            create.status_code()
        );

        // 4. profile must be flagged is_pre_launch = TRUE (invisible
        //    to mobile reads).
        assert!(
            read_is_pre_launch(email),
            "landing-created profile should start hidden (is_pre_launch=true)"
        );

        // 5. mobile finishes its own onboarding → finalize endpoint
        let finalize = server
            .post("/api/v1/profiles/me/finalize-pre-launch")
            .add_header(k.clone(), v.clone())
            .await;
        assert_eq!(finalize.status_code(), 200);
        let body: serde_json::Value = finalize.json();
        assert_eq!(body["data"]["finalized"], serde_json::json!(true));

        // 6. flag flipped → profile now visible to the app
        assert!(
            !read_is_pre_launch(email),
            "after finalize, is_pre_launch should be FALSE"
        );

        // 7. second finalize is a no-op (idempotent)
        let again = server
            .post("/api/v1/profiles/me/finalize-pre-launch")
            .add_header(k, v)
            .await;
        assert_eq!(again.status_code(), 200);
        let body: serde_json::Value = again.json();
        assert_eq!(body["data"]["finalized"], serde_json::json!(false));
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
