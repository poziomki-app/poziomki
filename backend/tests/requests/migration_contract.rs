use axum::http::{HeaderName, HeaderValue};
use axum_test::TestServer;
use axum_test::multipart::{MultipartForm, Part};
use base64::Engine as _;
use chrono::Utc;
use serial_test::serial;
use std::future::Future;

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
    base64::engine::general_purpose::STANDARD
        .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO2p9N8AAAAASUVORK5CYII=")
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
async fn matrix_session_requires_auth() {
    request(|request, _ctx| async move {
        let response = request
            .post("/api/v1/matrix/session")
            .json(&serde_json::json!({}))
            .await;
        assert_eq!(response.status_code(), 401);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["code"], "UNAUTHORIZED");
        assert!(payload["requestId"].is_string());
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
async fn matrix_config_endpoint_available() {
    request(|request, _ctx| async move {
        let response = request.get("/api/v1/matrix/config").await;
        assert_eq!(response.status_code(), 200);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["data"]["chat_mode"], "matrix-native");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matrix_session_is_not_cacheable() {
    request(|request, _ctx| async move {
        let response = request
            .post("/api/v1/matrix/session")
            .json(&serde_json::json!({}))
            .await;
        assert_eq!(response.status_code(), 401);

        let cache_control = response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert_eq!(cache_control, "no-store");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matrix_config_is_not_cacheable() {
    request(|request, _ctx| async move {
        let response = request.get("/api/v1/matrix/config").await;
        assert_eq!(response.status_code(), 200);

        let cache_control = response
            .headers()
            .get("cache-control")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        assert_eq!(cache_control, "no-store");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matrix_event_room_returns_existing_mapping_for_attendee() {
    request(|request, _ctx| async move {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use poziomki_backend::db::schema::events;
        use uuid::Uuid;

        let owner_token = sign_up_and_verify(
            &request,
            "event-room-owner@example.com",
            "secret123",
            "Owner",
        )
        .await;
        let (owner_auth_key, owner_auth_value) = auth_header(&owner_token);

        let profile_response = request
            .post("/api/v1/profiles")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "name": "Owner",
                "age": 24,
            }))
            .await;
        assert_eq!(profile_response.status_code(), 201);

        let create_event = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Room mapping event",
                "startsAt": "2030-01-10T12:00:00Z",
            }))
            .await;
        assert_eq!(create_event.status_code(), 201);
        let created_payload: serde_json::Value = create_event.json();
        let event_id = created_payload["data"]["id"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        assert!(!event_id.is_empty());

        let event_uuid = Uuid::parse_str(&event_id).expect("valid event UUID");
        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        diesel::update(events::table.filter(events::id.eq(event_uuid)))
            .set((
                events::conversation_id.eq(Some("!eventcanon:chat.poziomki.app")),
                events::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .await
            .expect("update event");

        let room_response = request
            .get(&format!("/api/v1/matrix/events/{event_id}/room"))
            .add_header(owner_auth_key, owner_auth_value)
            .await;
        assert_eq!(room_response.status_code(), 200);
        let room_payload: serde_json::Value = room_response.json();
        assert_eq!(
            room_payload["data"]["roomId"],
            "!eventcanon:chat.poziomki.app"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matrix_dm_endpoint_returns_existing_canonical_mapping() {
    request(|request, _ctx| async move {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use poziomki_backend::db::models::matrix_dm_rooms::NewMatrixDmRoom;
        use poziomki_backend::db::models::users::User;
        use poziomki_backend::db::schema::{matrix_dm_rooms, users};

        let token_a = sign_up_and_verify(&request, "dm-a@example.com", "secret123", "Alice").await;
        let token_b = sign_up_and_verify(&request, "dm-b@example.com", "secret123", "Bob").await;

        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        let user_a: User = users::table
            .filter(users::email.eq("dm-a@example.com"))
            .first(&mut conn)
            .await
            .expect("query user A");
        let user_b: User = users::table
            .filter(users::email.eq("dm-b@example.com"))
            .first(&mut conn)
            .await
            .expect("query user B");

        let (low, high) = if user_a.pid <= user_b.pid {
            (user_a.pid, user_b.pid)
        } else {
            (user_b.pid, user_a.pid)
        };

        let now = Utc::now();
        diesel::insert_into(matrix_dm_rooms::table)
            .values(NewMatrixDmRoom {
                id: uuid::Uuid::new_v4(),
                user_low_pid: low,
                user_high_pid: high,
                room_id: "!dmcanon:chat.poziomki.app".to_string(),
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("insert dm mapping");

        let (auth_key_a, auth_value_a) = auth_header(&token_a);
        let dm_response = request
            .post("/api/v1/matrix/dms")
            .add_header(auth_key_a, auth_value_a)
            .json(&serde_json::json!({ "userId": user_b.pid.to_string() }))
            .await;

        assert_eq!(dm_response.status_code(), 200);
        let payload: serde_json::Value = dm_response.json();
        assert_eq!(payload["data"]["roomId"], "!dmcanon:chat.poziomki.app");
        let _ = token_b;
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matrix_dm_endpoint_is_symmetric_for_both_users() {
    request(|request, _ctx| async move {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use poziomki_backend::db::models::matrix_dm_rooms::NewMatrixDmRoom;
        use poziomki_backend::db::models::users::User;
        use poziomki_backend::db::schema::{matrix_dm_rooms, users};

        let token_a =
            sign_up_and_verify(&request, "dm-symmetric-a@example.com", "secret123", "Alice").await;
        let token_b =
            sign_up_and_verify(&request, "dm-symmetric-b@example.com", "secret123", "Bob").await;

        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        let user_a: User = users::table
            .filter(users::email.eq("dm-symmetric-a@example.com"))
            .first(&mut conn)
            .await
            .expect("query user A");
        let user_b: User = users::table
            .filter(users::email.eq("dm-symmetric-b@example.com"))
            .first(&mut conn)
            .await
            .expect("query user B");

        let (low, high) = if user_a.pid <= user_b.pid {
            (user_a.pid, user_b.pid)
        } else {
            (user_b.pid, user_a.pid)
        };

        let now = Utc::now();
        diesel::insert_into(matrix_dm_rooms::table)
            .values(NewMatrixDmRoom {
                id: uuid::Uuid::new_v4(),
                user_low_pid: low,
                user_high_pid: high,
                room_id: "!dmsymmetric:chat.poziomki.app".to_string(),
                created_at: now,
                updated_at: now,
            })
            .execute(&mut conn)
            .await
            .expect("insert dm mapping");

        let (auth_key_a, auth_value_a) = auth_header(&token_a);
        let response_a = request
            .post("/api/v1/matrix/dms")
            .add_header(auth_key_a, auth_value_a)
            .json(&serde_json::json!({ "userId": user_b.pid.to_string() }))
            .await;
        assert_eq!(response_a.status_code(), 200);
        let payload_a: serde_json::Value = response_a.json();
        assert_eq!(
            payload_a["data"]["roomId"],
            "!dmsymmetric:chat.poziomki.app"
        );

        let (auth_key_b, auth_value_b) = auth_header(&token_b);
        let response_b = request
            .post("/api/v1/matrix/dms")
            .add_header(auth_key_b, auth_value_b)
            .json(&serde_json::json!({ "userId": user_a.pid.to_string() }))
            .await;
        assert_eq!(response_b.status_code(), 200);
        let payload_b: serde_json::Value = response_b.json();
        assert_eq!(
            payload_b["data"]["roomId"],
            "!dmsymmetric:chat.poziomki.app"
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn matrix_event_room_access_tracks_attendance() {
    request(|request, _ctx| async move {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;
        use poziomki_backend::db::schema::events;
        use uuid::Uuid;

        let owner_token = sign_up_and_verify(
            &request,
            "event-access-owner@example.com",
            "secret123",
            "Owner",
        )
        .await;
        let attendee_token = sign_up_and_verify(
            &request,
            "event-access-attendee@example.com",
            "secret123",
            "Attendee",
        )
        .await;

        let (owner_auth_key, owner_auth_value) = auth_header(&owner_token);
        let owner_profile = request
            .post("/api/v1/profiles")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({ "name": "Owner", "age": 24 }))
            .await;
        assert_eq!(owner_profile.status_code(), 201);

        let (attendee_auth_key, attendee_auth_value) = auth_header(&attendee_token);
        let attendee_profile = request
            .post("/api/v1/profiles")
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .json(&serde_json::json!({ "name": "Attendee", "age": 23 }))
            .await;
        assert_eq!(attendee_profile.status_code(), 201);

        let create_event = request
            .post("/api/v1/events")
            .add_header(owner_auth_key.clone(), owner_auth_value.clone())
            .json(&serde_json::json!({
                "title": "Attendance gate room",
                "startsAt": "2031-01-10T12:00:00Z",
            }))
            .await;
        assert_eq!(create_event.status_code(), 201);
        let created_payload: serde_json::Value = create_event.json();
        let event_id = created_payload["data"]["id"]
            .as_str()
            .map(ToOwned::to_owned)
            .expect("event id should exist");

        let event_uuid = Uuid::parse_str(&event_id).expect("valid event UUID");
        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        diesel::update(events::table.filter(events::id.eq(event_uuid)))
            .set((
                events::conversation_id.eq(Some("!eventaccess:chat.poziomki.app")),
                events::updated_at.eq(Utc::now()),
            ))
            .execute(&mut conn)
            .await
            .expect("update event");

        let pre_attend = request
            .get(&format!("/api/v1/matrix/events/{event_id}/room"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(pre_attend.status_code(), 403);

        let attend = request
            .post(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .json(&serde_json::json!({ "status": "going" }))
            .await;
        assert_eq!(attend.status_code(), 200);

        let post_attend = request
            .get(&format!("/api/v1/matrix/events/{event_id}/room"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(post_attend.status_code(), 200);
        let room_payload: serde_json::Value = post_attend.json();
        assert_eq!(
            room_payload["data"]["roomId"],
            "!eventaccess:chat.poziomki.app"
        );

        let leave = request
            .delete(&format!("/api/v1/events/{event_id}/attend"))
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(leave.status_code(), 200);

        let post_leave = request
            .get(&format!("/api/v1/matrix/events/{event_id}/room"))
            .add_header(attendee_auth_key, attendee_auth_value)
            .await;
        assert_eq!(post_leave.status_code(), 403);
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
        assert_eq!(created_payload["data"]["isAttending"], true);
        assert_eq!(created_payload["data"]["latitude"], serde_json::Value::Null);
        assert_eq!(
            created_payload["data"]["longitude"],
            serde_json::Value::Null
        );

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
                "longitude": 19.9450
            }))
            .await;
        assert_eq!(update_geo.status_code(), 200);
        let updated_payload: serde_json::Value = update_geo.json();
        assert_eq!(updated_payload["data"]["latitude"], 50.0647);
        assert_eq!(updated_payload["data"]["longitude"], 19.945);

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

        let list_response = request
            .get("/api/v1/events")
            .add_header(attendee_auth_key.clone(), attendee_auth_value.clone())
            .await;
        assert_eq!(list_response.status_code(), 200);
        let list_payload: serde_json::Value = list_response.json();
        assert_eq!(list_payload["data"][0]["id"], event_id);
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
async fn upload_returns_error_when_upload_row_insert_fails() {
    request(|request, _ctx| async move {
        use diesel_async::RunQueryDsl;

        let (auth_key, auth_value) =
            create_user_with_profile(&request, "insert-fail@example.com", "Insert Fail").await;

        let mut conn = poziomki_backend::db::conn().await.expect("get DB conn");
        diesel::sql_query(
            r"
            CREATE OR REPLACE FUNCTION test_fail_uploads_insert() RETURNS trigger AS $$
            BEGIN
              RAISE EXCEPTION 'forced uploads insert failure';
            END;
            $$ LANGUAGE plpgsql;

            DROP TRIGGER IF EXISTS trg_test_fail_uploads_insert ON uploads;
            CREATE TRIGGER trg_test_fail_uploads_insert
            BEFORE INSERT ON uploads
            FOR EACH ROW
            EXECUTE FUNCTION test_fail_uploads_insert();
            ",
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

        diesel::sql_query(
            r"
            DROP TRIGGER IF EXISTS trg_test_fail_uploads_insert ON uploads;
            DROP FUNCTION IF EXISTS test_fail_uploads_insert();
            ",
        )
        .execute(&mut conn)
        .await
        .expect("drop insert-fail trigger");

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
            r"
            CREATE OR REPLACE FUNCTION test_fail_uploads_update() RETURNS trigger AS $$
            BEGIN
              RAISE EXCEPTION 'forced uploads update failure';
            END;
            $$ LANGUAGE plpgsql;

            DROP TRIGGER IF EXISTS trg_test_fail_uploads_update ON uploads;
            CREATE TRIGGER trg_test_fail_uploads_update
            BEFORE UPDATE ON uploads
            FOR EACH ROW
            EXECUTE FUNCTION test_fail_uploads_update();
            ",
        )
        .execute(&mut conn)
        .await
        .expect("create update-fail trigger");

        let delete = request
            .delete(&format!("/api/v1/uploads/{filename}"))
            .add_header(auth_key, auth_value)
            .await;

        diesel::sql_query(
            r"
            DROP TRIGGER IF EXISTS trg_test_fail_uploads_update ON uploads;
            DROP FUNCTION IF EXISTS test_fail_uploads_update();
            ",
        )
        .execute(&mut conn)
        .await
        .expect("drop update-fail trigger");

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

        for i in 1..=20 {
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
