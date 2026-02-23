use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct SessionResponse {
    pub(in crate::api) session: Option<SessionView>,
    pub(in crate::api) user: Option<UserView>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct SessionView {
    pub(in crate::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::api) user_id: String,
    #[serde(rename = "expiresAt")]
    pub(in crate::api) expires_at: String,
    #[serde(rename = "createdAt")]
    pub(in crate::api) created_at: String,
    #[serde(rename = "updatedAt")]
    pub(in crate::api) updated_at: String,
    #[serde(rename = "ipAddress")]
    pub(in crate::api) ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub(in crate::api) user_agent: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct UserView {
    pub(in crate::api) id: String,
    pub(in crate::api) email: String,
    pub(in crate::api) name: String,
    #[serde(rename = "emailVerified")]
    pub(in crate::api) email_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct SessionListItem {
    pub(in crate::api) id: String,
    #[serde(rename = "userId")]
    pub(in crate::api) user_id: String,
    #[serde(rename = "expiresAt")]
    pub(in crate::api) expires_at: String,
    #[serde(rename = "createdAt")]
    pub(in crate::api) created_at: String,
    #[serde(rename = "ipAddress")]
    pub(in crate::api) ip_address: Option<String>,
    #[serde(rename = "userAgent")]
    pub(in crate::api) user_agent: Option<String>,
}
