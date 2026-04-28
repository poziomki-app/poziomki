use axum::http::{HeaderName, HeaderValue};
use axum_test::multipart::{MultipartForm, Part};
use axum_test::TestServer;
use base64::Engine as _;

use serial_test::serial;
use std::future::Future;
use std::io::Read;

fn parse_export_zip(bytes: &[u8]) -> serde_json::Value {
    let cursor = std::io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).unwrap();
    let mut file = archive.by_name("data.json").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    serde_json::from_str(&contents).unwrap()
}

fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
    (HeaderName::from_static("authorization"), value)
}

fn sign_up_json(email: &str, password: &str) -> serde_json::Value {
    serde_json::json!({
        "email": email,
        "name": "Test User",
        "password": password,
    })
}

fn tiny_png_bytes() -> Vec<u8> {
    // Minimal 1x1 grayscale PNG with a correct IDAT CRC. The previous
    // fixture had a mismatched CRC — the image crate's dimension
    // parser tolerated it but the full decoder (used by the
    // EXIF-strip step) rejects the chunk. Replacement generated via
    // python: png.chunk('IHDR', IHDR bytes) + valid-crc IDAT + IEND.
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAAAAAA6fptVAAAACklEQVR4nGP4DwABAQEAsTj2FAAAAABJRU5ErkJggg==")
        .expect("valid embedded png")
}

async fn request<F, Fut>(f: F)
where
    F: FnOnce(TestServer, poziomki_backend::app::AppContext) -> Fut,
    Fut: Future<Output = ()>,
{
    let _ = dotenvy::dotenv();
    let ctx = poziomki_backend::app::build_test_app_context().expect("build test app context");
    poziomki_backend::app::reset_test_database()
        .await
        .expect("truncate test tables");
    let server = TestServer::new(poziomki_backend::app::build_router_with_state(ctx.clone()));
    f(server, ctx).await;
}

async fn create_user_with_profile(
    request: &TestServer,
    email: &str,
    name: &str,
) -> (HeaderName, HeaderValue) {
    let token = sign_up_and_verify(request, email, "secret123", name).await;

    let (auth_key, auth_value) = auth_header(&token);
    let profile = request
        .post("/api/v1/profiles")
        .add_header(auth_key.clone(), auth_value.clone())
        .json(&serde_json::json!({ "name": name, "age": 26 }))
        .await;
    assert_eq!(profile.status_code(), 201);

    (auth_key, auth_value)
}

async fn upload_png(
    request: &TestServer,
    auth_key: &HeaderName,
    auth_value: &HeaderValue,
) -> serde_json::Value {
    let form = MultipartForm::new()
        .add_text("context", "profile_gallery")
        .add_part(
            "file",
            Part::bytes(tiny_png_bytes())
                .file_name("tiny.png")
                .mime_type("image/png"),
        );

    let upload = request
        .post("/api/v1/uploads")
        .add_header(auth_key.clone(), auth_value.clone())
        .multipart(form)
        .await;
    assert_eq!(upload.status_code(), 200);
    upload.json()
}

/// Query the OTP code from the database for a given email.
async fn get_otp_code(email: &str) -> String {
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use poziomki_backend::db::models::otp_codes::OtpCode;
    use poziomki_backend::db::schema::otp_codes;

    let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
    otp_codes::table
        .filter(otp_codes::email.eq(email))
        .first::<OtpCode>(&mut conn)
        .await
        .expect("No OTP found for email")
        .code
}

async fn sign_up_and_verify(
    request: &TestServer,
    email: &str,
    password: &str,
    name: &str,
) -> String {
    let sign_up = request
        .post("/api/v1/auth/sign-up/email")
        .json(&serde_json::json!({
            "email": email,
            "name": name,
            "password": password,
        }))
        .await;
    assert_eq!(sign_up.status_code(), 200);
    let sign_up_payload: serde_json::Value = sign_up.json();
    assert!(sign_up_payload["data"]["token"].is_null());

    let otp_code = get_otp_code(email).await;
    let verify = request
        .post("/api/v1/auth/verify-otp")
        .json(&serde_json::json!({
            "email": email,
            "otp": otp_code,
        }))
        .await;
    assert_eq!(verify.status_code(), 200);
    let verify_payload: serde_json::Value = verify.json();
    verify_payload["data"]["token"]
        .as_str()
        .map(ToOwned::to_owned)
        .expect("verify-otp should return token")
}

