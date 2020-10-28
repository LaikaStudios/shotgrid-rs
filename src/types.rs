use serde_json::Value;
use std::collections::HashMap;

/// <https://developer.shotgunsoftware.com/rest-api/#tocSactivityupdate>
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

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSbatchcreateoptionsparameter>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchCreateOptionsParameter {
    pub options: Option<serde_json::Map<String, Value>>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSbatchedrequestsresponse>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BatchedRequestsResponse {
    pub data: Option<Vec<Record>>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSclientcredentialsrequest>
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

/// <https://developer.shotgunsoftware.com/rest-api/#tocScreatefieldrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateFieldRequest {
    pub data_type: FieldDataType,
    pub properties: Vec<CreateUpdateFieldProperty>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocScreateupdatefieldproperty>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CreateUpdateFieldProperty {
    pub property_name: String,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Entity {
    pub id: i32,
    pub r#type: String,
}

/// EntityActivityStreamData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityActivityStreamData {
    pub entity_id: Option<i32>,
    pub entity_type: Option<String>,
    pub latest_update_id: Option<i32>,
    pub earliest_update_id: Option<i32>,
    pub updates: Option<Vec<ActivityUpdate>>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSentityactivitystreamresponse>
pub type EntityActivityStreamResponse = SingleResourceResponse<EntityActivityStreamData, SelfLink>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityIdentifier {
    pub record_id: Option<i32>,
    pub entity: Option<String>,
}

/// EntityThreadContentsData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct EntityThreadContentsData {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub content: Option<String>,
    pub created_at: Option<String>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSentitythreadcontentsresponse>
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

/// How to perform the grouping for a given summary request.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum FieldDataType {
    #[serde(rename = "checkbox")]
    Checkbox,
    #[serde(rename = "currency")]
    Currency,
    #[serde(rename = "date")]
    Date,
    #[serde(rename = "date_time")]
    DateTime,
    #[serde(rename = "duration")]
    Duration,
    #[serde(rename = "entity")]
    Entity,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "int")]
    Int,
    #[serde(rename = "list")]
    List,
    #[serde(rename = "multi_entity")]
    MultiEntity,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "percent")]
    Percent,
    #[serde(rename = "status_list")]
    StatusList,
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "timecode")]
    Timecode,
    #[serde(rename = "footage")]
    Footage,
    #[serde(rename = "url")]
    URL,
    #[serde(rename = "uuid")]
    UUID,
    #[serde(rename = "calculated")]
    Calculated,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSfieldhashresponse>
pub type FieldHashResponse = SingleResourceResponse<Value, SelfLink>;

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSfilterhash>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FilterHash {
    pub logical_operator: Option<LogicalOperator>,
    // Either an array or a hash
    pub conditions: Option<Value>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSfollowerrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowerRecord {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub attributes: Option<serde_json::Map<String, Value>>,
    pub links: Option<SelfLink>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSfollowrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FollowRecord {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub links: Option<SelfLink>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSgetworkdayrulesresponse>
pub type GetWorkDayRulesResponse = MultipleResourceResponse<WorkDayRulesData, SelfLink>;

/// A grouping for a summary request.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Grouping {
    /// The field to group by.
    pub field: String,
    /// The aggregate operation to use to derive the grouping.
    pub r#type: GroupingType,
    /// The direction to order the grouping (ASC or DESC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<GroupingDirection>,
}

/// Direction to order a summary grouping.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GroupingDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

/// How to perform the grouping for a given summary request.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GroupingType {
    #[serde(rename = "exact")]
    Exact,
    #[serde(rename = "tens")]
    Tens,
    #[serde(rename = "hundreds")]
    Hundreds,
    #[serde(rename = "thousands")]
    Thousands,
    #[serde(rename = "tensofthousands")]
    TensOfThousands,
    #[serde(rename = "hundredsofthousands")]
    HundredsOfThousands,
    #[serde(rename = "millions")]
    Millions,
    #[serde(rename = "day")]
    Day,
    #[serde(rename = "week")]
    Week,
    #[serde(rename = "month")]
    Month,
    #[serde(rename = "quarter")]
    Quarter,
    #[serde(rename = "year")]
    Year,
    #[serde(rename = "clustered_date")]
    ClusteredDate,
    #[serde(rename = "oneday")]
    OneDay,
    #[serde(rename = "fivedays")]
    FiveDays,
    #[serde(rename = "entitytype")]
    EntityType,
    #[serde(rename = "firstletter")]
    FirstLetter,
}

/// HierarchyEntityFields is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyEntityFields {
    pub entity: Option<String>,
    pub fields: Option<Vec<String>>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocShierarchyexpandrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyExpandRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_fields: Option<Vec<HierarchyEntityFields>>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_entity_field: Option<String>,
}

