/// UploadInfoData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadInfoData {
    pub timestamp: Option<String>,
    pub upload_type: Option<String>,
    pub upload_id: Option<String>,
    pub storage_service: Option<String>,
    pub original_filename: Option<String>,
    pub multipart_upload: Option<bool>,
}

/// UploadInfoLinks is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadInfoLinks {
    pub upload: Option<String>,
    pub complete_upload: Option<String>,
    pub get_next_part: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NextUploadPartLinks {
    pub upload: Option<String>,
    pub get_next_part: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct NextUploadPartResponse {
    pub links: Option<NextUploadPartLinks>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSuploadinforesponse>
pub type UploadInfoResponse = SingleResourceResponse<UploadInfoData, UploadInfoLinks>;

/// UploadResponseData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadResponseData {
    pub upload_id: Option<String>,
    pub original_filename: Option<String>,
}

/// UploadResponseLinks is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadResponseLinks {
    pub complete_upload: Option<String>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSuploadresponse>
pub type UploadResponse = SingleResourceResponse<UploadResponseData, UploadResponseLinks>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entity {
    pub id: i32,
    pub r#type: String,
}

impl Entity {
    pub fn new<S: Into<String>>(r#type: S, id: i32) -> Entity {
        Entity {
            id,
            r#type: r#type.into(),
        }
    }
}

/// Unlike SingleRecordResponse, this is not part of Shotgun's REST API.
/// This is a generic.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SingleResourceResponse<R, L> {
    /// Resource data
    pub data: Option<R>,
    /// Related resource links
    pub links: Option<L>,
}
