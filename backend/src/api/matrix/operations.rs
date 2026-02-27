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

/// Authenticated Matrix API client that bundles connection credentials.
pub(in crate::api) struct MatrixClient<'a> {
    pub(in crate::api) http_client: &'a reqwest::Client,
    pub(in crate::api) homeserver: &'a str,
    pub(in crate::api) access_token: &'a str,
}

impl<'a> MatrixClient<'a> {
    pub(in crate::api) const fn new(
        http_client: &'a reqwest::Client,
        homeserver: &'a str,
        access_token: &'a str,
    ) -> Self {
        Self {
            http_client,
            homeserver,
            access_token,
        }
    }

    pub(in crate::api) async fn set_display_name(
        &self,
        user_id: &str,
        display_name: &str,
    ) -> std::result::Result<(), String> {
        let encoded_user_id = encode_matrix_user_id(user_id);
        let url = matrix_endpoint(
            self.homeserver,
            &format!("/_matrix/client/v3/profile/{encoded_user_id}/displayname"),
        );
        let response = self
            .http_client
            .put(&url)
            .bearer_auth(self.access_token)
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
    pub(in crate::api) async fn upload_media(
        &self,
        bytes: Vec<u8>,
        content_type: &str,
        filename: &str,
    ) -> std::result::Result<String, String> {
        let url = matrix_endpoint(
            self.homeserver,
            &format!("/_matrix/media/v3/upload?filename={filename}"),
        );
        let response = self
            .http_client
            .post(&url)
            .bearer_auth(self.access_token)
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

    pub(in crate::api) async fn set_avatar_url(
        &self,
        user_id: &str,
        avatar_url: &str,
    ) -> std::result::Result<(), String> {
        let encoded_user_id = encode_matrix_user_id(user_id);
        let url = matrix_endpoint(
            self.homeserver,
            &format!("/_matrix/client/v3/profile/{encoded_user_id}/avatar_url"),
        );
        let response = self
            .http_client
            .put(&url)
            .bearer_auth(self.access_token)
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

    pub(in crate::api) async fn create_private_room(
        &self,
        room_name: Option<&str>,
        invited_user_ids: &[String],
        is_direct: bool,
    ) -> std::result::Result<String, MatrixRequestError> {
        let url = matrix_endpoint(self.homeserver, "/_matrix/client/v3/createRoom");
        let preset = if is_direct {
            "trusted_private_chat"
        } else {
            "private_chat"
        };
        let mut payload = json!({
            "preset": preset,
            "is_direct": is_direct,
            "invite": invited_user_ids,
        });
        if let Some(name) = room_name {
            payload["name"] = json!(name);
        }
        let body: MatrixCreateRoomResponse = execute_matrix_json_request(
            self.http_client
                .post(&url)
                .bearer_auth(self.access_token)
                .json(&payload),
        )
        .await?;
        Ok(body.room_id)
    }

    pub(in crate::api) async fn invite_user_to_room(
        &self,
        room_id: &str,
        user_id: &str,
    ) -> std::result::Result<(), MatrixRequestError> {
        let encoded_room_id = encode_path_component(room_id);
        let url = matrix_endpoint(
            self.homeserver,
            &format!("/_matrix/client/v3/rooms/{encoded_room_id}/invite"),
        );
        execute_matrix_empty_request(
            self.http_client
                .post(&url)
                .bearer_auth(self.access_token)
                .json(&json!({ "user_id": user_id })),
        )
        .await
    }

    pub(in crate::api) async fn join_room(
        &self,
        room_id: &str,
    ) -> std::result::Result<(), MatrixRequestError> {
        let encoded_room_id = encode_path_component(room_id);
        let url = matrix_endpoint(
            self.homeserver,
            &format!("/_matrix/client/v3/rooms/{encoded_room_id}/join"),
        );
        execute_matrix_empty_request(
            self.http_client
                .post(&url)
                .bearer_auth(self.access_token)
                .json(&json!({})),
        )
        .await
    }

    pub(in crate::api) async fn leave_room(
        &self,
        room_id: &str,
    ) -> std::result::Result<(), MatrixRequestError> {
        let encoded_room_id = encode_path_component(room_id);
        let url = matrix_endpoint(
            self.homeserver,
            &format!("/_matrix/client/v3/rooms/{encoded_room_id}/leave"),
        );
        execute_matrix_empty_request(
            self.http_client
                .post(&url)
                .bearer_auth(self.access_token)
                .json(&json!({})),
        )
        .await
    }
}

pub(super) fn content_type_from_filename(filename: &str) -> Option<&'static str> {
    let ext = std::path::Path::new(filename)
        .extension()?
        .to_ascii_lowercase();
    match ext.to_str()? {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn encode_matrix_user_id(user_id: &str) -> String {
    user_id
        .replace('%', "%25")
        .replace('@', "%40")
        .replace(':', "%3A")
}
