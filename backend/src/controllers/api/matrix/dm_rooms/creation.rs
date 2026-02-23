use axum::http::HeaderMap;
use axum::response::Response;
use uuid::Uuid;

use super::super::{bootstrap_matrix_auth, matrix_support};

pub(super) async fn create_dm_room(
    headers: &HeaderMap,
    own_user_pid: Uuid,
    other_user_pid: Uuid,
) -> std::result::Result<String, Response> {
    let bootstrap = bootstrap_matrix_auth(&own_user_pid.to_string(), headers, None, None).await?;
    let server_name = matrix_support::matrix_server_name_from_user_id(&bootstrap.auth.user_id)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            super::super::chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service returned an invalid user identifier",
                "CHAT_UNAVAILABLE",
            )
        })?;
    let target_matrix_user_id =
        matrix_support::matrix_user_id_from_pid(&other_user_pid, &server_name);
    let invites = vec![target_matrix_user_id];

    bootstrap
        .client()
        .create_private_room("Wiadomość", &invites, true)
        .await
        .map_err(|error| {
            tracing::warn!(
                status_code = error.status_code,
                errcode = error.errcode,
                message = error.message,
                "failed to create Matrix DM room"
            );
            super::super::chat_bootstrap_error(
                axum::http::StatusCode::BAD_GATEWAY,
                headers,
                "Messaging service is temporarily unavailable",
                "CHAT_UNAVAILABLE",
            )
        })
}
