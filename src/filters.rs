//! A *somewhat type-safe* filtering API.
//!
//! Filters come in two flavors: [`basic()`] and [`complex()`].
//! Each of these functions will ultimately produce a [`FinalizedFilters`], which
//! can be then handed off to a query method.
//!
//! Filters play a role in a number of places in the Shotgun API including:
//!
//! - [`Session::search()`](`crate::session::Session::search()`)
//! - [`Session::summarize()`](`crate::session::Session::summarize()`)
//! - [`Session::text_search()`](`crate::session::Session::text_search()`)
//!
//! # Building Filter Conditions
//!
//! *Individual filter conditions* are created via the [`field()`] function.
//! This is the API for defining the rules or predicates that will go into
//! either the [`basic()`] or [`complex()`] functions.
//!
//! > For a full list of the various filter conditions available see the docs
//! > for [`Field`] (the return of the [`field()`] function).
//!
//! ```
//! use shotgun_rs::filters::{field, EntityRef};
//!
//! let project = field("project").is(EntityRef::new("Project", 123));
//! let created_by = field("created_by.HumanUser.id").in_(&[456, 789]);
//! ```
//!
//! Many of the various methods for filter conditions can accept a range of
//! value types.
//!
//! This is achieved by accepting types that are [`Into<FieldValue>`](`FieldValue`),
//! for which a number of `From` impls have been provided. "Scalar types" such
//! as the various ints, floats, and strings as well as [`EntityRef`].
//!
//! ## The Problem with `None`
//!
//! Any place where a [`FieldValue`] is accepted, it's expected you'll also be
//! able to pass a `None`, per the Shotgun Python API's conventions.
//!
//! In Rust, this can cause *type inference problems* in cases where you want to
//! write filters in the same style as you might with the Python API:
//!
//! ```compile_fail
//! use shotgun_rs::filters::field;
//!
//! field("due_date").is(None); // Won't work!
//! field("entity.Asset.id").in_(&[1, 2, 3, None]); // Also won't work!
//! ```
//!
//! The compiler output in these cases will look like:
//!
//! ```text
//! error[E0282]: type annotations needed
//!  --> src/filters.rs:47:22
//!   |
//! 6 | field("due_date").is(None); // Won't work!
//!   |                      ^^^^ cannot infer type for type parameter `T` declared on the enum `Option`
//!
//! error[E0308]: mismatched types
//! --> src/filters.rs:48:41
//!   |
//! 7 | field("entity.Asset.id").in_(&[1, 2, 3, None]); // Also won't work!
//!   |                                         ^^^^ expected integer, found enum `Option`
//!   |
//! = note: expected type `{integer}`
//! found enum `Option<_>`
//! ```
//!
//! In the first case, the issue is that `None` in Rust is not a free-standing
//! value like it is in Python. Instead, it's one of two variants of the
//! [`Option`](`std::option`) enum.
//!
//! `Option` is defined as:
//!
//! ```
//! pub enum Option<T> {
//!     None,
//!     Some(T),
//! }
//! ```
//!
//! The generic type `T` essentially means an `Option` is only an `Option` in
//! terms of *some other type*. Even if you're only interested in the `None`
//! part, the type for `T` must be known to the compiler.
//!
//! In order to pass a `None` into one of the filter methods, there are a couple
//! workarounds.
//!
//! You can either give the `Option` an inner type (any of the types
//! that can convert to [`FieldValue`] will work):
//!
//! ```
//! use shotgun_rs::filters::field;
//!
//! // Arbitrarily say the Option is T=&str.
//! field("due_date").is(Option::<&str>::None);
//! ```
//!
//! The other way is to use [`FieldValue::None`], which *is a free-standing*
//! value, *not framed in terms of some other type* (like it is in Python).
//!
//! ```
//! use shotgun_rs::filters::{field, FieldValue};
//!
//! field("due_date").is(FieldValue::None);
//! ```
//!
//! For the problem with the 2nd case, while using [`Field::in_()`], things might
//! be a little more conventional.
//!
//! While `&[1, 2, 3, None]` won't work (since the collection is *a mix of ints
//! and also there's an Option in there*), you can rewrite in terms of `Option<i32>`:
//!
//! ```
//! use shotgun_rs::filters::field;
//!
//! // All items are Option<i32>, so it works!
//! field("entity.Asset.id").in_(&[Some(1), Some(2), Some(3), None]);
//! ```
//!
//! # Basic Filters
//!
//! Basic filters are created with [`basic()`] and are comprised of a *list of
//! filters* created using the [`field()`] function.
//!
//! ```
//! use shotgun_rs::filters::{self, field, EntityRef};
//!
//! let task_filters = filters::basic(&[
//!     field("due_date").in_calendar_month(0),
//!     field("entity").is(EntityRef::new("Asset", 1234)),
//!     field("project.Project.id").in_(&[1, 2, 3]),
//!     field("sg_status_list").is_not("apr"),
//! ]);
//! ```
//!
//! For records to match the above filters, they must satisfy *all the conditions*
//! in the list:
//!
//! - things that are due this month
//! - linked to a certain Asset via the entity field
//! - linked to one of a series of projects
//! - status is not yet approved
//!
//! # Complex Filters
//!
//! Complex filters are created with [`complex()`] and are comprised of a
//! *combination of filters and logical filter operators*.
//!
//! The chief difference between "basic" and "complex" filters is [`complex()`]
//! allows you to define groups of filters that will match records using any/all
//! semantics via the *logical filter operator* [`and()`] and [`or()`] functions.
//!
//! Complex filters can represent branching logic by nesting
//! [`and()`]/[`or()`] arbitrarily along with filters conditions produced by
//! [`field()`].
//!
//! ```
//! use shotgun_rs::filters::{self, field, FieldValue};
//!
//! // Find tasks that either have
//! // - a due date but no assignee, or
//! // - a start date but no due date
//! let tasks_for_audit = filters::complex(
//!     filters::or(&[
//!         filters::and(&[
//!             field("due_date").is_not(FieldValue::None),
//!             field("task_assignee").is(FieldValue::None),
//!         ]),
//!         filters::and(&[
//!             field("start_date").is_not(FieldValue::None),
//!             field("due_date").is(FieldValue::None),
//!         ]),
//!     ])
//! ).unwrap();
//! ```
//!
//! > The "root" argument to [`complex()`] must be either [`and()`] or [`or()`],
//! > but based on the current type signatures in use, we can't prevent a filter
//! > condition from being accepted.
//! >
//! > For this reason, [`complex()`] returns a `Result` and will give an `Err`
//! > if a *filter* is supplied as the root instead of a *logical filter operator*.
//!
//! ## `Filter` vs `ComplexFilter`
//!
//! Distinct types are used to help enforce the more strict rules that separate
//! basic and complex filters.
//!
//! The [`field()`] function wll return [`Field`] which is compatible with the
//! [`basic()`] function.
//!
//! The [`and()`] and [`or()`] functions return [`ComplexFilter`] which is
//! required by [`complex()`].
//!
//! When mixing [`field()`] with [`and()`]/[`or()`] at the *same level*
//! (ie, when they are siblings), you may see type mismatch errors.
//!
//! The fix in this situation is to "upgrade" your `field()` usage using
//! `.into()`.
//!
//! ```
//! use shotgun_rs::filters::{self, field};
//!
//! let approved_ensemble = filters::complex(
//!     filters::and(&[
//!         // `.into()` here "upgrades" the `Filter` into a `ComplexFilter`,
//!         // matching the return type of `and()`/`or()`.
//!         field("sg_status_list").is("apr").into(),
//!
//!         filters::or(&[
//!             // `.into()` is not required when all members of the list are `Filter`.
//!             // The "upgrade" to `ComplexFilter` is handled automatically.
//!             field("name").starts_with("Bub"),
//!             field("name").starts_with("Courtney"),
//!             field("name").starts_with("Mitch"),
//!             field("name").starts_with("Neil"),
//!             field("name").starts_with("Norman"),
//!         ])
//!     ])
//! ).unwrap();
//! ```
//!
//! # See Also
//!
//! For more on filtering Shotgun queries:
//!
//! - <https://developer.shotgunsoftware.com/rest-api/#filtering>
//! - <https://developer.shotgunsoftware.com/python-api/reference.html#filter-syntax>

