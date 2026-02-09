use loco_rs::testing::prelude::*;
use poziomki_backend::app::App;
use serial_test::serial;

use super::prepare_data::auth_header;

fn sign_up_json(email: &str, password: &str) -> serde_json::Value {
    serde_json::json!({
        "email": email,
        "name": "Test User",
        "password": password,
    })
}

#[tokio::test]
#[serial]
async fn health_endpoint_matches_contract() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/health").await;
        assert_eq!(response.status_code(), 200);
        response.assert_json(&serde_json::json!({ "status": "ok" }));
    })
    .await;
}

#[tokio::test]
#[serial]
async fn root_endpoint_matches_contract() {
    request::<App, _, _>(|request, _ctx| async move {
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
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/api/v1/matrix/session").await;
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
    request::<App, _, _>(|request, _ctx| async move {
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
async fn legacy_chat_http_returns_gone() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/v1/chats").await;
        assert_eq!(response.status_code(), 410);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["code"], "CHAT_MIGRATED_TO_MATRIX");
        assert_eq!(payload["details"]["migrationPath"], "/api/v1/matrix");
        assert!(payload["requestId"].is_string());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn legacy_chat_websocket_path_returns_gone() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/ws/chat").await;
        assert_eq!(response.status_code(), 410);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["code"], "CHAT_MIGRATED_TO_MATRIX");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn auth_get_session_returns_unwrapped_shape() {
    request::<App, _, _>(|request, _ctx| async move {
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
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/v1/matrix/config").await;
        assert_eq!(response.status_code(), 200);

        let payload: serde_json::Value = response.json();
        assert_eq!(payload["data"]["chat_mode"], "matrix-native");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn events_flow_matches_phase_3_contract() {
    request::<App, _, _>(|request, _ctx| async move {
        let sign_up_response = request
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "owner@example.com",
                "name": "Owner",
                "password": "secret123",
            }))
            .await;
        assert_eq!(sign_up_response.status_code(), 200);
        let sign_up_payload: serde_json::Value = sign_up_response.json();
        let owner_token = sign_up_payload["data"]["token"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        assert!(!owner_token.is_empty());

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

        let attendee_signup = request
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "attendee@example.com",
                "name": "Attendee",
                "password": "secret123",
            }))
            .await;
        assert_eq!(attendee_signup.status_code(), 200);
        let attendee_signup_payload: serde_json::Value = attendee_signup.json();
        let attendee_token = attendee_signup_payload["data"]["token"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();
        assert!(!attendee_token.is_empty());

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
    request::<App, _, _>(|request, _ctx| async move {
        let signup_a = request
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "match-a@example.com",
                "name": "A",
                "password": "secret123",
            }))
            .await;
        assert_eq!(signup_a.status_code(), 200);
        let payload_a: serde_json::Value = signup_a.json();
        let token_a = payload_a["data"]["token"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();

        let signup_b = request
            .post("/api/v1/auth/sign-up/email")
            .json(&serde_json::json!({
                "email": "match-b@example.com",
                "name": "B",
                "password": "secret123",
            }))
            .await;
        assert_eq!(signup_b.status_code(), 200);
        let payload_b: serde_json::Value = signup_b.json();
        let token_b = payload_b["data"]["token"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();

        let (auth_key_a, auth_value_a) = auth_header(&token_a);
        let (auth_key_b, auth_value_b) = auth_header(&token_b);

        let profile_a = request
            .post("/api/v1/profiles")
            .add_header(auth_key_a.clone(), auth_value_a.clone())
            .json(&serde_json::json!({ "name": "A", "age": 21 }))
            .await;
        assert_eq!(profile_a.status_code(), 201);
        let profile_b = request
            .post("/api/v1/profiles")
            .add_header(auth_key_b.clone(), auth_value_b.clone())
            .json(&serde_json::json!({ "name": "B", "age": 22 }))
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

        let missing_upload_response = request.get("/api/v1/uploads/missing.png").await;
        assert_eq!(missing_upload_response.status_code(), 404);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn sign_in_verifies_hashed_password() {
    request::<App, _, _>(|request, _ctx| async move {
        let sign_up = request
            .post("/api/v1/auth/sign-up/email")
            .json(&sign_up_json("hash-test@example.com", "correct-password"))
            .await;
        assert_eq!(sign_up.status_code(), 200);

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
async fn delete_account_verifies_hashed_password() {
    request::<App, _, _>(|request, _ctx| async move {
        let sign_up = request
            .post("/api/v1/auth/sign-up/email")
            .json(&sign_up_json("delete-test@example.com", "my-password"))
            .await;
        assert_eq!(sign_up.status_code(), 200);
        let payload: serde_json::Value = sign_up.json();
        let token = payload["data"]["token"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_default();

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
