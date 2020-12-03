use crate::types::{ResourceMapResponse, SelfLink, SingleResourceResponse};
use serde_json::Value;
use std::collections::HashMap;

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

/// <https://developer.shotgunsoftware.com/rest-api/?shell#schemaschemaentityrecord>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaEntityRecord {
    pub name: Option<SchemaResponseValue>,
    pub visible: Option<SchemaResponseValue>,
}

/// <https://developer.shotgunsoftware.com/rest-api/?shell#tocSschemaentityresponse>
pub type SchemaEntityResponse = SingleResourceResponse<SchemaEntityRecord, SelfLink>;

/// <https://developer.shotgunsoftware.com/rest-api/#tocSschemaentitiesresponse>
pub type SchemaEntitiesResponse = ResourceMapResponse<SchemaEntityRecord, SelfLink>;

/// <https://developer.shotgunsoftware.com/rest-api/?shell#schemaschemaresponsevalue>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SchemaResponseValue {
    /// Can be a string or a boolean
    pub value: Option<Value>,
    pub editable: Option<bool>,
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

impl<K, V> From<(K, V)> for CreateUpdateFieldProperty
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    fn from(pair: (K, V)) -> Self {
        Self {
            property_name: pair.0.as_ref().to_string(),
            value: pair.1.as_ref().to_string(),
        }
    }
}

impl<K, V> From<&(K, V)> for CreateUpdateFieldProperty
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    fn from(pair: &(K, V)) -> Self {
        Self {
            property_name: pair.0.as_ref().to_string(),
            value: pair.1.as_ref().to_string(),
        }
    }
}

/// <https://developer.shotgunsoftware.com/rest-api/#tocSupdatefieldrequest>
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateFieldRequest {
    pub properties: Vec<CreateUpdateFieldProperty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i32>,
}
