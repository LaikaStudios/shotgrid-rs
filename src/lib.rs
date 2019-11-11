use futures::future::{self, Future};
use futures::Stream;
use std::env;
use std::fs::File;
use std::io::Read;
#[macro_use]
extern crate serde_derive;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
#[macro_use]
extern crate failure;
use log::{debug, error, trace};
/// This client represents the the http transport layer used by `Shotgun`.
///
/// Should you need to manually configure your client, you can do so then
/// initialize your Shotgun instance via `Shotgun::with_client()`.
pub use reqwest::r#async::Client;
use reqwest::r#async::Response;

use std::borrow::Cow;

/// Get a default http client with ca certs added to it if specified via env var.
fn get_client() -> Result<Client, ShotgunError> {
    let builder = Client::builder();

    let builder = if let Ok(fp) = env::var("CA_BUNDLE") {
        debug!("Using ca bundle from: `{}`", fp);
        let mut buf = Vec::new();
        File::open(fp)
            .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))?
            .read_to_end(&mut buf)
            .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))?;
        let cert = reqwest::Certificate::from_pem(&buf)
            .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))?;
        builder.add_root_certificate(cert)
    } else {
        builder
    };

    builder
        .build()
        .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))
}

fn get_filter_mime(filters: &Value) -> Result<&'static str, ShotgunError> {
    let maybe_filters = filters.get("filters");

    if maybe_filters.map(|v| v.is_array()) == Some(true) {
        Ok("application/vnd+shotgun.api3_array+json")
    } else if maybe_filters.map(|v| v.is_object()) == Some(true) {
        Ok("application/vnd+shotgun.api3_hash+json")
    } else {
        Err(ShotgunError::InvalidFilters)
    }
}

#[derive(Clone, Debug)]
pub struct Shotgun {
    /// Base url for the shotgun server.
    sg_server: String,
    /// HTTP Client used internally to make requests to shotgun.
    client: Client,
    /// API User (aka "script") name, used to generate API Tokens.
    script_name: Option<String>,
    /// API User (aka "script") secret key, used to generate API Tokens.
    script_key: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ErrorResponse {
    errors: Vec<ErrorObject>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ErrorObject {
    pub id: Option<String>,
    pub status: Option<i64>,
    pub code: Option<i64>,
    pub title: Option<String>,
    pub detail: Option<String>,
    pub source: Option<serde_json::Map<String, Value>>,
    pub meta: Option<serde_json::Map<String, Value>>,
}

impl Shotgun {
    /// Create a new Shotgun API Client using all defaults.
    ///
    /// By default, the HTTP Client initialized while looking to a `CA_BUNDLE` environment var
    /// for a file path to a TLS cert.
    ///
    /// This will `Err` when:
    ///
    /// - `CA_BUNDLE` is set, but the file path it points to is invalid.
    pub fn new(
        sg_server: String,
        script_name: Option<&str>,
        script_key: Option<&str>,
    ) -> Result<Self, ShotgunError> {
        let client = get_client()?;
        Ok(Self {
            sg_server,
            client,
            script_name: script_name.map(Into::into),
            script_key: script_key.map(Into::into),
        })
    }

    /// Create a new Shotgun API Client, but configure the HTTP client yourself.
    ///
    /// This may be the option for you if you need to adjust resource limits, or timeouts, etc on
    /// the HTTP client itself.
    pub fn with_client(
        sg_server: String,
        script_name: Option<&str>,
        script_key: Option<&str>,
        client: Client,
    ) -> Self {
        Self {
            sg_server,
            client,
            script_name: script_name.map(Into::into),
            script_key: script_key.map(Into::into),
        }
    }

    /// Handles running authentication requests.
    fn authenticate<D: 'static>(
        &self,
        form_data: &[(&str, &str)],
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        self.client
            .post(&format!("{}/api/v1/auth/access_token", self.sg_server))
            .form(form_data)
            .header("Accept", "application/json")
            .send()
            .from_err()
            .and_then(handle_response)
    }

    /// Run a credential (human user logging in) challenge.
    pub fn authenticate_user<D: 'static>(
        &self,
        username: &str,
        password: &str,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        self.authenticate(&[
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
        ])
    }