use serde::{
    ser::{SerializeMap, SerializeSeq},
    Serialize, Serializer,
};

pub const MIME_FILTER_ARRAY: &str = "application/vnd+shotgun.api3_array+json";
pub const MIME_FILTER_HASH: &str = "application/vnd+shotgun.api3_hash+json";

impl From<Filter> for ComplexFilter {
    fn from(x: Filter) -> Self {
        ComplexFilter::Filter(x)
    }
}
impl From<LogicalFilterOperator> for ComplexFilter {
    fn from(x: LogicalFilterOperator) -> Self {
        ComplexFilter::LogicalFilterOperator(x)
    }
}

/// The "basic" filter constructor.
///
/// Accepts a list of filter predicates. Records matching these filters must
/// satisfy all of the specified conditions.
pub fn basic(filters: &[Filter]) -> FinalizedFilters {
    FinalizedFilters::Basic(filters.to_vec())
}

/// The "complex" filter constructor. Accepts a "root" which should be a
/// [`ComplexFilter`] such as [`and()`] or [`or()`].
///
/// Inside the root, you can pass any combination of [`and()`], [`or()`], and
/// filters produced by [`field()`].
///
/// Will return `Err` if the root is a filter (rather than an [`and()`] or
/// [`or()`]).
pub fn complex(root: ComplexFilter) -> crate::Result<FinalizedFilters> {
    match root {
        ComplexFilter::LogicalFilterOperator(_) => {}
        _ => return Err(crate::Error::InvalidFilters),
    }

    Ok(FinalizedFilters::Complex(root))
}