/// HierarchyReferenceEntity is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchyReferenceEntity {
    pub id: Option<i32>,
    pub r#type: Option<String>,
}

/// HierarchySearchCriteria is not represented as a named schema in the Shotgun OpenAPI Spec.
/// This option MUST contain only ONE of the properties, as per the
/// note in HierarchySearchRequest in Shotgun's docs.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchySearchCriteria {
    pub search_string: Option<String>,
    pub entity: Option<Entity>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocShierarchysearchrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchySearchRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_path: Option<String>,
    pub search_criteria: HierarchySearchCriteria,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_entity_field: Option<String>,
}

/// HierarchySearchResponseData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HierarchySearchResponseData {
    pub label: Option<String>,
    pub incremental_path: Option<Vec<String>>,
    pub path_label: Option<String>,
    pub r#ref: Option<HierarchyReferenceEntity>,
    pub project_id: Option<i32>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocShierarchysearchresponse>
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

/// MultipleResourceResponse is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MultipleResourceResponse<R, L> {
    /// Resource data
    pub data: Option<Vec<R>>,
    /// Related resource links
    pub links: Option<L>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSoptionsparameter>
#[derive(Clone, Debug, Serialize)]
pub struct OptionsParameter {
    pub return_only: Option<ReturnOnly>,
    pub include_archived_projects: Option<bool>,
}

impl Default for OptionsParameter {
    fn default() -> Self {
        Self {
            return_only: None,
            include_archived_projects: None,
        }
    }
}

/// This controls the paging of search-style list API calls.
/// <https://developer.shotgunsoftware.com/rest-api/#tocSpaginationparameter>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PaginationParameter {
    ///  Pages start at 1, not 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<usize>,
    /// Shotgun's default currently is 500
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

/// <https://developer.shotgunsoftware.com/rest-api/#tocSpaginationlinks>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PaginationLinks {
    // Has to rename because we can't do raw self
    #[serde(rename = "self")]
    pub self_link: Option<String>,
    pub next: Option<String>,
    pub prev: Option<String>,
}

pub type PaginatedRecordResponse = MultipleResourceResponse<Record, PaginationLinks>;

/// <https://developer.shotgunsoftware.com/rest-api/#tocSpasswordrequest>
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

/// This does not exist as a part of Shotgun's REST API
pub type ProjectAccessUpdateResponse = SingleResourceResponse<Entity, SelfLink>;

/// <https://developer.shotgunsoftware.com/rest-api/#tocSrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Record {
    pub id: Option<i32>,
    pub r#type: Option<String>,
    pub attributes: Option<serde_json::Map<String, Value>>,
    pub relationships: Option<serde_json::Map<String, Value>>,
    pub links: Option<SelfLink>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSrefreshrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RefreshRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSrelationshipsresponse>
/// The value is either a Record or a vec of records
pub type RelationshipsResponse = SingleResourceResponse<Value, SelfLink>;

