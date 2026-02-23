use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct SuccessResponse {
    pub(in crate::api) success: bool,
}

#[derive(Clone, Debug, Serialize)]
pub(in crate::api) struct DataResponse<T> {
    pub(in crate::api) data: T,
}