#[tokio::test]
#[serial]
async fn health_endpoint_matches_contract() {
    request(|request, _ctx| async move {
        let response = request.get("/health").await;
        assert_eq!(response.status_code(), 200);
        response.assert_json(&serde_json::json!({ "status": "ok" }));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn root_endpoint_matches_contract() {
    request(|request, _ctx| async move {
        let response = request.get("/").await;
        assert_eq!(response.status_code(), 200);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["docs"], "/api/docs");
        assert_eq!(payload["message"], "poziomki API v1");
        assert_eq!(payload["version"], "1.0.0");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn profiles_me_requires_auth_after_phase_2() {
    request(|request, _ctx| async move {
        let response = request.get("/api/v1/profiles/me").await;
        assert_eq!(response.status_code(), 401);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["code"], "UNAUTHORIZED");
        assert!(payload["requestId"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn auth_get_session_returns_unwrapped_shape() {
    request(|request, _ctx| async move {
        let response = request.get("/api/v1/auth/get-session").await;
        assert_eq!(response.status_code(), 200);
        response.assert_json(&serde_json::json!({
            "session": null,
            "user": null
        }));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn events_flow_matches_phase_3_contract() {
    request(|request, _ctx| async move {
        let owner_token =
            sign_up_and_verify(&request, "owner@example.com", "secret123", "Owner").await;

        let (owner_auth_key, owner_auth_value) = auth_header(&owner_token);
        let profile_response = request
            .post("/api/v1/profiles")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "name": "Owner",
                "age": 21,
            }))
            .await;
        assert_eq!(profile_response.status_code(), 201);

        // Create event without geo
        let create_response = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Rust migration event",
                "startsAt": "2030-01-01T12:00:00Z",
                "endsAt": "2030-01-01T13:00:00Z",
                "maxAttendees": 2,
                "tags": ["music"]
            }))
            .await;
        assert_eq!(create_response.status_code(), 201);

        let created_payload: serde_json::Value = create_response.json();
        let event_id = created_payload["data"]["id"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        assert!(!event_id.is_empty());
        assert_eq!(created_payload["data"]["attendeesCount"], 1);
        assert_eq!(created_payload["data"]["maxAttendees"], 2);
        assert_eq!(created_payload["data"]["isAttending"], true);
        assert_eq!(created_payload["data"]["isSaved"], false);
        assert_eq!(created_payload["data"]["latitude"], serde_json::Value::Null);
        assert_eq!(
            created_payload["data"]["longitude"],
            serde_json::Value::Null
        );
        assert_eq!(created_payload["data"]["tags"][0]["name"], "music");

        let invalid_tag_create = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Broken tag ids event",
                "startsAt": "2030-01-02T12:00:00Z",
                "tagIds": ["11111111-1111-1111-1111-111111111111"]
            }))
            .await;
        assert_eq!(invalid_tag_create.status_code(), 400);

        // Absurdly high max_attendees is rejected — 10 000 is the upper bound.
        let over_cap_create = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Stadium event",
                "startsAt": "2030-01-02T14:00:00Z",
                "maxAttendees": 10_001,
            }))
            .await;
        assert_eq!(over_cap_create.status_code(), 400);

        let owner_list_after_invalid = request
            .get("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .await;
        assert_eq!(owner_list_after_invalid.status_code(), 200);
        let owner_list_payload: serde_json::Value = owner_list_after_invalid.json();
        assert!(!owner_list_payload["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .any(|row| row["title"] == "Broken tag ids event"));

        let second_music_event = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Second music event",
                "startsAt": "2030-01-03T12:00:00Z",
                "tags": ["music"]
            }))
            .await;
        assert_eq!(second_music_event.status_code(), 201);
        let second_music_payload: serde_json::Value = second_music_event.json();
        let second_music_event_id = second_music_payload["data"]["id"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        assert!(!second_music_event_id.is_empty());

        // Create event with geo coordinates
        let geo_create = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Geo event in Warsaw",
                "startsAt": "2030-06-01T10:00:00Z",
                "latitude": 52.2297,
                "longitude": 21.0122
            }))
            .await;
        assert_eq!(geo_create.status_code(), 201);
        let geo_payload: serde_json::Value = geo_create.json();
        let geo_event_id = geo_payload["data"]["id"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        assert!(!geo_event_id.is_empty());
        assert_eq!(geo_payload["data"]["latitude"], 52.2297);
        assert_eq!(geo_payload["data"]["longitude"], 21.0122);

        // Verify geo event appears in list with coordinates
        let geo_detail = request
            .get(&format!("/api/v1/events/{geo_event_id}"))
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .await;
        assert_eq!(geo_detail.status_code(), 200);
        let detail_payload: serde_json::Value = geo_detail.json();
        assert_eq!(detail_payload["data"]["latitude"], 52.2297);
        assert_eq!(detail_payload["data"]["longitude"], 21.0122);

        // Update event to add geo coordinates
        let update_geo = request
            .patch(&format!("/api/v1/events/{event_id}"))
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "latitude": 50.0647,
                "longitude": 19.9450,
                "maxAttendees": 3
            }))
            .await;
        assert_eq!(update_geo.status_code(), 200);
        let updated_payload: serde_json::Value = update_geo.json();
        assert_eq!(updated_payload["data"]["latitude"], 50.0647);
        assert_eq!(updated_payload["data"]["longitude"], 19.945);
        assert_eq!(updated_payload["data"]["maxAttendees"], 3);

        // Update event to clear geo coordinates
        let clear_geo = request
            .patch(&format!("/api/v1/events/{event_id}"))
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "latitude": null,
                "longitude": null
            }))
            .await;
        assert_eq!(clear_geo.status_code(), 200);
        let cleared_payload: serde_json::Value = clear_geo.json();
        assert_eq!(cleared_payload["data"]["latitude"], serde_json::Value::Null);
        assert_eq!(
            cleared_payload["data"]["longitude"],
            serde_json::Value::Null
        );

        let attendee_token =
            sign_up_and_verify(&request, "attendee@example.com", "secret123", "Attendee").await;

        let (attendee_auth_key, attendee_auth_value) = auth_header(&attendee_token);
        let attendee_profile_response = request
            .post("/api/v1/profiles")
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .json(&serde_json::json!({
                "name": "Attendee",
                "age": 22,
            }))
            .await;
        assert_eq!(attendee_profile_response.status_code(), 201);

        let attend_response = request
            .post(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .json(&serde_json::json!({ "status": "going" }))
            .await;
        assert_eq!(attend_response.status_code(), 200);
        let attend_payload: serde_json::Value = attend_response.json();
        assert_eq!(attend_payload["data"]["attendeesCount"], 2);
        assert_eq!(attend_payload["data"]["isAttending"], true);

        let save_response = request
            .post(&format!("/api/v1/events/{event_id}/save"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(save_response.status_code(), 200);
        let save_payload: serde_json::Value = save_response.json();
        assert_eq!(save_payload["data"]["isSaved"], true);

        let export_response = request
            .get("/api/v1/auth/export")
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(export_response.status_code(), 200);
        let export_payload = parse_export_zip(export_response.as_bytes());
        let interactions = export_payload["eventInteractions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(interactions.iter().any(|row| row["kind"] == "joined"));
        assert!(interactions.iter().any(|row| row["kind"] == "saved"));

        let unsave_response = request
            .delete(&format!("/api/v1/events/{event_id}/save"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(unsave_response.status_code(), 200);
        let unsave_payload: serde_json::Value = unsave_response.json();
        assert_eq!(unsave_payload["data"]["isSaved"], false);

        let resave_response = request
            .post(&format!("/api/v1/events/{event_id}/save"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(resave_response.status_code(), 200);

        let matching_events_response = request
            .get("/api/v1/matching/events")
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(matching_events_response.status_code(), 200);
        let matching_events_payload: serde_json::Value = matching_events_response.json();
        let matching_event_ids = matching_events_payload["data"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|row| row["id"].as_str().map(ToOwned::to_owned))
            .collect::<Vec<_>>();
        assert!(!matching_event_ids.iter().any(|id| id == &event_id));
        assert!(matching_event_ids
            .iter()
            .any(|id| id == &second_music_event_id));

        let list_response = request
            .get("/api/v1/events")
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(list_response.status_code(), 200);
        let list_payload: serde_json::Value = list_response.json();
        assert_eq!(list_payload["data"][0]["id"], event_id);
        assert_eq!(list_payload["data"][0]["maxAttendees"], 3);

        let suggestions_response = request
            .post("/api/v1/tags/suggestions")
            .json(&serde_json::json!({
                "scope": "event",
                "title": "music meetup",
                "description": "live music event",
            }))
            .await;
        assert_eq!(suggestions_response.status_code(), 200);
        let suggestions_payload: serde_json::Value = suggestions_response.json();
        assert!(suggestions_payload["data"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()));

        let interested_token = sign_up_and_verify(
            &request,
            "interested@example.com",
            "secret123",
            "Interested",
        )
        .await;
        let (interested_auth_key, interested_auth_value) = auth_header(&interested_token);
        let interested_profile_response = request
            .post("/api/v1/profiles")
            .add_header(interested_auth_key.clone(), interested_auth_value.clone())
            .json(&serde_json::json!({
                "name": "Interested",
                "age": 23,
            }))
            .await;
        assert_eq!(interested_profile_response.status_code(), 201);

        let interested_attend_response = request
            .post(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(interested_auth_key.clone(), interested_auth_value.clone())
            .json(&serde_json::json!({ "status": "interested" }))
            .await;
        assert_eq!(interested_attend_response.status_code(), 200);

        let interested_export_response = request
            .get("/api/v1/auth/export")
            .add_header(interested_auth_key.clone(), interested_auth_value.clone())
            .await;
        assert_eq!(interested_export_response.status_code(), 200);
        let interested_export_payload = parse_export_zip(interested_export_response.as_bytes());
        let interested_interactions = interested_export_payload["eventInteractions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(!interested_interactions
            .iter()
            .any(|row| row["kind"] == "joined"));

        let third_token =
            sign_up_and_verify(&request, "third@example.com", "secret123", "Third").await;

        let (third_auth_key, third_auth_value) = auth_header(&third_token);
        let third_profile_response = request
            .post("/api/v1/profiles")
            .add_header(third_auth_key.clone(), third_auth_value.clone())
            .json(&serde_json::json!({
                "name": "Third",
                "age": 24,
            }))
            .await;
        assert_eq!(third_profile_response.status_code(), 201);

        let full_update = request
            .patch(&format!("/api/v1/events/{event_id}"))
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "maxAttendees": 2
            }))
            .await;
        assert_eq!(full_update.status_code(), 200);
        let full_update_payload: serde_json::Value = full_update.json();
        assert_eq!(full_update_payload["data"]["maxAttendees"], 2);

        let blocked_attend = request
            .post(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(third_auth_key.clone(), third_auth_value.clone())
            .json(&serde_json::json!({ "status": "going" }))
            .await;
        assert_eq!(blocked_attend.status_code(), 400);
        let blocked_payload: serde_json::Value = blocked_attend.json();
        assert_eq!(blocked_payload["code"], "VALIDATION_ERROR");
        assert_eq!(blocked_payload["error"], "Event is full");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matching_and_uploads_endpoints_available() {
    request(|request, _ctx| async move {
        let token_a =
            sign_up_and_verify(&request, "match-a@example.com", "secret123", "Alice").await;
        let token_b = sign_up_and_verify(&request, "match-b@example.com", "secret123", "Bob").await;

        let (auth_key_a, auth_value_a) = auth_header(&token_a);
        let (auth_key_b, auth_value_b) = auth_header(&token_b);

        let profile_a = request
            .post("/api/v1/profiles")
            .add_header(auth_key_a.clone(), auth_value_a.clone())
            .json(&serde_json::json!({ "name": "Alice", "age": 21 }))
            .await;
        assert_eq!(profile_a.status_code(), 201);
        let profile_b = request
            .post("/api/v1/profiles")
            .add_header(auth_key_b.clone(), auth_value_b.clone())
            .json(&serde_json::json!({ "name": "Bob", "age": 22 }))
            .await;
        assert_eq!(profile_b.status_code(), 201);

        let matching_response = request
            .get("/api/v1/matching/profiles?limit=10")
            .add_header(auth_key_a.clone(), auth_value_a.clone())
            .await;
        assert_eq!(matching_response.status_code(), 200);
        let matching_payload: serde_json::Value = matching_response.json();
        assert_eq!(matching_payload["data"].as_array().map_or(0, Vec::len), 1);

        let uploads_auth_check_response = request
            .get("/api/v1/uploads/auth-check")
            .add_header(auth_key_a.clone(), auth_value_a.clone())
            .await;
        assert_eq!(uploads_auth_check_response.status_code(), 400);
        let uploads_auth_check_payload: serde_json::Value = uploads_auth_check_response.json();
        assert_eq!(uploads_auth_check_payload["code"], "MISSING_URI");

        let missing_upload_response = request
            .get("/api/v1/uploads/missing.png")
            .add_header(auth_key_a.clone(), auth_value_a.clone())
            .await;
        assert_eq!(missing_upload_response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ws_upgrade_rejects_foreign_origin() {
    request(|request, _ctx| async move {
        // The origin gate is a route-layer middleware, so it runs on the
        // raw Request before Axum's WebSocketUpgrade extractor performs
        // its own HTTP-version check. A plain GET with a foreign Origin
        // therefore reaches the gate and returns 403.
        let response = request
            .get("/api/v1/chat/ws")
            .add_header(
                HeaderName::from_static("origin"),
                HeaderValue::from_static("https://evil.example.com"),
            )
            .await;
        assert_eq!(response.status_code(), 403);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn ws_upgrade_is_rate_limited_per_ip() {
    request(|request, _ctx| async move {
        // Cap is 60/min (CHAT_WS_UPGRADE_MAX_PER_MIN). The rate-limit gate
        // sits in the same route-layer middleware as the origin check, so
        // a plain GET from the same IP 61 times yields 429 on the last one.
        let ip_header = (
            HeaderName::from_static("x-real-ip"),
            HeaderValue::from_static("203.0.113.200"),
        );

        let mut last_status = 0;
        for _ in 0..61 {
            let response = request
                .get("/api/v1/chat/ws")
                .add_header(ip_header.0.clone(), ip_header.1.clone())
                .await;
            last_status = response.status_code().as_u16();
        }
        assert_eq!(
            last_status, 429,
            "expected the 61st ws_upgrade to be rate-limited"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matching_profiles_is_rate_limited_per_ip() {
    request(|request, _ctx| async move {
        // Cap is 30/min (MATCHING_PROFILES_MAX_PER_MIN). The rate-limit
        // check fires before auth, so no session is needed. The 31st
        // request from the same forwarded IP should receive 429 with a
        // Retry-After hint in the 1..=60 range.
        let ip_header = (
            HeaderName::from_static("x-real-ip"),
            HeaderValue::from_static("203.0.113.201"),
        );

        let mut last_status = 0;
        let mut last_retry_after: Option<String> = None;
        for _ in 0..31 {
            let response = request
                .get("/api/v1/matching/profiles?limit=1")
                .add_header(ip_header.0.clone(), ip_header.1.clone())
                .await;
            last_status = response.status_code().as_u16();
            last_retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .map(ToOwned::to_owned);
        }
        assert_eq!(
            last_status, 429,
            "expected the 31st /matching/profiles to be rate-limited"
        );
        let retry_after = last_retry_after.expect("Retry-After header must be set on 429");
        let secs: u32 = retry_after.parse().expect("Retry-After must be an integer");
        assert!(
            (1..=60).contains(&secs),
            "Retry-After should be in (0, 60]s window, got {secs}"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn sign_in_verifies_hashed_password() {
    request(|request, _ctx| async move {
        let sign_up = request
            .post("/api/v1/auth/sign-up/email")
            .json(&sign_up_json("hash-test@example.com", "correct-password"))
            .await;
        assert_eq!(sign_up.status_code(), 200);
        let sign_up_payload: serde_json::Value = sign_up.json();
        assert!(sign_up_payload["data"]["token"].is_null());

        // Verify email via OTP before sign-in (email verification is required)
        let otp_code = get_otp_code("hash-test@example.com").await;
        let verify_response = request
            .post("/api/v1/auth/verify-otp")
            .json(&serde_json::json!({
                "email": "hash-test@example.com",
                "otp": otp_code,
            }))
            .await;
        assert_eq!(verify_response.status_code(), 200);

        // sign in with the correct password succeeds
        let sign_in_ok = request
            .post("/api/v1/auth/sign-in/email")
            .json(&serde_json::json!({
                "email": "hash-test@example.com",
                "password": "correct-password",
            }))
            .await;
        assert_eq!(sign_in_ok.status_code(), 200);
        let ok_payload: serde_json::Value = sign_in_ok.json();
        assert!(ok_payload["data"]["token"].is_string());

        // sign in with a wrong password fails
        let sign_in_bad = request
            .post("/api/v1/auth/sign-in/email")
            .json(&serde_json::json!({
                "email": "hash-test@example.com",
                "password": "wrong-password",
            }))
            .await;
        assert_eq!(sign_in_bad.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn unverified_sign_in_requires_otp_and_resend_works() {
    request(|request, _ctx| async move {
        let sign_up = request
            .post("/api/v1/auth/sign-up/email")
            .json(&sign_up_json("otp-flow@example.com", "correct-password"))
            .await;
        assert_eq!(sign_up.status_code(), 200);
        let sign_up_payload: serde_json::Value = sign_up.json();
        assert!(sign_up_payload["data"]["token"].is_null());

        let sign_in_unverified = request
            .post("/api/v1/auth/sign-in/email")
            .json(&serde_json::json!({
                "email": "otp-flow@example.com",
                "password": "correct-password",
            }))
            .await;
        assert_eq!(sign_in_unverified.status_code(), 403);
        let sign_in_error: serde_json::Value = sign_in_unverified.json();
        assert_eq!(sign_in_error["code"], "EMAIL_NOT_VERIFIED");

        let resend = request
            .post("/api/v1/auth/resend-otp")
            .json(&serde_json::json!({ "email": "otp-flow@example.com" }))
            .await;
        assert_eq!(resend.status_code(), 200);
        let resend_payload: serde_json::Value = resend.json();
        assert_eq!(resend_payload["success"], true);

        let otp_code = get_otp_code("otp-flow@example.com").await;
        let verify_response = request
            .post("/api/v1/auth/verify-otp")
            .json(&serde_json::json!({
                "email": "otp-flow@example.com",
                "otp": otp_code,
            }))
            .await;
        assert_eq!(verify_response.status_code(), 200);
        let verify_payload: serde_json::Value = verify_response.json();
        assert!(verify_payload["data"]["token"].is_string());
        assert_eq!(verify_payload["data"]["user"]["emailVerified"], true);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn delete_account_verifies_hashed_password() {
    request(|request, _ctx| async move {
        let token = sign_up_and_verify(
            &request,
            "delete-test@example.com",
            "my-password",
            "Test User",
        )
        .await;

        let (auth_key, auth_value) = auth_header(&token);

        // delete with wrong password fails
        let delete_bad = request
            .delete("/api/v1/auth/account")
            .add_header(auth_key.clone(), auth_value.clone())
            .json(&serde_json::json!({ "password": "wrong-password" }))
            .await;
        assert_eq!(delete_bad.status_code(), 401);

        // delete with correct password succeeds
        let delete_ok = request
            .delete("/api/v1/auth/account")
            .add_header(auth_key.clone(), auth_value.clone())
            .json(&serde_json::json!({ "password": "my-password" }))
            .await;
        assert_eq!(delete_ok.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn change_password_rotates_credentials_and_invalidates_sessions() {
    request(|request, _ctx| async move {
        let token = sign_up_and_verify(
            &request,
            "change-password@example.com",
            "old-password",
            "Password User",
        )
        .await;

        let (auth_key, auth_value) = auth_header(&token);

        let change_bad = request
            .patch("/api/v1/auth/account/password")
            .add_header(auth_key.clone(), auth_value.clone())
            .json(&serde_json::json!({
                "currentPassword": "wrong-password",
                "newPassword": "new-password-123",
            }))
            .await;
        assert_eq!(change_bad.status_code(), 401);

        let change_ok = request
            .patch("/api/v1/auth/account/password")
            .add_header(auth_key.clone(), auth_value.clone())
            .json(&serde_json::json!({
                "currentPassword": "old-password",
                "newPassword": "new-password-123",
            }))
            .await;
        assert_eq!(change_ok.status_code(), 200);

        let old_sessions = request
            .get("/api/v1/auth/sessions")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(old_sessions.status_code(), 401);

        let old_sign_in = request
            .post("/api/v1/auth/sign-in/email")
            .json(&serde_json::json!({
                "email": "change-password@example.com",
                "password": "old-password",
            }))
            .await;
        assert_eq!(old_sign_in.status_code(), 401);

        let new_sign_in = request
            .post("/api/v1/auth/sign-in/email")
            .json(&serde_json::json!({
                "email": "change-password@example.com",
                "password": "new-password-123",
            }))
            .await;
        assert_eq!(new_sign_in.status_code(), 200);
        let new_payload: serde_json::Value = new_sign_in.json();
        assert!(new_payload["data"]["token"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn uploads_auth_check_accepts_owned_variant_url() {
    request(|request, _ctx| async move {
        let (auth_key, auth_value) =
            create_user_with_profile(&request, "variant-owner@example.com", "Variant Owner").await;

        let upload_payload = upload_png(&request, &auth_key, &auth_value).await;
        let thumbnail_url = upload_payload["data"]["thumbnail_url"]
            .as_str()
            .map(ToOwned::to_owned)
            .expect("thumbnail_url should be present");
        let thumb_filename = thumbnail_url
            .strip_prefix("/api/v1/uploads/")
            .map(ToOwned::to_owned)
            .expect("dev thumbnail URL shape");

        let auth_check = request
            .get("/api/v1/uploads/auth-check")
            .add_header(auth_key, auth_value)
            .add_header(
                HeaderName::from_static("x-original-uri"),
                HeaderValue::from_str(&format!("/uploads/{thumb_filename}")).expect("valid header"),
            )
            .await;

        assert_eq!(auth_check.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn uploads_auth_check_rejects_other_users_variant_url() {
    request(|request, _ctx| async move {
        let (owner_key, owner_value) =
            create_user_with_profile(&request, "idor-owner@example.com", "Owner").await;
        let (attacker_key, attacker_value) =
            create_user_with_profile(&request, "idor-attacker@example.com", "Attacker").await;

        let upload_payload = upload_png(&request, &owner_key, &owner_value).await;
        let owner_filename = upload_payload["data"]["filename"]
            .as_str()
            .map(ToOwned::to_owned)
            .expect("filename should be present");
        let thumb_filename = upload_payload["data"]["thumbnail_url"]
            .as_str()
            .and_then(|url| url.strip_prefix("/api/v1/uploads/"))
            .map(ToOwned::to_owned)
            .expect("dev thumbnail URL shape");

        // Attacker tries to auth-check the original: must not succeed.
        let auth_check_original = request
            .get("/api/v1/uploads/auth-check")
            .add_header(attacker_key.clone(), attacker_value.clone())
            .add_header(
                HeaderName::from_static("x-original-uri"),
                HeaderValue::from_str(&format!("/uploads/{owner_filename}")).expect("valid header"),
            )
            .await;
        assert_ne!(
            auth_check_original.status_code(),
            200,
            "attacker must not auth-check owner's original file"
        );

        // Attacker tries to auth-check the thumb variant: must not succeed.
        let auth_check_variant = request
            .get("/api/v1/uploads/auth-check")
            .add_header(attacker_key, attacker_value)
            .add_header(
                HeaderName::from_static("x-original-uri"),
                HeaderValue::from_str(&format!("/uploads/{thumb_filename}")).expect("valid header"),
            )
            .await;
        assert_ne!(
            auth_check_variant.status_code(),
            200,
            "attacker must not auth-check owner's variant file"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn upload_returns_error_when_upload_row_insert_fails() {
    request(|request, _ctx| async move {
        use diesel_async::RunQueryDsl;

        let (auth_key, auth_value) =
            create_user_with_profile(&request, "insert-fail@example.com", "Insert Fail").await;

        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        diesel::sql_query(
            r"CREATE OR REPLACE FUNCTION test_fail_uploads_insert() RETURNS trigger AS $$
            BEGIN RAISE EXCEPTION 'forced uploads insert failure'; END; $$ LANGUAGE plpgsql",
        )
        .execute(&mut conn)
        .await
        .expect("create insert-fail function");
        diesel::sql_query("DROP TRIGGER IF EXISTS trg_test_fail_uploads_insert ON uploads")
            .execute(&mut conn)
            .await
            .expect("drop old insert-fail trigger");
        diesel::sql_query(
            r"CREATE TRIGGER trg_test_fail_uploads_insert
            BEFORE INSERT ON uploads FOR EACH ROW EXECUTE FUNCTION test_fail_uploads_insert()",
        )
        .execute(&mut conn)
        .await
        .expect("create insert-fail trigger");

        let form = MultipartForm::new()
            .add_text("context", "profile_gallery")
            .add_part(
                "file",
                Part::bytes(tiny_png_bytes())
                    .file_name("insert-fail.png")
                    .mime_type("image/png"),
            );
        let upload = request
            .post("/api/v1/uploads")
            .add_header(auth_key, auth_value)
            .multipart(form)
            .await;

        diesel::sql_query("DROP TRIGGER IF EXISTS trg_test_fail_uploads_insert ON uploads")
            .execute(&mut conn)
            .await
            .expect("drop insert-fail trigger");
        diesel::sql_query("DROP FUNCTION IF EXISTS test_fail_uploads_insert()")
            .execute(&mut conn)
            .await
            .expect("drop insert-fail function");

        assert_eq!(upload.status_code(), 500);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn delete_returns_error_when_upload_row_update_fails() {
    request(|request, _ctx| async move {
        use diesel_async::RunQueryDsl;

        let (auth_key, auth_value) =
            create_user_with_profile(&request, "update-fail@example.com", "Update Fail").await;
        let upload_payload = upload_png(&request, &auth_key, &auth_value).await;
        let filename = upload_payload["data"]["filename"]
            .as_str()
            .map(ToOwned::to_owned)
            .expect("filename should be present");

        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        diesel::sql_query(
            r"CREATE OR REPLACE FUNCTION test_fail_uploads_update() RETURNS trigger AS $$
            BEGIN RAISE EXCEPTION 'forced uploads update failure'; END; $$ LANGUAGE plpgsql",
        )
        .execute(&mut conn)
        .await
        .expect("create update-fail function");
        diesel::sql_query("DROP TRIGGER IF EXISTS trg_test_fail_uploads_update ON uploads")
            .execute(&mut conn)
            .await
            .expect("drop old update-fail trigger");
        diesel::sql_query(
            r"CREATE TRIGGER trg_test_fail_uploads_update
            BEFORE UPDATE ON uploads FOR EACH ROW EXECUTE FUNCTION test_fail_uploads_update()",
        )
        .execute(&mut conn)
        .await
        .expect("create update-fail trigger");

        let delete = request
            .delete(&format!("/api/v1/uploads/{filename}"))
            .add_header(auth_key, auth_value)
            .await;

        diesel::sql_query("DROP TRIGGER IF EXISTS trg_test_fail_uploads_update ON uploads")
            .execute(&mut conn)
            .await
            .expect("drop update-fail trigger");
        diesel::sql_query("DROP FUNCTION IF EXISTS test_fail_uploads_update()")
            .execute(&mut conn)
            .await
            .expect("drop update-fail function");

        assert_eq!(delete.status_code(), 500);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn sign_up_existing_email_returns_generic_success_and_keeps_single_user() {
    request(|request, _ctx| async move {
        use diesel::dsl::count_star;
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use poziomki_backend::db::models::users::User;
        use poziomki_backend::db::schema::users;

        let email = "existing-signup@example.com";

        let first = request
            .post("/api/v1/auth/sign-up/email")
            .json(&sign_up_json(email, "first-password"))
            .await;
        assert_eq!(first.status_code(), 200);

        let second = request
            .post("/api/v1/auth/sign-up/email")
            .json(&sign_up_json(email, "different-password"))
            .await;
        assert_eq!(second.status_code(), 200);
        let second_payload: serde_json::Value = second.json();
        assert!(second_payload["data"]["token"].is_null());
        assert_eq!(second_payload["data"]["user"]["email"], email);

        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        let count = users::table
            .filter(users::email.eq(email))
            .select(count_star())
            .first::<i64>(&mut conn)
            .await
            .expect("count users by email");
        assert_eq!(count, 1);

        let saved = users::table
            .filter(users::email.eq(email))
            .first::<User>(&mut conn)
            .await
            .expect("load saved user");
        assert!(
            poziomki_backend::security::verify_password("first-password", &saved.password),
            "second sign-up must not overwrite existing password hash"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn sign_in_rate_limit_not_bypassed_by_spoofed_forwarded_for_headers() {
    request(|request, _ctx| async move {
        let email = "no-account-rate-limit@example.com";
        let password = "wrong-password";

        for i in 1..=8 {
            let response = request
                .post("/api/v1/auth/sign-in/email")
                .add_header(
                    HeaderName::from_static("x-forwarded-for"),
                    HeaderValue::from_str(&format!("203.0.113.{i}")).expect("valid ip header"),
                )
                .json(&serde_json::json!({
                    "email": email,
                    "password": password,
                }))
                .await;
            assert_eq!(response.status_code(), 401);
        }

        let limited = request
            .post("/api/v1/auth/sign-in/email")
            .add_header(
                HeaderName::from_static("x-forwarded-for"),
                HeaderValue::from_static("198.51.100.250"),
            )
            .json(&serde_json::json!({
                "email": email,
                "password": password,
            }))
            .await;
        assert_eq!(limited.status_code(), 429);
    })
    .await;
}

// ---------------------------------------------------------------------------
// Phase-6 events coverage: endpoints the phase-3 contract test doesn't hit.
// These exercise the viewer-scoped transaction wrappers on the endpoints that
// were otherwise only reachable via phased scenarios.
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn event_attendees_listing_returns_user_pids() {
    request(|request, _ctx| async move {
        let (owner_key, owner_value) =
            create_user_with_profile(&request, "attlist-owner@example.com", "Owner").await;
        let (attendee_key, attendee_value) =
            create_user_with_profile(&request, "attlist-att@example.com", "Attendee").await;

        let create = request
            .post("/api/v1/events")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&serde_json::json!({
                "title": "Attendee listing",
                "startsAt": "2030-06-01T12:00:00Z",
            }))
            .await;
        assert_eq!(create.status_code(), 201);
        let event_id = create.json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("event id")
            .to_string();

        let attend = request
            .post(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(attendee_key.clone(), attendee_value.clone())
            .json(&serde_json::json!({ "status": "going" }))
            .await;
        assert_eq!(attend.status_code(), 200);

        let attendees = request
            .get(&format!("/api/v1/events/{event_id}/attendees"))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(attendees.status_code(), 200);
        let payload: serde_json::Value = attendees.json();
        let rows = payload["data"].as_array().expect("attendees array");
        assert_eq!(rows.len(), 2);
        // The narrow `app.user_pids_for_ids` helper returns real pids — not
        // the nil UUID fallback. Both attendees should expose non-nil userIds
        // so the mobile client can open DMs or link to profiles.
        for row in rows {
            let uid = row["userId"].as_str().unwrap_or_default();
            assert_ne!(uid, "00000000-0000-0000-0000-000000000000");
            assert!(uuid::Uuid::parse_str(uid).is_ok(), "userId must be a uuid");
        }
    })
    .await;
}

#[tokio::test]
#[serial]
async fn event_delete_removes_event_and_is_creator_only() {
    request(|request, _ctx| async move {
        let (owner_key, owner_value) =
            create_user_with_profile(&request, "del-owner@example.com", "Owner").await;
        let (other_key, other_value) =
            create_user_with_profile(&request, "del-other@example.com", "Other").await;

        let create = request
            .post("/api/v1/events")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&serde_json::json!({
                "title": "To delete",
                "startsAt": "2030-06-02T12:00:00Z",
            }))
            .await;
        let event_id = create.json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("event id")
            .to_string();

        // Non-creator gets 403
        let forbidden = request
            .delete(&format!("/api/v1/events/{event_id}"))
            .add_header(other_key.clone(), other_value.clone())
            .await;
        assert_eq!(forbidden.status_code(), 403);

        // Creator deletes successfully
        let deleted = request
            .delete(&format!("/api/v1/events/{event_id}"))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(deleted.status_code(), 200);

        // GET returns 404
        let after = request
            .get(&format!("/api/v1/events/{event_id}"))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(after.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn event_leave_removes_attendee_and_blocks_creator() {
    request(|request, _ctx| async move {
        let (owner_key, owner_value) =
            create_user_with_profile(&request, "leave-owner@example.com", "Owner").await;
        let (att_key, att_value) =
            create_user_with_profile(&request, "leave-att@example.com", "Att").await;

        let event_id = request
            .post("/api/v1/events")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&serde_json::json!({
                "title": "Leave flow",
                "startsAt": "2030-06-03T12:00:00Z",
            }))
            .await
            .json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("event id")
            .to_string();

        // Creator cannot leave their own event
        let creator_leave = request
            .delete(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(creator_leave.status_code(), 403);

        // Attendee joins then leaves
        let attend = request
            .post(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(att_key.clone(), att_value.clone())
            .json(&serde_json::json!({ "status": "going" }))
            .await;
        assert_eq!(attend.status_code(), 200);

        let leave = request
            .delete(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(att_key.clone(), att_value.clone())
            .await;
        assert_eq!(leave.status_code(), 200);
        let payload: serde_json::Value = leave.json();
        assert_eq!(payload["data"]["isAttending"], false);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn event_approve_and_reject_flow() {
    request(|request, _ctx| async move {
        let (owner_key, owner_value) =
            create_user_with_profile(&request, "approve-owner@example.com", "Owner").await;
        let (ok_key, ok_value) =
            create_user_with_profile(&request, "approve-ok@example.com", "Ok").await;
        let (rej_key, rej_value) =
            create_user_with_profile(&request, "approve-rej@example.com", "Rej").await;

        let event = request
            .post("/api/v1/events")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&serde_json::json!({
                "title": "Approvals",
                "startsAt": "2030-06-04T12:00:00Z",
                "requiresApproval": true,
            }))
            .await;
        let event_id = event.json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("event id")
            .to_string();

        // Both attendees request to go — approval gate downgrades to pending.
        for (k, v) in [
            (ok_key.clone(), ok_value.clone()),
            (rej_key.clone(), rej_value.clone()),
        ] {
            let r = request
                .post(&format!("/api/v1/events/{event_id}/attend"))
                .add_header(k, v)
                .json(&serde_json::json!({ "status": "going" }))
                .await;
            assert_eq!(r.status_code(), 200);
            let p: serde_json::Value = r.json();
            assert_eq!(p["data"]["isPending"], true);
            assert_eq!(p["data"]["isAttending"], false);
        }

        // Rejecting a non-creator is 403.
        let ok_profile_id = request
            .get("/api/v1/profiles/me")
            .add_header(ok_key.clone(), ok_value.clone())
            .await
            .json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("profile id")
            .to_string();
        let rej_profile_id = request
            .get("/api/v1/profiles/me")
            .add_header(rej_key.clone(), rej_value.clone())
            .await
            .json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("profile id")
            .to_string();

        let non_creator_approve = request
            .post(&format!(
                "/api/v1/events/{event_id}/attendees/{ok_profile_id}/approve"
            ))
            .add_header(ok_key.clone(), ok_value.clone())
            .await;
        assert_eq!(non_creator_approve.status_code(), 403);

        // Creator approves one and rejects the other.
        let approve = request
            .post(&format!(
                "/api/v1/events/{event_id}/attendees/{ok_profile_id}/approve"
            ))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(approve.status_code(), 200);

        let reject = request
            .post(&format!(
                "/api/v1/events/{event_id}/attendees/{rej_profile_id}/reject"
            ))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(reject.status_code(), 200);

        // Bad profile id → 400 BAD_REQUEST, not 500.
        let bad = request
            .post(&format!(
                "/api/v1/events/{event_id}/attendees/not-a-uuid/approve"
            ))
            .add_header(owner_key.clone(), owner_value.clone())
            .await;
        assert_eq!(bad.status_code(), 400);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn event_report_flow() {
    request(|request, _ctx| async move {
        let (owner_key, owner_value) =
            create_user_with_profile(&request, "report-owner@example.com", "Owner").await;
        let (reporter_key, reporter_value) =
            create_user_with_profile(&request, "report-reporter@example.com", "Reporter").await;

        let event_id = request
            .post("/api/v1/events")
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&serde_json::json!({
                "title": "Report target",
                "startsAt": "2030-06-05T12:00:00Z",
            }))
            .await
            .json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("event id")
            .to_string();

        // Owner reporting own event → 403
        let self_report = request
            .post(&format!("/api/v1/events/{event_id}/report"))
            .add_header(owner_key.clone(), owner_value.clone())
            .json(&serde_json::json!({ "reason": "spam" }))
            .await;
        assert_eq!(self_report.status_code(), 403);

        // Invalid reason → 400
        let bad_reason = request
            .post(&format!("/api/v1/events/{event_id}/report"))
            .add_header(reporter_key.clone(), reporter_value.clone())
            .json(&serde_json::json!({ "reason": "not-a-reason" }))
            .await;
        assert_eq!(bad_reason.status_code(), 400);

        // First report → 200 success
        let ok = request
            .post(&format!("/api/v1/events/{event_id}/report"))
            .add_header(reporter_key.clone(), reporter_value.clone())
            .json(&serde_json::json!({ "reason": "spam" }))
            .await;
        assert_eq!(ok.status_code(), 200);

        // Duplicate → 400 VALIDATION_ERROR
        let dup = request
            .post(&format!("/api/v1/events/{event_id}/report"))
            .add_header(reporter_key.clone(), reporter_value.clone())
            .json(&serde_json::json!({ "reason": "spam" }))
            .await;
        assert_eq!(dup.status_code(), 400);

        // Missing event → 404
        let missing_id = uuid::Uuid::new_v4();
        let missing = request
            .post(&format!("/api/v1/events/{missing_id}/report"))
            .add_header(reporter_key.clone(), reporter_value.clone())
            .json(&serde_json::json!({ "reason": "spam" }))
            .await;
        assert_eq!(missing.status_code(), 404);
    })
    .await;
}

// Regression: PATCH /profiles/{id} with `images: [F, F]` was rejected
// because `verify_uploads_ownership` compared `owned.len()` to the raw
// `filenames.len()` while SQL `IN` dedupes — owned came back as 1 even
// when F was owned. Set-membership now passes iff every requested
// filename is owned, so duplicates are accepted.
#[tokio::test]
#[serial]
async fn profile_images_patch_accepts_duplicate_owned_filename() {
    request(|request, _ctx| async move {
        let (auth_key, auth_value) =
            create_user_with_profile(&request, "dup_owner@example.com", "Dup").await;

        let upload = upload_png(&request, &auth_key, &auth_value).await;
        let filename = upload["data"]["filename"]
            .as_str()
            .expect("upload returns filename")
            .to_string();

        let me = request
            .get("/api/v1/profiles/me")
            .add_header(auth_key.clone(), auth_value.clone())
            .await;
        assert_eq!(me.status_code(), 200);
        let profile_id = me.json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("profile id")
            .to_string();

        let resp = request
            .patch(&format!("/api/v1/profiles/{profile_id}"))
            .add_header(auth_key.clone(), auth_value.clone())
            .json(&serde_json::json!({
                "images": [filename.clone(), filename.clone()],
            }))
            .await;
        assert_eq!(
            resp.status_code(),
            200,
            "duplicate owned filenames must be accepted, body: {}",
            resp.text()
        );

        // And both copies of the filename should appear in the rendered
        // gallery — server returns signed URLs, so just look for the
        // base filename as a substring twice.
        let payload: serde_json::Value = resp.json();
        let images = payload["data"]["images"].as_array().expect("images array");
        assert_eq!(images.len(), 2);
        for url in images {
            let s = url.as_str().expect("image url is a string");
            assert!(
                s.contains(&filename),
                "expected url {s} to reference {filename}"
            );
        }
    })
    .await;
}

// Companion negative test: a stranger's filename in `images[]` must be
// rejected. Without the ownership check, the API would issue signed
// URLs over the original owner's bytes — a content-theft, no-copy.
#[tokio::test]
#[serial]
async fn profile_images_patch_rejects_other_users_filename() {
    request(|request, _ctx| async move {
        let (a_key, a_val) = create_user_with_profile(&request, "stealer_a@example.com", "A").await;
        let upload_a = upload_png(&request, &a_key, &a_val).await;
        let a_filename = upload_a["data"]["filename"]
            .as_str()
            .expect("a filename")
            .to_string();

        let (b_key, b_val) = create_user_with_profile(&request, "stealer_b@example.com", "B").await;
        let upload_b = upload_png(&request, &b_key, &b_val).await;
        let b_filename = upload_b["data"]["filename"]
            .as_str()
            .expect("b filename")
            .to_string();

        let me_b = request
            .get("/api/v1/profiles/me")
            .add_header(b_key.clone(), b_val.clone())
            .await;
        let b_profile_id = me_b.json::<serde_json::Value>()["data"]["id"]
            .as_str()
            .expect("b profile id")
            .to_string();

        let resp = request
            .patch(&format!("/api/v1/profiles/{b_profile_id}"))
            .add_header(b_key.clone(), b_val.clone())
            .json(&serde_json::json!({
                "images": [b_filename.clone(), a_filename.clone()],
            }))
            .await;
        assert_eq!(
            resp.status_code(),
            400,
            "B must not be able to claim A's filename, body: {}",
            resp.text()
        );
    })
    .await;
}
