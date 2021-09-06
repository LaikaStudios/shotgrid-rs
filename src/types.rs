pub use crate::schema::{
    CreateFieldRequest, CreateUpdateFieldProperty, FieldDataType, SchemaEntitiesResponse,
    SchemaEntityRecord, SchemaEntityResponse, SchemaFieldProperties, SchemaFieldRecord,
    SchemaFieldResponse, SchemaFieldsResponse, SchemaResponseValue, UpdateFieldRequest,
};
pub use crate::summarize::{
    Grouping, GroupingDirection, GroupingType, SummarizeRequest, SummarizeResponse, SummaryData,
    SummaryField, SummaryFieldType, SummaryMap, SummaryOptions,
};
use serde_json::Value;
use std::collections::HashMap;

/// <https://developer.shotgridsoftware.com/rest-api/#tocSactivityupdate>
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ActivityUpdate {
    id: Option<i32>,
    update_type: Option<String>,
    meta: Option<serde_json::Map<String, Value>>,
    read: Option<bool>,
    primary_entity: Option<serde_json::Map<String, Value>>,
    created_by: Option<serde_json::Map<String, Value>>,
}

/// Alternate images
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum AltImages {
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "thumbnail")]
    Thumbnail,
}

/// <https://developer.shotgridsoftware.com/rest-api/?shell#tocSbatchcreateoptionsparameter>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchCreateOptionsParameter {
    pub options: Option<serde_json::Map<String, Value>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSbatchedrequestsresponse>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchedRequestsResponse {
    pub data: Option<Vec<Record>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSclientcredentialsrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClientCredentialsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
}

impl Default for ClientCredentialsRequest {
    fn default() -> Self {
        Self {
            grant_type: Some(String::from("client_credentials")),
            client_id: None,
            client_secret: None,
        }
    }
}

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

/// EntityActivityStreamData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityActivityStreamData {
    pub entity_id: Option<i32>,
    pub entity_type: Option<String>,
    pub latest_update_id: Option<i32>,
    pub earliest_update_id: Option<i32>,
    pub updates: Option<Vec<ActivityUpdate>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSentityactivitystreamresponse>
pub type EntityActivityStreamResponse = SingleResourceResponse<EntityActivityStreamData, SelfLink>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityIdentifier {
    pub record_id: Option<i32>,
    pub entity: Option<String>,
}

