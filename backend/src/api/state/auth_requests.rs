use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct SignUpBody {
    pub(in crate::api) email: String,
    pub(in crate::api) name: String,
    pub(in crate::api) password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct SignInBody {
    pub(in crate::api) email: String,
    pub(in crate::api) password: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct VerifyOtpBody {
    pub(in crate::api) email: String,
    pub(in crate::api) otp: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct ResendOtpBody {
    pub(in crate::api) email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct DeleteAccountBody {
    pub(in crate::api) password: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct ChangePasswordBody {
    pub(in crate::api) current_password: String,
    pub(in crate::api) new_password: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct ForgotPasswordBody {
    pub(in crate::api) email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(in crate::api) struct ForgotPasswordVerifyBody {
    pub(in crate::api) email: String,
    pub(in crate::api) otp: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(in crate::api) struct ResetPasswordBody {
    pub(in crate::api) email: String,
    pub(in crate::api) reset_token: String,
    pub(in crate::api) new_password: String,
}
