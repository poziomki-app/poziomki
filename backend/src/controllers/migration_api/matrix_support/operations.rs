use serde::Deserialize;
use serde_json::json;

use super::{
    encode_path_component, execute_matrix_empty_request, execute_matrix_json_request,
    matrix_endpoint, MatrixRequestError,
};

#[derive(Deserialize)]
struct MediaUploadResponse {
    content_uri: String,
}

#[derive(Deserialize)]
struct MatrixCreateRoomResponse {
    room_id: String,
}

pub(super) async fn set_display_name(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    user_id: &str,
    display_name: &str,
) -> std::result::Result<(), String> {
    let encoded_user_id = user_id
        .replace('%', "%25")
        .replace('@', "%40")
        .replace(':', "%3A");
    let url = matrix_endpoint(
        homeserver,
        &format!("/_matrix/client/v3/profile/{encoded_user_id}/displayname"),
    );
    let response = http_client
        .put(&url)
        .bearer_auth(access_token)
        .json(&json!({ "displayname": display_name }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}

/// Upload media to the homeserver content repository.
///
/// Uses `POST /_matrix/media/v3/upload` which is the correct, non-deprecated
/// upload endpoint per the Matrix spec (MSC3916 only moved *download/thumbnail*
/// to `/_matrix/client/v1/media/`; upload was already authenticated and stays
/// under the media namespace).
pub(super) async fn upload_media(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    bytes: Vec<u8>,
    content_type: &str,
    filename: &str,
) -> std::result::Result<String, String> {
    let url = matrix_endpoint(
        homeserver,
        &format!("/_matrix/media/v3/upload?filename={filename}"),
    );
    let response = http_client
        .post(&url)
        .bearer_auth(access_token)
        .header("Content-Type", content_type)
        .body(bytes)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    let body: MediaUploadResponse = response.json().await.map_err(|e| e.to_string())?;
    Ok(body.content_uri)
}

pub(super) async fn set_avatar_url(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    user_id: &str,
    avatar_url: &str,
) -> std::result::Result<(), String> {
    let encoded_user_id = user_id
        .replace('%', "%25")
        .replace('@', "%40")
        .replace(':', "%3A");
    let url = matrix_endpoint(
        homeserver,
        &format!("/_matrix/client/v3/profile/{encoded_user_id}/avatar_url"),
    );
    let response = http_client
        .put(&url)
        .bearer_auth(access_token)
        .json(&json!({ "avatar_url": avatar_url }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status().is_success() {
        Ok(())
    } else {
        Err(format!("HTTP {}", response.status()))
    }
}

pub(super) async fn create_private_room(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    room_name: &str,
    invited_user_ids: &[String],
    is_direct: bool,
) -> std::result::Result<String, MatrixRequestError> {
    let url = matrix_endpoint(homeserver, "/_matrix/client/v3/createRoom");
    let preset = if is_direct {
        "trusted_private_chat"
    } else {
        "private_chat"
    };
    let payload = json!({
        "name": room_name,
        "preset": preset,
        "is_direct": is_direct,
        "invite": invited_user_ids,
    });
    let body: MatrixCreateRoomResponse = execute_matrix_json_request(
        http_client
            .post(&url)
            .bearer_auth(access_token)
            .json(&payload),
    )
    .await?;
    Ok(body.room_id)
}

pub(super) async fn invite_user_to_room(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    room_id: &str,
    user_id: &str,
) -> std::result::Result<(), MatrixRequestError> {
    let encoded_room_id = encode_path_component(room_id);
    let url = matrix_endpoint(
        homeserver,
        &format!("/_matrix/client/v3/rooms/{encoded_room_id}/invite"),
    );
    execute_matrix_empty_request(
        http_client
            .post(&url)
            .bearer_auth(access_token)
            .json(&json!({ "user_id": user_id })),
    )
    .await
}

pub(super) async fn join_room(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    room_id: &str,
) -> std::result::Result<(), MatrixRequestError> {
    let encoded_room_id = encode_path_component(room_id);
    let url = matrix_endpoint(
        homeserver,
        &format!("/_matrix/client/v3/rooms/{encoded_room_id}/join"),
    );
    execute_matrix_empty_request(
        http_client
            .post(&url)
            .bearer_auth(access_token)
            .json(&json!({})),
    )
    .await
}

pub(super) async fn leave_room(
    http_client: &reqwest::Client,
    homeserver: &str,
    access_token: &str,
    room_id: &str,
) -> std::result::Result<(), MatrixRequestError> {
    let encoded_room_id = encode_path_component(room_id);
    let url = matrix_endpoint(
        homeserver,
        &format!("/_matrix/client/v3/rooms/{encoded_room_id}/leave"),
    );
    execute_matrix_empty_request(
        http_client
            .post(&url)
            .bearer_auth(access_token)
            .json(&json!({})),
    )
    .await
}

pub(super) fn content_type_from_filename(filename: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(filename)
        .extension()?
        .to_ascii_lowercase();
    match ext.to_str()? {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}