    /// Get an access token payload for a given Api User aka "script."
    ///
    /// This function relies on the script key and name fields being set and will fail with a
    /// `ShotgunError::BadClientConfig` if either is missing.
    pub fn authenticate_script<D: 'static>(&self) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        if let (Some(script_name), Some(script_key)) =
            (self.script_name.as_ref(), self.script_key.as_ref())
        {
            future::Either::A(self.authenticate(&[
                ("grant_type", "client_credentials"),
                ("client_id", &script_name),
                ("client_secret", &script_key),
            ]))
        } else {
            future::Either::B(future::err(ShotgunError::BadClientConfig(
                "Missing script name or key.".into(),
            )))
        }
    }

    /// The same as `authenticate_script()` except it also allows you to pass a username
    /// to "sudo" as.
    ///
    /// This function relies on the script key and name fields being set and will fail with a
    /// `ShotgunError::BadClientConfig` if either is missing.
    pub fn authenticate_script_as_user<D: 'static>(
        &self,
        login: &str,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        if let (Some(script_name), Some(script_key)) =
            (self.script_name.as_ref(), self.script_key.as_ref())
        {
            future::Either::A(self.authenticate(&[
                ("grant_type", "client_credentials"),
                ("client_id", &script_name),
                ("client_secret", &script_key),
                ("scope", &format!("sudo_as_login:{}", login)),
            ]))
        } else {
            future::Either::B(future::err(ShotgunError::BadClientConfig(
                "Missing script name or key.".into(),
            )))
        }
    }

    pub fn schema_read<D: 'static>(
        &self,
        token: &str,
        project_id: Option<i32>,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!("{}/api/v1/schema", self.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(id) = project_id {
            req = req.query(&[("project_id", id)]);
        }
        req.send().from_err().and_then(handle_response)
    }

    /// Return all schema field information for a given entity.
    /// Entity should be a snake cased version of the entity name.
    /// https://developer.shotgunsoftware.com/rest-api/#read-all-field-schemas-for-an-entity
    pub fn schema_fields_read<D: 'static>(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/schema/{}/fields",
                self.sg_server, entity
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(id) = project_id {
            req = req.query(&[("project_id", id)]);
        }
        req.send().from_err().and_then(handle_response)
    }

    /// Returns schema information about a specific field on a given entity.
    /// Entity should be a snaked cased version of the entity name.
    /// https://developer.shotgunsoftware.com/rest-api/#read-one-field-schema-for-an-entity
    pub fn schema_field_read<D: 'static>(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
        field_name: &str,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/schema/{}/fields/{}",
                self.sg_server, entity, field_name,
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(id) = project_id {
            req = req.query(&[("project_id", id)]);
        }

        req.send().from_err().and_then(handle_response)
    }

    /// Batch execute requests
    pub fn batch<D: 'static>(
        &self,
        token: &str,
        data: Value,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        self.client
            .post(&format!("{}/api/v1/entity/_batch", self.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data)
            .send()
            .from_err()
            .and_then(handle_response)
    }

    /// Create a new entity.
    ///
    /// The `data` field is used the request body, and as such should be an object where the keys
    /// are fields on the entity in question.
    ///
    /// `fields` can be specified to limit the returned fields from the request.
    /// Passing `None` will use the default behavior of returning _all fields_.
    ///
    /// > **Note**: `fields` currently does nothing due to a shotgun bug.
    /// > No ETA on the fix:
    /// > https://support.shotgunsoftware.com/hc/en-us/requests/106834
    pub fn create<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        data: Value,
        fields: Option<&str>,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let mut req = self
            .client
            .post(&format!("{}/api/v1/entity/{}", self.sg_server, entity,))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);

        if let Some(fields) = fields {
            req = req.query(&[("options[fields]", fields)]);
        }
        req.send().from_err().and_then(handle_response)
    }

    /// Read the data for a single entity.
    ///
    /// `fields` is an optional comma separated list of field names to return in the response.
    pub fn read<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        id: i32,
        fields: Option<&str>,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}",
                self.sg_server, entity, id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(fields) = fields {
            req = req.query(&[("fields", fields)]);
        }

        req.send().from_err().and_then(handle_response)
    }

    /// Modify an existing entity.
    ///
    /// `data` is used as the request body and as such should be an object with keys and values
    /// corresponding to the fields on the given entity.
    pub fn update<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        id: i32,
        data: Value,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        self.client
            .put(&format!(
                "{}/api/v1/entity/{}/{}",
                self.sg_server, entity, id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data)
            .send()
            .from_err()
            .and_then(handle_response)
    }

    /// Destroy (delete) an entity.
    pub fn destroy(
        &self,
        token: &str,
        entity: &str,
        id: i32,
    ) -> impl Future<Item = (), Error = ShotgunError> {
        let url = format!("{}/api/v1/entity/{}/{}", self.sg_server, entity, id,);
        self.client
            .delete(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .from_err()
            .and_then(move |resp| {
                if resp.status().is_success() {
                    Ok(())
                } else {
                    Err(ShotgunError::Unexpected(format!(
                        "Server responded to `DELETE {}` with `{}`",
                        &url,
                        resp.status()
                    )))
                }
            })
    }

    /// Find a list of entities matching some filter criteria.
    ///
    /// Search provides access to the Shotgun filter APIs, serving the same use cases as
    /// `find` from the Python client API.
    ///
    /// Filters come in 2 flavors, `Array` and `Hash`. These names refer to the shape of the data
    /// structure the filters are stored in. `Array` is the more simple of the two, and `Hash`
    /// offers more complex filter operations.
    ///
    /// For details on the filter syntax, please refer to the docs:
    ///
    /// https://developer.shotgunsoftware.com/rest-api/#searching
    ///
    pub fn search<D: 'static>(
        // FIXME: many parameters here can often be ignored. Switch to builder pattern.
        &self,
        token: &str,
        entity: &str,
        fields: &str,
        filters: &Value,
        sort: Option<String>,
        pagination: Option<PaginationParameter>,
        options: Option<OptionsParameter>,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let pagination = pagination
            .or_else(|| Some(PaginationParameter::default()))
            .unwrap();

        let content_type = match get_filter_mime(filters) {
            // early return if the filters are bogus and fail the sniff test
            Err(e) => return future::Either::A(future::err(e)),
            Ok(mime) => mime,
        };

        let mut qs: Vec<(&str, Cow<str>)> = vec![
            ("fields", Cow::Borrowed(fields)),
            ("page[number]", Cow::Owned(format!("{}", pagination.number))),
        ];

        // The page size is optional so we don't have to hard code
        // shotgun's *current* default of 500 into the library.
        //
        // If/when shotgun changes their default, folks who haven't
        // specified a page size should get whatever shotgun says, not *our*
        // hard-coded default.
        if let Some(size) = pagination.size {
            qs.push(("page[size]", Cow::Owned(format!("{}", size))));
        }

        if let Some(sort) = sort {
            qs.push(("sort", Cow::Owned(sort)));
        }

        if let Some(opts) = options {
            if let Some(return_only) = opts.return_only {
                qs.push((
                    "options[return_only]",
                    Cow::Borrowed(match return_only {
                        ReturnOnly::Active => "active",
                        ReturnOnly::Retired => "retired",
                    }),
                ));
            }

            if let Some(include_archived_projects) = opts.include_archived_projects {
                qs.push((
                    "options[include_archived_projects]",
                    Cow::Owned(format!("{}", include_archived_projects)),
                ));
            }
        }

        let f = self
            .client
            .post(&format!(
                "{}/api/v1/entity/{}/_search",
                self.sg_server, entity
            ))
            .query(&qs)
            .header("Accept", "application/json")
            .bearer_auth(token)
            .header("Content-Type", content_type)
            // XXX: the content type is being set to shotgun's custom mime types
            //   to indicate the shape of the filter payload. Do not be tempted to use
            //   `.json()` here instead of `.body()` or you'll end up reverting the
            //   header set above.
            .body(filters.to_string())
            .send()
            .from_err()
            .and_then(handle_response);
        future::Either::B(f)
    }

    /// Search for entities of the given type(s) and returns a list of basic entity data
    /// that fits the search. Rich filters can be used to narrow down searches to entities
    /// that match the filters.
    ///
    /// For details on the filter syntax, please refer to the docs:
    ///
    /// https://developer.shotgunsoftware.com/rest-api/#search-text-entries
    ///
    pub fn text_search<D: 'static>(
        &self,
        token: &str,
        filters: &Value,
        pagination: Option<PaginationParameter>,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let pagination = pagination
            .or_else(|| Some(PaginationParameter::default()))
            .unwrap();

        //
        let mut filters = filters.clone();
        {
            let map = filters.as_object_mut().unwrap();
            map.insert("page".to_string(), json!(pagination));
        }

        self.client
            .post(&format!("{}/api/v1/entity/_text_search", self.sg_server))
            .header("Content-Type", "application/vnd+shotgun.api3_array+json")
            .header("Accept", "application/json")
            .bearer_auth(token)
            .body(filters.to_string())
            .send()
            .from_err()
            .and_then(handle_response)
    }
}