/// Sometimes you don't really want to filter by anything!
/// We got you. Use an *empty* in this situation. It's wide open.
pub fn empty() -> FinalizedFilters {
    FinalizedFilters::Basic(vec![])
}

/// Finalized filter data, ready to be handed off to a query method.
#[derive(Clone, Serialize, Debug)]
#[serde(untagged)]
pub enum FinalizedFilters {
    Basic(Vec<Filter>),
    Complex(ComplexFilter),
}

impl FinalizedFilters {
    pub fn get_mime(&self) -> &'static str {
        match self {
            Self::Basic(_) => MIME_FILTER_ARRAY,
            Self::Complex(_) => MIME_FILTER_HASH,
        }
    }
}

/// These represent the groupings of filter clauses.
///
/// The *complex* filtering syntax mixes `And` and `Or` as if they were filters
/// themselves.
#[derive(Clone, Debug)]
pub enum LogicalFilterOperator {
    And(Vec<ComplexFilter>),
    Or(Vec<ComplexFilter>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ComplexFilter {
    LogicalFilterOperator(LogicalFilterOperator),
    Filter(Filter),
}

pub fn and<F>(conditions: &[F]) -> ComplexFilter
where
    F: Into<ComplexFilter> + Clone,
{
    ComplexFilter::LogicalFilterOperator(LogicalFilterOperator::And(
        conditions.to_vec().into_iter().map(Into::into).collect(),
    ))
}

pub fn or<F>(conditions: &[F]) -> ComplexFilter
where
    F: Into<ComplexFilter> + Clone,
{
    ComplexFilter::LogicalFilterOperator(LogicalFilterOperator::Or(
        conditions.to_vec().into_iter().map(Into::into).collect(),
    ))
}

impl Serialize for LogicalFilterOperator {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_map(Some(2))?;
        let (op, conditions) = match self {
            LogicalFilterOperator::And(conditions) => ("and", conditions),
            LogicalFilterOperator::Or(conditions) => ("or", conditions),
        };
        state.serialize_entry("logical_operator", op)?;
        state.serialize_entry("conditions", &conditions)?;
        state.end()
    }
}

/// The type and primary key of an entity record.
///
/// This is useful for writing filters for multi-entity link fields.
///
/// ```
/// use shotgun_rs::filters::{self, field, EntityRef};
///
/// field("entity").in_(&[
///     EntityRef::new("Shot", 123),
///     EntityRef::new("Shot", 456),
///     EntityRef::new("Sequence", 9000)
/// ]);
/// ```
#[derive(Clone, Debug, Serialize)]
pub struct EntityRef {
    r#type: String,
    id: i32,
}

impl EntityRef {
    pub fn new<S: Into<String>>(r#type: S, id: i32) -> Self {
        Self {
            r#type: r#type.into(),
            id,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Filter {
    Is {
        field: String,
        value: FieldValue,
    },
    IsNot {
        field: String,
        value: FieldValue,
    },
    LessThan {
        field: String,
        value: FieldValue,
    },
    GreaterThan {
        field: String,
        value: FieldValue,
    },
    Contains {
        field: String,
        value: FieldValue,
    },
    NotContains {
        field: String,
        value: FieldValue,
    },
    StartsWith {
        field: String,
        value: String,
    },
    EndsWith {
        field: String,
        value: String,
    },
    Between {
        field: String,
        lower: FieldValue,
        upper: FieldValue,
    },
    NotBetween {
        field: String,
        lower: FieldValue,
        upper: FieldValue,
    },
    InLast {
        field: String,
        value: i32,
        period: String,
    },
    InNext {
        field: String,
        value: i32,
        period: String,
    },
    In {
        field: String,
        values: Vec<FieldValue>,
    },
    TypeIs {
        field: String,
        // The docs call for this to be optional, but how the heck can a record
        // have no type?
        value: String,
    },
    TypeIsNot {
        field: String,
        // The docs call for this to be optional, but how the heck can a record
        // have no type?
        value: String,
    },
    InCalendarDay {
        field: String,
        value: FieldValue,
    },
    InCalendarWeek {
        field: String,
        value: FieldValue,
    },
    InCalendarMonth {
        field: String,
        value: FieldValue,
    },
    NameContains {
        field: String,
        value: String,
    },
    NameNotContains {
        field: String,
        value: String,
    },
    NameStartsWith {
        field: String,
        value: String,
    },
    NameEndsWith {
        field: String,
        value: String,
    },
}

impl Serialize for Filter {
    fn serialize<S>(&self, serializer: S) -> Result<<S as Serializer>::Ok, <S as Serializer>::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_seq(None)?;
        match self {
            Filter::Is { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("is")?;
                state.serialize_element(&value)?;
            }
            Filter::IsNot { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("is_not")?;
                state.serialize_element(&value)?;
            }
            Filter::LessThan { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("less_than")?;
                state.serialize_element(&value)?;
            }
            Filter::GreaterThan { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("greater_than")?;
                state.serialize_element(&value)?;
            }
            Filter::Contains { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("contains")?;
                state.serialize_element(&value)?;
            }
            Filter::NotContains { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("not_contains")?;
                state.serialize_element(&value)?;
            }
            Filter::StartsWith { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("starts_with")?;
                state.serialize_element(&value)?;
            }
            Filter::EndsWith { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("ends_with")?;
                state.serialize_element(&value)?;
            }
            Filter::Between {
                field,
                lower,
                upper,
            } => {
                state.serialize_element(&field)?;
                state.serialize_element("between")?;
                state.serialize_element(&lower)?;
                state.serialize_element(&upper)?;
            }
            Filter::NotBetween {
                field,
                lower,
                upper,
            } => {
                state.serialize_element(&field)?;
                state.serialize_element("not_between")?;
                state.serialize_element(&lower)?;
                state.serialize_element(&upper)?;
            }
            Filter::InLast {
                field,
                value,
                period,
            } => {
                state.serialize_element(&field)?;
                state.serialize_element("in_last")?;
                state.serialize_element(&value)?;
                state.serialize_element(&period)?;
            }
            Filter::InNext {
                field,
                value,
                period,
            } => {
                state.serialize_element(&field)?;
                state.serialize_element("in_next")?;
                state.serialize_element(&value)?;
                state.serialize_element(&period)?;
            }
            Filter::In { field, values } => {
                state.serialize_element(&field)?;
                state.serialize_element("in")?;
                state.serialize_element(&values)?;
            }
            Filter::TypeIs { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("type_is")?;
                state.serialize_element(&value)?;
            }
            Filter::TypeIsNot { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("type_is_not")?;
                state.serialize_element(&value)?;
            }
            Filter::InCalendarDay { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("in_calendar_day")?;
                state.serialize_element(&value)?;
            }
            Filter::InCalendarWeek { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("in_calendar_week")?;
                state.serialize_element(&value)?;
            }
            Filter::InCalendarMonth { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("in_calendar_month")?;
                state.serialize_element(&value)?;
            }
            Filter::NameContains { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("name_contains")?;
                state.serialize_element(&value)?;
            }
            Filter::NameNotContains { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("name_not_contains")?;
                state.serialize_element(&value)?;
            }
            Filter::NameStartsWith { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("name_starts_with")?;
                state.serialize_element(&value)?;
            }
            Filter::NameEndsWith { field, value } => {
                state.serialize_element(&field)?;
                state.serialize_element("name_ends_with")?;
                state.serialize_element(&value)?;
            }
        }
        state.end()
    }
}

pub fn field<S: Into<String>>(name: S) -> Field {
    Field { field: name.into() }
}

pub struct Field {
    /// The name of the field the filter will be run on.
    field: String,
}

impl Field {
    pub fn is<V>(self, value: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::Is {
            field: self.field,
            value: value.into(),
        }
    }

    // noinspection RsSelfConvention
    pub fn is_not<V>(self, value: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::IsNot {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn less_than<V>(self, value: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::LessThan {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn greater_than<V>(self, value: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::GreaterThan {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn contains<V>(self, value: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::Contains {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn not_contains<V>(self, value: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::NotContains {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn starts_with<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::StartsWith {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn ends_with<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::EndsWith {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn between<V>(self, lower: V, upper: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::Between {
            field: self.field,
            lower: lower.into(),
            upper: upper.into(),
        }
    }

    pub fn not_between<V>(self, lower: V, upper: V) -> Filter
    where
        V: Into<FieldValue>,
    {
        Filter::NotBetween {
            field: self.field,
            lower: lower.into(),
            upper: upper.into(),
        }
    }

    /// Matches dates within the past number of `period`, where `period` is
    /// one of: "HOUR", "DAY", "WEEK", "MONTH", "YEAR".
    pub fn in_last<S>(self, offset: i32, period: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::InLast {
            field: self.field,
            value: offset,
            period: period.into(),
        }
    }

    /// Matches dates within the next number of `period`, where `period` is
    /// one of: "HOUR", "DAY", "WEEK", "MONTH", "YEAR".
    pub fn in_next<S>(self, value: i32, period: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::InNext {
            field: self.field,
            value,
            period: period.into(),
        }
    }

    pub fn in_<V>(self, values: &[V]) -> Filter
    where
        V: Into<FieldValue> + Clone,
    {
        Filter::In {
            field: self.field,
            values: values.to_vec().into_iter().map(Into::into).collect(),
        }
    }

    pub fn type_is<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::TypeIs {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn type_is_not<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::TypeIsNot {
            field: self.field,
            value: value.into(),
        }
    }

    /// `offset` is a *relative-to-now* offset (e.g. 0 = today, 1 = tomorrow,
    /// -1 = yesterday).
    pub fn in_calendar_day(self, offset: i32) -> Filter {
        Filter::InCalendarDay {
            field: self.field,
            value: offset.into(),
        }
    }

    /// `offset` is a *relative-to-now* offset (e.g. 0 = this week,
    /// 1 = next week, -1 = last week).
    pub fn in_calendar_week(self, offset: i32) -> Filter {
        Filter::InCalendarWeek {
            field: self.field,
            value: offset.into(),
        }
    }

    /// `offset` is a *relative-to-now* offset (e.g. 0 = this month,
    /// 1 = next month, -1 = last month).
    pub fn in_calendar_month(self, offset: i32) -> Filter {
        Filter::InCalendarMonth {
            field: self.field,
            value: offset.into(),
        }
    }

    pub fn name_contains<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::NameContains {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn name_not_contains<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::NameNotContains {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn name_starts_with<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::NameStartsWith {
            field: self.field,
            value: value.into(),
        }
    }

    pub fn name_ends_with<S>(self, value: S) -> Filter
    where
        S: Into<String>,
    {
        Filter::NameEndsWith {
            field: self.field,
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum FieldValue {
    Bool(bool),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    UInt32(u32),
    UInt64(u64),
    String(String),
    EntityRef { r#type: String, id: i32 },
    None,
}

impl From<bool> for FieldValue {
    fn from(x: bool) -> Self {
        FieldValue::Bool(x)
    }
}
impl From<Option<bool>> for FieldValue {
    fn from(x: Option<bool>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&bool> for FieldValue {
    fn from(x: &bool) -> Self {
        FieldValue::Bool(*x)
    }
}
impl From<Option<&bool>> for FieldValue {
    fn from(x: Option<&bool>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<f32> for FieldValue {
    fn from(x: f32) -> Self {
        FieldValue::Float32(x)
    }
}
impl From<Option<f32>> for FieldValue {
    fn from(x: Option<f32>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&f32> for FieldValue {
    fn from(x: &f32) -> Self {
        FieldValue::Float32(*x)
    }
}
impl From<Option<&f32>> for FieldValue {
    fn from(x: Option<&f32>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<f64> for FieldValue {
    fn from(x: f64) -> Self {
        FieldValue::Float64(x)
    }
}
impl From<Option<f64>> for FieldValue {
    fn from(x: Option<f64>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&f64> for FieldValue {
    fn from(x: &f64) -> Self {
        FieldValue::Float64(*x)
    }
}
impl From<Option<&f64>> for FieldValue {
    fn from(x: Option<&f64>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<i32> for FieldValue {
    fn from(x: i32) -> Self {
        FieldValue::Int32(x)
    }
}
impl From<Option<i32>> for FieldValue {
    fn from(x: Option<i32>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&i32> for FieldValue {
    fn from(x: &i32) -> Self {
        FieldValue::Int32(*x)
    }
}
impl From<Option<&i32>> for FieldValue {
    fn from(x: Option<&i32>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<i64> for FieldValue {
    fn from(x: i64) -> Self {
        FieldValue::Int64(x)
    }
}
impl From<Option<i64>> for FieldValue {
    fn from(x: Option<i64>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&i64> for FieldValue {
    fn from(x: &i64) -> Self {
        FieldValue::Int64(*x)
    }
}
impl From<Option<&i64>> for FieldValue {
    fn from(x: Option<&i64>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<u32> for FieldValue {
    fn from(x: u32) -> Self {
        FieldValue::UInt32(x)
    }
}
impl From<Option<u32>> for FieldValue {
    fn from(x: Option<u32>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&u32> for FieldValue {
    fn from(x: &u32) -> Self {
        FieldValue::UInt32(*x)
    }
}
impl From<Option<&u32>> for FieldValue {
    fn from(x: Option<&u32>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<u64> for FieldValue {
    fn from(x: u64) -> Self {
        FieldValue::UInt64(x)
    }
}
impl From<Option<u64>> for FieldValue {
    fn from(x: Option<u64>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&u64> for FieldValue {
    fn from(x: &u64) -> Self {
        FieldValue::UInt64(*x)
    }
}
impl From<Option<&u64>> for FieldValue {
    fn from(x: Option<&u64>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<EntityRef> for FieldValue {
    fn from(x: EntityRef) -> Self {
        let EntityRef { r#type, id } = x;
        FieldValue::EntityRef { r#type, id }
    }
}
impl From<Option<EntityRef>> for FieldValue {
    fn from(x: Option<EntityRef>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<&EntityRef> for FieldValue {
    fn from(x: &EntityRef) -> Self {
        FieldValue::EntityRef {
            r#type: x.r#type.clone(),
            id: x.id,
        }
    }
}
impl From<Option<&EntityRef>> for FieldValue {
    fn from(x: Option<&EntityRef>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

impl From<&str> for FieldValue {
    fn from(x: &str) -> Self {
        FieldValue::String(x.into())
    }
}

impl From<&String> for FieldValue {
    fn from(x: &String) -> Self {
        FieldValue::String(x.clone())
    }
}
impl From<String> for FieldValue {
    fn from(x: String) -> Self {
        FieldValue::String(x)
    }
}
impl From<Option<&str>> for FieldValue {
    fn from(x: Option<&str>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<Option<&String>> for FieldValue {
    fn from(x: Option<&String>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}
impl From<Option<String>> for FieldValue {
    fn from(x: Option<String>) -> Self {
        match x {
            None => FieldValue::None,
            Some(x) => x.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_float_values() {
        // Comparing floats is annoying so we skip looking at these in the
        // "kitchen sink" tests.

        let val_f32: FieldValue = (1.1_f32).into();
        let val_f64: FieldValue = (1.1_f64).into();

        match val_f32 {
            FieldValue::Float32(_) => {}
            _ => panic!("unexpected value conversion"),
        }
        match val_f64 {
            FieldValue::Float64(_) => {}
            _ => panic!("unexpected value conversion"),
        }

        assert_relative_eq!(
            serde_json::json!(1.1_f32).as_f64().unwrap(),
            serde_json::json!(val_f32).as_f64().unwrap()
        );
        assert_relative_eq!(
            serde_json::json!(1.1_f64).as_f64().unwrap(),
            serde_json::json!(val_f64).as_f64().unwrap()
        );
    }

    #[test]
    fn test_int_values() {
        let val_i32: FieldValue = (1_i32).into();
        let val_u32: FieldValue = (1_u32).into();

        let val_i64: FieldValue = (1_i64).into();
        let val_u64: FieldValue = (1_u64).into();

        match val_i32 {
            FieldValue::Int32(_) => {}
            _ => panic!("unexpected value conversion"),
        }
        match val_u32 {
            FieldValue::UInt32(_) => {}
            _ => panic!("unexpected value conversion"),
        }

        match val_i64 {
            FieldValue::Int64(_) => {}
            _ => panic!("unexpected value conversion"),
        }

        match val_u64 {
            FieldValue::UInt64(_) => {}
            _ => panic!("unexpected value conversion"),
        }
    }

    #[test]
    fn test_string_values() {
        let owned = String::from("as_str");
        let borrowed = String::from("borrowed");
        field("static").starts_with("static");
        field(owned.clone()).starts_with(owned.clone());
        field(&borrowed).starts_with(&borrowed);
        field(owned.as_str()).starts_with(owned.as_str());
    }

    #[test]
    fn test_filter_params() {
        let _filters = &[
            field("project.Project.id").is(123),
            field("sg_status_list").is_not("cmpt"),
        ];
        field("project.Project.id").in_(&[Some(1), Some(2), Some(3), None]);
        field("project.Project.id").is(FieldValue::None);
        field("project.Project.id").between(1, 5);
        field("project.Project.id").between("a", "b");
    }

    #[test]
    fn test_basic_filters() {
        let filters = basic(&[
            field("project").name_not_contains("dev"),
            field("sg_status_list").is("apr"),
            field("sg_sort_priority").between(0, 20),
            field("created_by.HumanUser.id").in_(&[1, 2, 3]),
        ]);
        let expected = serde_json::json!([
            ["project", "name_not_contains", "dev"],
            ["sg_status_list", "is", "apr"],
            ["sg_sort_priority", "between", 0, 20],
            ["created_by.HumanUser.id", "in", [1, 2, 3]],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_complex_filters() {
        let filters = complex(and(&[
            field("sg_status_list").is("apr").into(),
            or(&[
                field("name").starts_with("Bub"),
                field("name").starts_with("Courtney"),
                field("name").starts_with("Mitch"),
                field("name").starts_with("Neil"),
                field("name").starts_with("Norman"),
            ]),
        ]))
        .unwrap();

        let expected = serde_json::json!({
            "logical_operator": "and",
            "conditions": [
                ["sg_status_list", "is", "apr"],
                {
                    "logical_operator": "or",
                    "conditions": [
                        ["name", "starts_with", "Bub"],
                        ["name", "starts_with", "Courtney"],
                        ["name", "starts_with", "Mitch"],
                        ["name", "starts_with", "Neil"],
                        ["name", "starts_with", "Norman"],
                    ]
                }
            ]
        });
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_is() {
        let filters = basic(&[
            field("x").is(false),
            field("x").is(1_i32),
            field("x").is(1_i64),
            field("x").is("one"),
            field("x").is(EntityRef::new("Asset", 123)),
            field("x").is(FieldValue::None),
            field("x").is_not(false),
            field("x").is_not(1_i32),
            field("x").is_not(1_i64),
            field("x").is_not("one"),
            field("x").is_not(EntityRef::new("Asset", 123)),
            field("x").is_not(FieldValue::None),
        ]);
        let expected = serde_json::json!([
            ["x", "is", false],
            ["x", "is", 1],
            ["x", "is", 1],
            ["x", "is", "one"],
            ["x", "is", { "type": "Asset", "id": 123 }],
            ["x", "is", null],
            ["x", "is_not", false],
            ["x", "is_not", 1],
            ["x", "is_not", 1],
            ["x", "is_not", "one"],
            ["x", "is_not", { "type": "Asset", "id": 123 }],
            ["x", "is_not", null],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_less_than_greater_than() {
        let filters = basic(&[
            field("x").less_than(false),
            field("x").less_than(1_i32),
            field("x").less_than(1_i64),
            field("x").less_than("one"),
            field("x").less_than(EntityRef::new("Asset", 123)),
            field("x").less_than(FieldValue::None),
            field("x").greater_than(false),
            field("x").greater_than(1_i32),
            field("x").greater_than(1_i64),
            field("x").greater_than("one"),
            field("x").greater_than(EntityRef::new("Asset", 123)),
            field("x").greater_than(FieldValue::None),
        ]);

        let expected = serde_json::json!([
            ["x", "less_than", false],
            ["x", "less_than", 1],
            ["x", "less_than", 1],
            ["x", "less_than", "one"],
            ["x", "less_than", { "type": "Asset", "id": 123 }],
            ["x", "less_than", null],
            ["x", "greater_than", false],
            ["x", "greater_than", 1],
            ["x", "greater_than", 1],
            ["x", "greater_than", "one"],
            ["x", "greater_than", { "type": "Asset", "id": 123 }],
            ["x", "greater_than", null],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_contains() {
        let filters = basic(&[
            field("x").contains(false),
            field("x").contains(1_i32),
            field("x").contains(1_i64),
            field("x").contains("one"),
            field("x").contains(EntityRef::new("Asset", 123)),
            field("x").contains(FieldValue::None),
            field("x").not_contains(false),
            field("x").not_contains(1_i32),
            field("x").not_contains(1_i64),
            field("x").not_contains("one"),
            field("x").not_contains(EntityRef::new("Asset", 123)),
            field("x").not_contains(FieldValue::None),
        ]);
        let expected = serde_json::json!([
            ["x", "contains", false],
            ["x", "contains", 1],
            ["x", "contains", 1],
            ["x", "contains", "one"],
            ["x", "contains", { "type": "Asset", "id": 123 }],
            ["x", "contains", null],
            ["x", "not_contains", false],
            ["x", "not_contains", 1],
            ["x", "not_contains", 1],
            ["x", "not_contains", "one"],
            ["x", "not_contains", { "type": "Asset", "id": 123 }],
            ["x", "not_contains", null],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_substr() {
        let filters = basic(&[
            field("x").starts_with("prefix"),
            field("x").ends_with("suffix"),
            field("x").name_contains("something"),
            field("x").name_not_contains("something"),
            field("x").name_starts_with("something"),
            field("x").name_ends_with("something"),
        ]);
        let expected = serde_json::json!([
            ["x", "starts_with", "prefix"],
            ["x", "ends_with", "suffix"],
            ["x", "name_contains", "something"],
            ["x", "name_not_contains", "something"],
            ["x", "name_starts_with", "something"],
            ["x", "name_ends_with", "something"],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_between() {
        let filters = basic(&[
            field("x").between(false, true),
            field("x").between(None, Some(true)),
            field("x").between(Some(false), None),
            field("x").between(1_i32, 2_i32),
            field("x").between(None, Some(2_i32)),
            field("x").between(Some(1_i32), None),
            field("x").between(1_i64, 2_i64),
            field("x").between(None, Some(2_i64)),
            field("x").between(Some(1_i64), None),
            field("x").between("one", "two"),
            field("x").between(None, Some("two")),
            field("x").between(Some("one"), None),
            field("x").between(EntityRef::new("Asset", 123), EntityRef::new("Asset", 456)),
            field("x").between(None, Some(EntityRef::new("Asset", 456))),
            field("x").between(Some(EntityRef::new("Asset", 123)), None),
            field("x").between(FieldValue::None, FieldValue::None), // ???
        ]);
        let expected = serde_json::json!([
            ["x", "between", false, true],
            ["x", "between", null, true],
            ["x", "between", false, null],
            ["x", "between", 1, 2],
            ["x", "between", null, 2],
            ["x", "between", 1, null],
            ["x", "between", 1, 2],
            ["x", "between", null, 2],
            ["x", "between", 1, null],
            ["x", "between", "one", "two"],
            ["x", "between", null, "two"],
            ["x", "between", "one", null],
            ["x", "between", { "type": "Asset", "id": 123 }, { "type": "Asset", "id": 456 }],
            ["x", "between", null, { "type": "Asset", "id": 456 }],
            ["x", "between", { "type": "Asset", "id": 123 }, null],
            ["x", "between", null, null],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }
    #[test]
    fn test_field_kitchen_sink_not_between() {
        let filters = basic(&[
            field("x").not_between(false, true),
            field("x").not_between(None, Some(true)),
            field("x").not_between(Some(false), None),
            field("x").not_between(1_i32, 2_i32),
            field("x").not_between(None, Some(2_i32)),
            field("x").not_between(Some(1_i32), None),
            field("x").not_between(1_i64, 2_i64),
            field("x").not_between(None, Some(2_i64)),
            field("x").not_between(Some(1_i64), None),
            field("x").not_between("one", "two"),
            field("x").not_between(None, Some("two")),
            field("x").not_between(Some("one"), None),
            field("x").not_between(EntityRef::new("Asset", 123), EntityRef::new("Asset", 456)),
            field("x").not_between(None, Some(EntityRef::new("Asset", 456))),
            field("x").not_between(Some(EntityRef::new("Asset", 123)), None),
            field("x").not_between(FieldValue::None, FieldValue::None), // ???
        ]);
        let expected = serde_json::json!([
            ["x", "not_between", false, true],
            ["x", "not_between", null, true],
            ["x", "not_between", false, null],
            ["x", "not_between", 1, 2],
            ["x", "not_between", null, 2],
            ["x", "not_between", 1, null],
            ["x", "not_between", 1, 2],
            ["x", "not_between", null, 2],
            ["x", "not_between", 1, null],
            ["x", "not_between", "one", "two"],
            ["x", "not_between", null, "two"],
            ["x", "not_between", "one", null],
            ["x", "not_between", { "type": "Asset", "id": 123 }, { "type": "Asset", "id": 456 }],
            ["x", "not_between", null, { "type": "Asset", "id": 456 }],
            ["x", "not_between", { "type": "Asset", "id": 123 }, null],
            ["x", "not_between", null, null]
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_next_last() {
        let filters = &[
            field("x").in_last(-1, "DAY"),
            field("x").in_last(0, "WEEK"),
            field("x").in_last(1, "MONTH"),
            field("x").in_next(-1, "DAY"),
            field("x").in_next(0, "WEEK"),
            field("x").in_next(1, "MONTH"),
        ];
        let expected = &[
            serde_json::json!(["x", "in_last", -1, "DAY"]),
            serde_json::json!(["x", "in_last", 0, "WEEK"]),
            serde_json::json!(["x", "in_last", 1, "MONTH"]),
            serde_json::json!(["x", "in_next", -1, "DAY"]),
            serde_json::json!(["x", "in_next", 0, "WEEK"]),
            serde_json::json!(["x", "in_next", 1, "MONTH"]),
        ];

        for (expected, filter) in expected.iter().zip(filters) {
            assert_eq!(expected, &serde_json::json!(filter));
        }
    }

    #[test]
    fn test_field_kitchen_sink_in_() {
        let filters = basic(&[
            field("x").in_(&[1, 2, 3]),
            field("x").in_(&[1.1, 2.1, 3.1]),
            field("x").in_(&["a", "b", "c"]),
            field("x").in_(&[Some(EntityRef::new("Asset", 123)), None]),
        ]);
        let expected = serde_json::json!([
            ["x", "in", [1, 2, 3]],
            ["x", "in", [1.1, 2.1, 3.1]],
            ["x", "in", ["a", "b", "c"]],
            ["x", "in", [{ "type": "Asset", "id": 123 }, null]],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }

    #[test]
    fn test_field_kitchen_sink_type() {
        let filters = basic(&[field("x").type_is("Asset"), field("x").type_is_not("Asset")]);
        let expected =
            serde_json::json!([["x", "type_is", "Asset"], ["x", "type_is_not", "Asset"],]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }
    #[test]
    fn test_field_kitchen_sink_calendar() {
        let filters = basic(&[
            field("x").in_calendar_day(-1),
            field("x").in_calendar_day(0),
            field("x").in_calendar_day(1),
            field("x").in_calendar_week(-1),
            field("x").in_calendar_week(0),
            field("x").in_calendar_week(1),
            field("x").in_calendar_month(-1),
            field("x").in_calendar_month(0),
            field("x").in_calendar_month(1),
        ]);
        let expected = serde_json::json!([
            ["x", "in_calendar_day", -1],
            ["x", "in_calendar_day", 0],
            ["x", "in_calendar_day", 1],
            ["x", "in_calendar_week", -1],
            ["x", "in_calendar_week", 0],
            ["x", "in_calendar_week", 1],
            ["x", "in_calendar_month", -1],
            ["x", "in_calendar_month", 0],
            ["x", "in_calendar_month", 1],
        ]);
        assert_eq!(&expected, &serde_json::json!(filters));
    }
}
