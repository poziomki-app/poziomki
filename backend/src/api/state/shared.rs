use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(in crate::api) enum TagScope {
    Interest,
    Activity,
    Event,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(in crate::api) enum AttendeeStatus {
    Going,
    Interested,
    Invited,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(in crate::api) enum UploadContext {
    ProfilePicture,
    ProfileGallery,
    EventCover,
    ChatCover,
    ChatAttachment,
    Bio,
}