/// EntityThreadContentsData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityThreadContentsData {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<String>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSentitythreadcontentsresponse>
pub type EntityThreadContentsResponse = SingleResourceResponse<EntityThreadContentsData, SelfLink>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    pub errors: Vec<ErrorObject>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ErrorObject {
    pub id: Option<String>,
    pub status: Option<i64>,
    pub code: Option<i64>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub source: Option<serde_json::Map<String, Value>>,
    pub meta: Option<serde_json::Map<String, Value>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/?shell#tocSfieldhashresponse>
pub type FieldHashResponse = SingleResourceResponse<Value, SelfLink>;

/// <https://developer.shotgridsoftware.com/rest-api/?shell#tocSfilterhash>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FilterHash {
    pub logical_operator: Option<LogicalOperator>,
    // Either an array or a hash
    pub conditions: Option<Value>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSfollowerrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowerRecord {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub attributes: Option<serde_json::Map<String, Value>>,
    pub links: Option<SelfLink>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSfollowrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowRecord {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub links: Option<SelfLink>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSgetworkdayrulesresponse>
pub type GetWorkDayRulesResponse = ResourceArrayResponse<WorkDayRulesData, SelfLink>;

/// HierarchyEntityFields is not represented as a named schema in the ShotGrid OpenAPI Spec.
// FIXME: the spec indicates `entity` and `fields` are optional, but if you send
//  a `HierarchyEntityFields` to the server without either of them, you'll get a
//  400 response.
//  Likely the spec is wrong and they just mean the outer object is optional.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyEntityFields {
    pub entity: Option<String>,
    pub fields: Option<Vec<String>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocShierarchyexpandrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_fields: Option<Vec<HierarchyEntityFields>>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_entity_field: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponseDataRefValue {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponseDataRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<HierarchyExpandResponseDataRefValue>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponseDataTargetEntitiesAdditionalFilterPresetSeed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponseDataTargetEntitiesAdditionalFilterPreset {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<HierarchyExpandResponseDataTargetEntitiesAdditionalFilterPresetSeed>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponseDataTargetEntities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_filter_presets:
        Option<Vec<HierarchyExpandResponseDataTargetEntitiesAdditionalFilterPreset>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponseData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#ref: Option<HierarchyExpandResponseDataRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_entities: Option<HierarchyExpandResponseDataTargetEntities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_children: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<HierarchyExpandResponseData>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocShierarchyexpandresponse>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandResponse {
    pub data: Option<HierarchyExpandResponseData>,
}

/// HierarchyReferenceEntity is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyReferenceEntity {
    pub id: Option<i32>,
    pub r#type: Option<String>,
}

/// What to search the hierarchy by.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum HierarchySearchCriteria {
    #[serde(rename = "search_string")]
    SearchString(String),
    #[serde(rename = "entity")]
    Entity(Entity),
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocShierarchysearchrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchySearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    pub search_criteria: HierarchySearchCriteria,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_entity_field: Option<String>,
}

/// HierarchySearchResponseData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchySearchResponseData {
    pub label: Option<String>,
    pub incremental_path: Option<Vec<String>>,
    pub path_label: Option<String>,
    pub r#ref: Option<HierarchyReferenceEntity>,
    pub project_id: Option<i32>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocShierarchysearchresponse>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchySearchResponse {
    pub data: Option<Vec<HierarchySearchResponseData>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

/// MultipleResourceResponse is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceArrayResponse<R, L> {
    /// Resource data
    pub data: Option<Vec<R>>,
    /// Related resource links
    pub links: Option<L>,
}

/// Resources stored in a string-keyed map.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ResourceMapResponse<R, L> {
    /// Resource data
    pub data: Option<HashMap<String, R>>,
    /// Related resource links
    pub links: Option<L>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSoptionsparameter>
#[derive(Clone, Debug, Default, Serialize)]
pub struct OptionsParameter {
    pub return_only: Option<ReturnOnly>,
    pub include_archived_projects: Option<bool>,
}

/// This controls the paging of search-style list API calls.
/// <https://developer.shotgridsoftware.com/rest-api/#tocSpaginationparameter>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PaginationParameter {
    ///  Pages start at 1, not 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<usize>,
    /// ShotGrid's default currently is 500
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

impl Default for PaginationParameter {
    fn default() -> Self {
        Self {
            number: Some(1),
            size: None,
        }
    }
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSpaginationlinks>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PaginationLinks {
    // Has to rename because we can't do raw self
    #[serde(rename = "self")]
    pub self_link: Option<String>,
    pub next: Option<String>,
    pub prev: Option<String>,
}

pub type PaginatedRecordResponse = ResourceArrayResponse<Record, PaginationLinks>;

/// <https://developer.shotgridsoftware.com/rest-api/#tocSpasswordrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PasswordRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

impl Default for PasswordRequest {
    fn default() -> Self {
        Self {
            grant_type: Some(String::from("password")),
            username: None,
            password: None,
        }
    }
}

/// This does not exist as a part of ShotGrid's REST API
pub type ProjectAccessUpdateResponse = SingleResourceResponse<Entity, SelfLink>;

/// <https://developer.shotgridsoftware.com/rest-api/#tocSrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub attributes: Option<serde_json::Map<String, Value>>,
    pub relationships: Option<serde_json::Map<String, Value>>,
    pub links: Option<SelfLink>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSrefreshrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefreshRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

/// <https://developer.shotgridsoftware.com/rest-api/?shell#tocSrelationshipsresponse>
/// The value is either a Record or a vec of records
pub type RelationshipsResponse = SingleResourceResponse<Value, SelfLink>;

#[derive(Clone, Debug, Serialize)]
pub enum ReturnOnly {
    Active,
    Retired,
}

/// <https://developer.shotgridsoftware.com/rest-api/?shell#tocSsearchrequest>
#[derive(Clone, Debug, Serialize)]
pub struct SearchRequest {
    /// Either an array of arrays or a FilterHash
    pub filters: Option<crate::filters::FinalizedFilters>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSselflink>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelfLink {
    #[serde(rename = "self")]
    pub self_link: Option<String>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSsinglerecordresponse>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SingleRecordResponse {
    pub data: Option<Record>,
    pub links: Option<SelfLink>,
}

/// Unlike SingleRecordResponse, this is not part of ShotGrid's REST API.
/// This is a generic.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SingleResourceResponse<R, L> {
    /// Resource data
    pub data: Option<R>,
    /// Related resource links
    pub links: Option<L>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocStextsearchrequest>
#[derive(Serialize, Debug, Clone)]
pub struct TextSearchRequest {
    pub entity_types: HashMap<String, crate::filters::FinalizedFilters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<PaginationParameter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSupdateworkdayrulesrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateWorkDayRulesRequest {
    pub date: String,
    pub working: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recalculate_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// UpdateWorkDayRulesData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateWorkDayRulesData {
    pub date: Option<String>,
    pub working: Option<bool>,
    pub description: Option<String>,
    pub reason: Option<String>,
}

/// <https://developer.shotgridsoftware.com/rest-api/?shell#tocSupdateworkdayrulesresponse>
pub type UpdateWorkDayRulesResponse = SingleResourceResponse<UpdateWorkDayRulesData, SelfLink>;

/// UploadInfoData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadInfoData {
    pub timestamp: Option<String>,
    pub upload_type: Option<String>,
    pub upload_id: Option<String>,
    pub storage_service: Option<String>,
    pub original_filename: Option<String>,
    pub multipart_upload: Option<bool>,
}

/// UploadInfoLinks is not represented as a named schema in the ShotGrid OpenAPI Spec.
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

/// <https://developer.shotgridsoftware.com/rest-api/#tocSuploadinforesponse>
pub type UploadInfoResponse = SingleResourceResponse<UploadInfoData, UploadInfoLinks>;

/// UploadResponseData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadResponseData {
    pub upload_id: Option<String>,
    pub original_filename: Option<String>,
}

/// UploadResponseLinks is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UploadResponseLinks {
    pub complete_upload: Option<String>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSuploadresponse>
pub type UploadResponse = SingleResourceResponse<UploadResponseData, UploadResponseLinks>;

/// WorkDayRulesData is not represented as a named schema in the ShotGrid OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkDayRulesData {
    pub date: Option<String>,
    pub working: Option<bool>,
    pub description: Option<String>,
    pub reason: Option<String>,
}