/// Checks to see if the `Value` is an object with a top level "errors" key.
fn contains_errors(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|obj| Some(obj.contains_key("errors")))
        .unwrap_or(false)
}

/// Converts a response body from shotgun into something more meaningful.
///
/// There are a handful of ways requests can be fulfilled:
///
/// - Good! _You got a payload that matches your expected shape_.
/// - Bad! _The payload is legit, but doesn't conform to your expectations_.
/// - More Bad! _The request you sent didn't make sense to shotgun, so shotgun replied
///   with some error details_.
/// - Really Bad! _The response was total garbage; can't even be parsed as json_.
///
/// This function aims to cover converting the raw body into either the shape you requested, or an
/// Error with some details about what went wrong if your shape doesn't fit, or any of that other
/// stuff happened.
fn handle_response<D>(resp: Response) -> impl Future<Item = D, Error = ShotgunError>
where
    D: DeserializeOwned,
{
    resp.into_body().concat2().from_err().and_then(|bytes| {
        // There are three (3) potential failure modes here:
        //
        // 1. Connection problems could lead to partial/garbled/non-json payload
        //    resulting in a json parse error.
        // 2. The payload could be json, but contain an error message from shotgun about
        //    the filter.
        // 3. The payload might parse as valid json, but the json might not fit the
        //    deserialization target `D`.
        let res: Result<D, ShotgunError> = match serde_json::from_slice::<Value>(&bytes) {
            Err(e) => {
                // case 1 - non-valid json
                error!("Failed to parse payload: `{}` - `{:?}`", e, &bytes);
                // if we can't parse the json at all, bail as-is
                Err(ShotgunError::from(e))
            }
            Ok(v) => {
                if contains_errors(&v) {
                    trace!("Got error response from shotgun:\n{}", &v.to_string());
                    // case 2 - shotgun response has error feedback.
                    match serde_json::from_value::<ErrorResponse>(v) {
                        Ok(resp) => {
                            let maybe_not_found = resp
                                .errors
                                .iter()
                                .find(|ErrorObject { status, .. }| status == &Some(404));

                            if let Some(ErrorObject { detail, .. }) = maybe_not_found {
                                Err(ShotgunError::NotFound(
                                    detail.clone().unwrap_or_else(|| "".into()),
                                ))
                            } else {
                                Err(ShotgunError::ServerError(resp.errors))
                            }
                        }
                        // also, a non-valid json/shape sub-case if the response doesn't
                        // look as expected.
                        Err(err) => Err(ShotgunError::from(err)),
                    }
                } else {
                    // case 3 - either we get the shape we want or we get an error
                    serde_json::from_value::<D>(v).map_err(ShotgunError::from)
                }
            }
        };
        res
    })
}