#[derive(Clone, Debug, Serialize)]
pub enum ReturnOnly {
    Active,
    Retired,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#schemaschemaentityrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaEntityRecord {
    pub name: Option<SchemaResponseValue>,
    pub visible: Option<SchemaResponseValue>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSschemaentityresponse>
pub type SchemaEntityResponse = SingleResourceResponse<SchemaEntityRecord, SelfLink>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaFieldProperties {
    pub default_value: Option<SchemaResponseValue>,
    pub regex_validation: Option<SchemaResponseValue>,
    pub regex_validation_enabled: Option<SchemaResponseValue>,
    pub summary_default: Option<SchemaResponseValue>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSschemafieldrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaFieldRecord {
    pub custom_metadata: Option<SchemaResponseValue>,
    pub data_type: Option<SchemaResponseValue>,
    pub description: Option<SchemaResponseValue>,
    pub editable: Option<SchemaResponseValue>,
    pub entity_type: Option<SchemaResponseValue>,
    pub mandatory: Option<SchemaResponseValue>,
    pub name: Option<SchemaResponseValue>,
    pub properties: Option<SchemaFieldProperties>,
    pub ui_value_displayable: Option<SchemaResponseValue>,
    pub unique: Option<SchemaResponseValue>,
    pub visible: Option<SchemaResponseValue>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSschemafieldresponse>
pub type SchemaFieldResponse = SingleResourceResponse<SchemaFieldRecord, SelfLink>;

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSschemafieldsresponse>
pub type SchemaFieldsResponse =
    SingleResourceResponse<HashMap<String, SchemaFieldRecord>, SelfLink>;

/// <https://developer.shotgunsoftware.com/rest-api/?shell#schemaschemaresponsevalue>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaResponseValue {
    /// Can be a string or a boolean
    pub value: Option<Value>,
    pub editable: Option<bool>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSsearchrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SearchRequest {
    /// Either an array of arrays or a FilterHash
    pub filters: Option<Value>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSselflink>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SelfLink {
    #[serde(rename = "self")]
    pub self_link: Option<String>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSsinglerecordresponse>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SingleRecordResponse {
    pub data: Option<Record>,
    pub links: Option<SelfLink>,
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

/// Request body of a summarize query.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SummarizeRequest {
    /// Filters used to perform the initial search for things you will be
    /// aggregating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<Value>,

    /// Summary fields represent the calculated values produced per
    /// grouping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_fields: Option<Vec<SummaryField>>,

    /// Groupings for aggregate operations. These are what you are
    /// _aggregating by_.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grouping: Option<Vec<Grouping>>,

    /// Options for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<SummaryOptions>,
}

/// A summary field consists of a concrete field on an entity and a summary
/// operation to use to aggregate it as part of a summary request.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SummaryField {
    pub field: String,
    pub r#type: SummaryFieldType,
}

/// The type of calculation to summarize.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum SummaryFieldType {
    #[serde(rename = "record_count")]
    RecordCount,
    #[serde(rename = "count")]
    Count,
    #[serde(rename = "sum")]
    Sum,
    #[serde(rename = "maximum")]
    Max,
    #[serde(rename = "minimum")]
    Min,
    #[serde(rename = "average")]
    Avg,
    #[serde(rename = "earliest")]
    Earliest,
    #[serde(rename = "latest")]
    Latest,
    #[serde(rename = "percentage")]
    Percentage,
    #[serde(rename = "status_percentage")]
    StatusPercentage,
    #[serde(rename = "status_list")]
    StatusList,
    #[serde(rename = "checked")]
    Checked,
    #[serde(rename = "unchecked")]
    Unchecked,
}

/// Options for a summary request.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SummaryOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_archived_projects: Option<bool>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSupdatefieldrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateFieldRequest {
    pub properties: Vec<CreateUpdateFieldProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i32>,
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSupdateworkdayrulesrequest>
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

/// UpdateWorkDayRulesData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateWorkDayRulesData {
    pub date: Option<String>,
    pub working: Option<bool>,
    pub description: Option<String>,
    pub reason: Option<String>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSupdateworkdayrulesresponse>
pub type UpdateWorkDayRulesResponse = SingleResourceResponse<UpdateWorkDayRulesData, SelfLink>;

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

/// WorkDayRulesData is not represented as a named schema in the Shotgun OpenAPI Spec.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkDayRulesData {
    pub date: Option<String>,
    pub working: Option<bool>,
    pub description: Option<String>,
    pub reason: Option<String>,
}
