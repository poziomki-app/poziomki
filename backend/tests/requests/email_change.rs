use axum::http::{HeaderName, HeaderValue};
use axum_test::TestServer;
use serial_test::serial;
use std::future::Future;

fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let value = HeaderValue::from_str(&format!("Bearer {token}")).unwrap();
    (HeaderName::from_static("authorization"), value)
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

async fn sign_up_and_verify(server: &TestServer, email: &str, password: &str) -> String {
    let sign_up = server
        .post("/api/v1/auth/sign-up/email")
        .json(&serde_json::json!({
            "email": email,
            "name": "Test User",
            "password": password,
        }))
        .await;
    assert_eq!(sign_up.status_code(), 200);

    let otp = get_otp_code(email).await;
    let verify = server
        .post("/api/v1/auth/verify-otp")
        .json(&serde_json::json!({ "email": email, "otp": otp }))
        .await;
    assert_eq!(verify.status_code(), 200);
    let body: serde_json::Value = verify.json();
    body["data"]["token"]
        .as_str()
        .map(ToOwned::to_owned)
        .expect("verify-otp should return token")
}

#[tokio::test]
#[serial]
async fn email_change_happy_path() {
    run(|server| async move {
        let token = sign_up_and_verify(&server, "old@example.com", "secret123").await;
        let (k, v) = auth_header(&token);

        let request = server
            .patch("/api/v1/auth/account/email/request")
            .add_header(k.clone(), v.clone())
            .json(&serde_json::json!({
                "newEmail": "new@example.com",
                "currentPassword": "secret123",
            }))
            .await;
        assert_eq!(request.status_code(), 200);

        let code = get_otp_code("new@example.com").await;
        let confirm = server
            .patch("/api/v1/auth/account/email/confirm")
            .add_header(k, v)
            .json(&serde_json::json!({
                "newEmail": "new@example.com",
                "code": code,
            }))
            .await;
        assert_eq!(confirm.status_code(), 200);
        let body: serde_json::Value = confirm.json();
        assert_eq!(body["data"]["email"], "new@example.com");

        // Sign-in with new email should now work
        let sign_in = server
            .post("/api/v1/auth/sign-in/email")
            .json(&serde_json::json!({
                "email": "new@example.com",
                "password": "secret123",
            }))
            .await;
        assert_eq!(sign_in.status_code(), 200);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn email_change_rejects_wrong_password() {
    run(|server| async move {
        let token = sign_up_and_verify(&server, "user@example.com", "secret123").await;
        let (k, v) = auth_header(&token);

        let response = server
            .patch("/api/v1/auth/account/email/request")
            .add_header(k, v)
            .json(&serde_json::json!({
                "newEmail": "other@example.com",
                "currentPassword": "wrong-password",
            }))
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn email_change_rejects_taken_email() {
    run(|server| async move {
        let _other = sign_up_and_verify(&server, "taken@example.com", "secret123").await;
        let token = sign_up_and_verify(&server, "me@example.com", "secret123").await;
        let (k, v) = auth_header(&token);

        let response = server
            .patch("/api/v1/auth/account/email/request")
            .add_header(k, v)
            .json(&serde_json::json!({
                "newEmail": "taken@example.com",
                "currentPassword": "secret123",
            }))
            .await;
        assert_eq!(response.status_code(), 409);
    })
    .await;
}
