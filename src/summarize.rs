use crate::filters::FinalizedFilters;
use crate::{handle_response, Session};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Request body of a summarize query.
#[derive(Serialize, Debug, Clone)]
pub struct SummarizeRequest {
    /// Filters used to perform the initial search for things you will be
    /// aggregating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<FinalizedFilters>,

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

// FIXME: `Value` here should be a concrete type that is string, number, bool,
//  or object (anything but array).
//  Either that, or we can do `Value` and just advise that the thing is not
//  going to be an array...
//  The main thing we get from calling this a hashmap is we enforce the top
//  level being a map.
//  We could do some kind of recursive enum deal. Yuck.
pub type SummaryMap = HashMap<String, Value>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SummaryGroups {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<SummaryGroups>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summaries: Option<SummaryMap>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SummaryData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summaries: Option<SummaryMap>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<Vec<SummaryGroups>>,
}

/// <https://developer.shotgridsoftware.com/rest-api/#tocSsummarizeresponse>
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SummarizeResponse {
    pub data: SummaryData,
}

/// A summary field consists of a concrete field on an entity and a summary
/// operation to use to aggregate it as part of a summary request.
///
/// For convenience, `SummaryField` can be built from a pair of
/// `(AsRef<str>, SummmaryFieldType)`.
///
/// ```
/// use shotgrid_rs::types::SummaryField;
///
/// # fn main() -> shotgrid_rs::Result<()> {
/// use shotgrid_rs::types::SummaryFieldType;
/// let id_count = SummaryField::from(("id", SummaryFieldType::Count));
/// let max_due_date: SummaryField = ("due_date", SummaryFieldType::Max).into();
/// # Ok(())
/// # }
/// ```
///
/// When making a call to [`Session::summarize()`] you may want several
/// `SummaryField` instances.
/// For this you can convert a Vec of pairs into a `Vec<SummaryField>` by
/// doing something like:
///
/// ```
/// use shotgrid_rs::types::{SummaryField, SummaryFieldType};
///
/// # fn main() -> shotgrid_rs::Result<()> {
/// let summary_fields: Vec<SummaryField> = vec![
///     ("id", SummaryFieldType::Count),
///     ("due_date", SummaryFieldType::Max),
///     ("start_date", SummaryFieldType::Min)
/// ]
/// .into_iter()
/// .map(Into::into)
/// .collect();
/// # Ok(())
/// # }
/// ```
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SummaryField {
    pub field: String,
    pub r#type: SummaryFieldType,
}

impl<S> From<(S, SummaryFieldType)> for SummaryField
where
    S: AsRef<str>,
{
    fn from(pair: (S, SummaryFieldType)) -> Self {
        Self {
            field: pair.0.as_ref().into(),
            r#type: pair.1,
        }
    }
}

impl<S> From<&(S, SummaryFieldType)> for SummaryField
where
    S: AsRef<str>,
{
    fn from(pair: &(S, SummaryFieldType)) -> Self {
        Self {
            field: pair.0.as_ref().into(),
            r#type: pair.1.clone(),
        }
    }
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

/// A grouping for a summary request.
///
/// For convenience, `Grouping`s can be built from three or two element tuples
/// depending on whether or not you want to specify a "direction" for the
/// grouping.
///
/// ```
/// use shotgrid_rs::types::{Grouping, GroupingType, GroupingDirection};
///
/// // For 3 element tuples, GroupingDirection can be an "implicit" Option:
/// // `GroupingDirection::Desc` is the same as `Some(GroupingDirection::Desc)`
/// let by_due_date: Grouping = ("due_date", GroupingType::Exact, GroupingDirection::Desc).into();
/// let by_status: Grouping = ("sg_status_list", GroupingType::Exact, None).into();
///
/// // For two-element tuples, the direction is defaulted to `None`
/// let by_content_first_letter: Grouping = ("content", GroupingType::FirstLetter).into();
/// ```
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

impl<S, D> From<(S, GroupingType, D)> for Grouping
where
    S: AsRef<str>,
    D: Into<Option<GroupingDirection>>,
{
    fn from(triple: (S, GroupingType, D)) -> Self {
        Self {
            field: triple.0.as_ref().into(),
            r#type: triple.1,
            direction: triple.2.into(),
        }
    }
}

impl<S, D> From<&(S, GroupingType, D)> for Grouping
where
    S: AsRef<str>,
    D: Clone + Into<Option<GroupingDirection>>,
{
    fn from(triple: &(S, GroupingType, D)) -> Self {
        Self {
            field: triple.0.as_ref().into(),
            r#type: triple.1.clone(),
            direction: triple.2.clone().into(),
        }
    }
}

impl<S> From<(S, GroupingType)> for Grouping
where
    S: AsRef<str>,
{
    fn from(pair: (S, GroupingType)) -> Self {
        Self {
            field: pair.0.as_ref().into(),
            r#type: pair.1,
            direction: None,
        }
    }
}

impl<S> From<&(S, GroupingType)> for Grouping
where
    S: AsRef<str>,
{
    fn from(pair: &(S, GroupingType)) -> Self {
        Self {
            field: pair.0.as_ref().into(),
            r#type: pair.1.clone(),
            direction: None,
        }
    }
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

pub struct SummarizeReqBuilder<'a> {
    session: &'a Session<'a>,
    entity: &'a str,
    // FIXME: python api treats filters as required (and we fallback to empty array).
    //  Maybe just make it required?
    filters: Option<FinalizedFilters>,
    // required (but empty array is legal) despite api spec
    summary_fields: Vec<SummaryField>,
    // TODO: move these to a builder
    grouping: Option<Vec<Grouping>>,
    options: Option<SummaryOptions>,
}

impl<'a> SummarizeReqBuilder<'a> {
    pub fn new(
        session: &'a Session<'a>,
        entity: &'a str,
        filters: Option<FinalizedFilters>,
        summary_fields: Vec<SummaryField>,
    ) -> SummarizeReqBuilder<'a> {
        SummarizeReqBuilder {
            session,
            entity,
            filters,
            summary_fields,
            grouping: None,
            options: None,
        }
    }

    pub fn grouping(mut self, value: Option<Vec<Grouping>>) -> Self {
        self.grouping = value;
        self
    }

    pub fn include_archived_projects(mut self, value: Option<bool>) -> Self {
        self.options = value.map(|x| SummaryOptions {
            include_archived_projects: Some(x),
        });
        self
    }

    pub async fn execute(self) -> crate::Result<SummarizeResponse> {
        // FIXME: python api treats filters as required (and we fallback to empty array).
        //  Maybe just make it required?
        let content_type = self
            .filters
            .as_ref()
            .map(|filters| filters.get_mime())
            .unwrap_or(crate::filters::MIME_FILTER_ARRAY);

        let body = SummarizeRequest {
            filters: self.filters,
            summary_fields: Some(self.summary_fields),
            grouping: self.grouping,
            options: self.options,
        };

        let (sg, token) = self.session.get_sg().await?;

        let req = sg
            .http
            .post(&format!(
                "{}/api/v1/entity/{}/_summarize",
                sg.sg_server, self.entity
            ))
            .header("Accept", "application/json")
            .bearer_auth(token)
            .header("Content-Type", content_type)
            // The content type is being set to ShotGrid's custom mime types
            // to indicate the shape of the filter payload. Do not be tempted to
            // use `.json()` here instead of `.body()` or you'll end up
            // reverting the header set above.
            .body(json!(body).to_string());
        handle_response(req.send().await?).await
    }
}