#[derive(Clone, Debug)]
pub enum ReturnOnly {
    Active,
    Retired,
}

#[derive(Clone, Debug)]
pub struct OptionsParameter {
    pub return_only: Option<ReturnOnly>,
    pub include_archived_projects: Option<bool>,
}

/// This controls the paging of search-style list API calls.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PaginationParameter {
    ///  Pages start at 1, not 0.
    pub number: usize,
    /// Shotgun's default currently is 500
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
}

impl Default for PaginationParameter {
    fn default() -> Self {
        Self {
            number: 1,
            size: None,
        }
    }
}

#[derive(Debug, Fail)]
pub enum ShotgunError {
    #[fail(display = "Client Configuration Error: `{}`.", _0)]
    BadClientConfig(String),

    #[fail(
        display = "Invalid Filters: expected `filters` key to be array or object; was neither."
    )]
    InvalidFilters,

    #[fail(display = "JSON Parse Error: `{}`.", _0)]
    ClientError(#[fail(cause)] reqwest::Error),

    #[fail(display = "JSON Parse Error: `{}`.", _0)]
    JsonParse(#[fail(cause)] serde_json::Error),

    #[fail(display = "Entity Not Found - `{}`", _0)]
    NotFound(String),

    #[fail(display = "Authentication Failed - `{}`", _0)]
    Unauthorized(#[fail(cause)] reqwest::Error),

    #[fail(display = "Unexpected Error - `{}`", _0)]
    Unexpected(String),

    #[fail(display = "Server Error - `{:?}`", _0)]
    ServerError(Vec<ErrorObject>),
}

impl From<serde_json::Error> for ShotgunError {
    fn from(e: serde_json::Error) -> Self {
        ShotgunError::JsonParse(e)
    }
}

impl From<reqwest::Error> for ShotgunError {
    fn from(e: reqwest::Error) -> Self {
        ShotgunError::ClientError(e)
    }
}

/// Response from Shotgun after a successful auth challenge.
#[derive(Clone, Debug, Deserialize)]
pub struct TokenResponse {
    pub token_type: String,
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
}
