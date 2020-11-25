//! # Welcome to Shotgun-rs!
//!
//! This is a delicately hand-crafted REST API client for working with
//! [Autodesk Shotgun][shotgun].
//!
//! ## Features
//!
//! There are a handful of features to help control the configuration of the
//! underlying HTTP client.
//!
//! > By default `native-tls` is enabled, which uses the
//! > [native-tls crate] to delegate to whatever the canonical tls implementation
//! > is for the platform.
//! > The expectation is the system will already have this library installed
//! > (whichever library it may be).
//! >
//! > Other tls backends are available and can be selected with the features
//! > listed below.
//!
//! - `gzip` (to enable gzip compression).
//! - `native-tls` (as discussed above, uses whatever the canonical tls library
//!    is for the platform).
//! - `native-tls-vendored` (same as `native-tls` but will compile the tls
//!    library from source as a part of the crate's build script).
//! - `rustls` (uses the [rustls crate] which is a *pure rust tls implementation*).
//!
//! ## Usage
//!
//! The general pattern of usage starts with a `shotgun_rs::Shotgun` client.
//!
//! ```no_run
//! # use shotgun_rs::Shotgun;
//! # #[tokio::main]
//! # async fn main() -> shotgun_rs::Result<()> {
//! let server = "https://my-shotgun.example.com";
//! let script_name = "my-api-user";
//! let script_key = "********";
//!
//! let sg = Shotgun::new(server.to_string(), Some(script_name), Some(script_key))?;
//! # Ok(())
//! # }
//! ```
//!
//! Once your client is in hand, you'd use one of the authentication methods to
//! get an `access_token`.
//!
//! ```no_run
//! # use shotgun_rs::{Shotgun, TokenResponse};
//! # #[tokio::main]
//! # async fn main() -> shotgun_rs::Result<()> {
//! #    let server = "https://my-shotgun.example.com";
//! #    let script_name = "my-api-user";
//! #    let script_key = "********";
//! #    let sg = Shotgun::new(server.to_string(), Some(script_name), Some(script_key))?;
//! // Authenticates using the script name and script key held by the client.
//! let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
//! # Ok(())
//! # }
//! ```
//!
//! From there, you can pass that access token around to the various query methods.
//!
//! For operations where the schema of the response is flexible (based on the
//! entity type and return fields specified), we use generics to allow the
//! caller to unpack the response into the type of their choosing. The type just
//! needs to implement [serde]'s `Deserialize` trait. A number of structs are
//! provided (ex: `TokenResponse`) to cover responses that are pretty much the
//! same for everybody.
//!
//! Others structs are generic over types deeper in the data structure.
//! For example, `ResourceArrayResponse<R, L>` is generic over `R`
//! (the resource) which is the items in the array portion of the response, and
//! `L` which is the type for the response's "links" key).
//!
//! ```no_run
//! use serde_derive::Deserialize;
//! use serde_json::json;
//! use shotgun_rs::types::{PaginationLinks, ResourceArrayResponse, SelfLink};
//! use shotgun_rs::{Shotgun, TokenResponse};
//!
//!
//! /// This struct should match the return fields specified for the search.
//! #[derive(Debug, Clone, Deserialize)]
//! struct ProjectAttrs {
//!     code: Option<String>,
//!     name: Option<String>,
//! }
//!
//! #[derive(Clone, Debug, Deserialize)]
//! struct Project {
//!     id: Option<i32>,
//!     r#type: Option<String>,
//!     attributes: Option<ProjectAttrs>,
//!     links: Option<SelfLink>,
//! }
//!
//!
//! #[tokio::main]
//! async fn main() -> shotgun_rs::Result<()> {
//!
//!     let server = "https://my-shotgun.example.com";
//!     let script_name = "my-api-user";
//!     let script_key = "********";
//!
//!     let sg = Shotgun::new(server.to_string(), Some(script_name), Some(script_key))?;
//!
//!     let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
//!
//!     let return_fields = ["id", "code", "name"].join(",");
//!
//!     // Using type ascription (or a turbofish), we tell search() how to
//!     // deserialize the response.
//!     let resp: ResourceArrayResponse<Project, PaginationLinks> = sg
//!         .search(&access_token, "Project", &return_fields, &json!([]))?
//!         .size(Some(3))
//!         .execute()
//!         .await?;
//!
//!     let items = resp.data.unwrap_or_default();
//!
//!     for project in items {
//!         println!("{:?}", project);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! For times where you don't want to bother defining structs to represent the
//! response, you can always deserialize to a `serde_json::Value` and interrogate
//! the value yourself.
//!
//! ## Logging
//!
//! The `shotgun_rs` crate offers some logging, though most of it relates to the
//! internals of the library itself.
//!
//! If you're interested in logging the HTTP-transport layer, since we're using
//! [reqwest], you can get some visibility into the transport layer by setting
//! `reqwest` to `DEBUG`.
//!
//! Please refer to the docs for your logger crate to see how to adjust log levels
//! for crates and modules.
//!
//! [native-tls crate]: https://crates.io/crates/native-tls
//! [rustls crate]: https://crates.io/crates/rustls
//! [shotgun]: https://www.shotgunsoftware.com/
//! [reqwest]: https://crates.io/crates/reqwest
//! [serde]: https://crates.io/crates/serde
//! [serde_json]: https://crates.io/crates/serde_json

use std::borrow::Cow;
use std::env;
use std::fs::File;
use std::io::Read;
#[macro_use]
extern crate serde_derive;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
#[macro_use]
extern crate failure;
use crate::types::{
    AltImages, BatchedRequestsResponse, CreateFieldRequest, EntityActivityStreamResponse,
    EntityIdentifier, ErrorObject, ErrorResponse, FieldHashResponse, Grouping,
    HierarchyExpandRequest, HierarchyExpandResponse, HierarchySearchRequest,
    HierarchySearchResponse, OptionsParameter, PaginationParameter, ProjectAccessUpdateResponse,
    ReturnOnly, SchemaEntityResponse, SchemaFieldResponse, SchemaFieldsResponse, SummarizeRequest,
    SummaryField, SummaryOptions, UpdateFieldRequest, UploadInfoResponse,
};
use log::{debug, error, trace};
use reqwest::Response;
/// Re-export to provide access in case callers need to manually configure the
/// Client via `Shotgun::with_client()`.
// FIXME: re-export the whole reqwest crate.
pub use reqwest::{Certificate, Client};
use std::collections::HashMap;
pub mod types;
mod upload;
pub use upload::{UploadReqBuilder, MAX_MULTIPART_CHUNK_SIZE, MIN_MULTIPART_CHUNK_SIZE};
pub type Result<T> = std::result::Result<T, ShotgunError>;

/// Get a default http client with ca certs added to it if specified via env var.
fn get_client() -> Result<Client> {
    let builder = Client::builder();

    let builder = if let Ok(fp) = env::var("CA_BUNDLE") {
        debug!("Using ca bundle from: `{}`", fp);
        let mut buf = Vec::new();
        File::open(fp)
            .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))?
            .read_to_end(&mut buf)
            .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))?;
        let cert = Certificate::from_pem(&buf)
            .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))?;
        builder.add_root_certificate(cert)
    } else {
        builder
    };
    builder
        .build()
        .map_err(|e| ShotgunError::BadClientConfig(e.to_string()))
}

fn get_filter_mime(maybe_filters: &Value) -> Result<&'static str> {
    if maybe_filters.is_array() {
        Ok("application/vnd+shotgun.api3_array+json")
    } else if maybe_filters.is_object() {
        Ok("application/vnd+shotgun.api3_hash+json")
    } else {
        Err(ShotgunError::InvalidFilters)
    }
}

// Gets the mime type based on the entity_types.
// If they don't all match the same type (array vs object), an error is returned
fn get_entity_types_mime(maybe_filters: &Value) -> Result<&'static str> {
    let mut content_type: Option<&str> = None;
    let filters = maybe_filters["entity_types"].as_object();
    // FIXME: check to make sure there's at least one key in this object.
    if filters.is_none() {
        return Err(ShotgunError::InvalidFilters);
    }

    for (_, value) in filters.unwrap() {
        content_type = match get_filter_mime(&value) {
            Err(e) => return Err(e),
            Ok(mime) => {
                // If all entity_type filters don't match the same content_type, raise an error
                if content_type.is_some() && Some(mime) != content_type {
                    return Err(ShotgunError::InvalidFilters);
                }
                Some(mime)
            }
        };
    }
    // Should not panic because we return Err in all other cases
    // FIXME: this does panic if the "entity_types" key is an empty object
    Ok(content_type.unwrap())
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

pub struct SearchBuilder<'a> {
    sg: &'a Shotgun,
    token: &'a str,
    entity: &'a str,
    fields: &'a str,
    filters: &'a Value,
    sort: Option<String>,
    pagination: Option<PaginationParameter>,
    options: Option<OptionsParameter>,
}

impl<'a> SearchBuilder<'a> {
    pub fn new(
        sg: &'a Shotgun,
        token: &'a str,
        entity: &'a str,
        fields: &'a str,
        filters: &'a Value,
    ) -> Result<SearchBuilder<'a>> {
        Ok(SearchBuilder {
            sg,
            token,
            entity,
            fields,
            filters,
            sort: None,
            pagination: None,
            options: None,
        })
    }

    pub fn sort(mut self, value: Option<&'a str>) -> Self {
        self.sort = value.map(|f| f.to_string());
        self
    }

    pub fn size(mut self, value: Option<usize>) -> Self {
        let mut pagination = self.pagination.take().unwrap_or_default();
        if pagination.number.is_none() && value.is_none() {
            self.pagination = None;
        } else {
            pagination.size = value;
            self.pagination.replace(pagination);
        }
        self
    }

    pub fn number(mut self, value: Option<usize>) -> Self {
        let mut pagination = self.pagination.take().unwrap_or_default();
        if pagination.size.is_none() && value.is_none() {
            self.pagination = None;
        } else {
            pagination.number = value;
            self.pagination.replace(pagination);
        }
        self
    }

    pub fn return_only(mut self, value: Option<ReturnOnly>) -> Self {
        let mut options = self.options.take().unwrap_or_default();
        if options.include_archived_projects.is_none() && value.is_none() {
            self.options = None;
        } else {
            options.return_only = value;
            self.options.replace(options);
        }
        self
    }

    pub fn include_archived_projects(mut self, value: Option<bool>) -> Self {
        let mut options = self.options.take().unwrap_or_default();
        if options.return_only.is_none() && value.is_none() {
            self.options = None;
        } else {
            options.include_archived_projects = value;
            self.options.replace(options);
        }
        self
    }

    pub async fn execute<D: 'static>(self) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let content_type = match get_filter_mime(&self.filters) {
            // early return if the filters are bogus and fail the sniff test
            Err(e) => return Err(e),
            Ok(mime) => mime,
        };

        let mut qs: Vec<(&str, Cow<str>)> = vec![("fields", Cow::Borrowed(self.fields))];
        if let Some(pag) = self.pagination {
            if let Some(number) = pag.number {
                qs.push(("page[number]", Cow::Owned(format!("{}", number))));
            }

            // The page size is optional so we don't have to hard code
            // shotgun's *current* default of 500 into the library.
            //
            // If/when shotgun changes their default, folks who haven't
            // specified a page size should get whatever shotgun says, not *our*
            // hard-coded default.
            if let Some(size) = pag.size {
                qs.push(("page[size]", Cow::Owned(format!("{}", size))));
            }
        }

        if let Some(sort) = self.sort {
            qs.push(("sort", Cow::Owned(sort)));
        }

        if let Some(opts) = self.options {
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

        let req = self
            .sg
            .client
            .post(&format!(
                "{}/api/v1/entity/{}/_search",
                self.sg.sg_server, self.entity
            ))
            .query(&qs)
            .header("Accept", "application/json")
            .bearer_auth(self.token)
            .header("Content-Type", content_type)
            // XXX: the content type is being set to shotgun's custom mime types
            //   to indicate the shape of the filter payload. Do not be tempted to use
            //   `.json()` here instead of `.body()` or you'll end up reverting the
            //   header set above.
            .body(json!({"filters": self.filters}).to_string());

        handle_response(req.send().await?).await
    }
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
    ) -> Result<Self> {
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
    async fn authenticate<D: 'static>(&self, form_data: &[(&str, &str)]) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let resp = self
            .client
            .post(&format!("{}/api/v1/auth/access_token", self.sg_server))
            .form(form_data)
            .header("Accept", "application/json")
            .send()
            .await?;
        handle_response(resp).await
    }

    /// Run a credential (human user logging in) challenge.
    pub async fn authenticate_user<D: 'static>(&self, username: &str, password: &str) -> Result<D>
        where
            D: DeserializeOwned,
    {
        self.authenticate(&[
            ("grant_type", "password"),
            ("username", username),
            ("password", password),
        ])
            .await
    }

    /// Get an access token payload for a given Api User aka "script."
    ///
    /// This function relies on the script key and name fields being set and will fail with a
    /// `ShotgunError::BadClientConfig` if either is missing.
    pub async fn authenticate_script<D: 'static>(&self) -> Result<D>
        where
            D: DeserializeOwned,
    {
        if let (Some(script_name), Some(script_key)) =
        (self.script_name.as_ref(), self.script_key.as_ref())
        {
            Ok(self
                .authenticate(&[
                    ("grant_type", "client_credentials"),
                    ("client_id", &script_name),
                    ("client_secret", &script_key),
                ])
                .await?)
        } else {
            Err(ShotgunError::BadClientConfig(
                "Missing script name or key.".into(),
            ))
        }
    }

    /// The same as `authenticate_script()` except it also allows you to pass a username
    /// to "sudo" as.
    ///
    /// This function relies on the script key and name fields being set and will fail with a
    /// `ShotgunError::BadClientConfig` if either is missing.
    pub async fn authenticate_script_as_user<D: 'static>(&self, login: &str) -> Result<D>
        where
            D: DeserializeOwned,
    {
        if let (Some(script_name), Some(script_key)) =
        (self.script_name.as_ref(), self.script_key.as_ref())
        {
            Ok(self
                .authenticate(&[
                    ("grant_type", "client_credentials"),
                    ("client_id", &script_name),
                    ("client_secret", &script_key),
                    ("scope", &format!("sudo_as_login:{}", login)),
                ])
                .await?)
        } else {
            Err(ShotgunError::BadClientConfig(
                "Missing script name or key.".into(),
            ))
        }
    }

    /// Provides the values of a subset of site preferences.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-preferences>
    pub async fn preferences_read<D: 'static>(&self, token: &str) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let req = self
            .client
            .get(&format!("{}/api/v1/preferences", self.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json");
        handle_response(req.send().await?).await
    }

    /// Provides version information about the Shotgun server and the REST API.
    /// Does not require authentication
    pub async fn info<D: 'static>(&self) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let req = self
            .client
            .get(&format!("{}/api/v1/", self.sg_server))
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Read the work day rules for each day specified in the query.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-work-day-rules>
    pub async fn work_days_rules_read<D: 'static>(
        &self,
        token: &str,
        start_date: &str,
        end_date: &str,
        project_id: Option<i32>,
        user_id: Option<i32>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/schedule/work_day_rules?start_date={}&end_date={}",
                self.sg_server, start_date, end_date
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(pid) = project_id {
            req = req.query(&[("project_id", pid)]);
        }

        if let Some(uid) = user_id {
            req = req.query(&[("user_id", uid)])
        }

        handle_response(req.send().await?).await
    }

    pub async fn schema_read<D: 'static>(&self, token: &str, project_id: Option<i32>) -> Result<D>
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
        handle_response(req.send().await?).await
    }

    /// Return schema information for the given entity.
    /// Entity should be a snake cased version of the entity name.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-schema-for-a-single-entity>
    pub async fn schema_entity_read(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
    ) -> Result<SchemaEntityResponse> {
        let mut req = self
            .client
            .get(&format!("{}/api/v1/schema/{}", self.sg_server, entity))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(id) = project_id {
            req = req.query(&[("project_id", id)]);
        }
        handle_response(req.send().await?).await
    }

    /// Return all schema field information for a given entity.
    /// Entity should be a snake cased version of the entity name.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-all-field-schemas-for-an-entity>
    pub async fn schema_fields_read(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
    ) -> Result<SchemaFieldsResponse> {
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
        handle_response(req.send().await?).await
    }

    /// Create a new field on the given entity
    /// <https://developer.shotgunsoftware.com/rest-api/#create-new-field-on-entity>
    pub async fn schema_field_create(
        &self,
        token: &str,
        entity_type: &str,
        data: &CreateFieldRequest,
    ) -> Result<SchemaFieldResponse> {
        let req = self
            .client
            .post(&format!(
                "{}/api/v1/schema/{}/fields",
                self.sg_server, entity_type,
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!(data));

        handle_response(req.send().await?).await
    }

    /// Delete a field on a given entity
    /// <https://developer.shotgunsoftware.com/rest-api/#delete-one-field-from-an-entity>
    pub async fn schema_field_delete(
        &self,
        token: &str,
        entity_type: &str,
        field_name: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/schema/{}/fields/{}",
            self.sg_server, entity_type, field_name
        );
        let req = self
            .client
            .delete(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;

        if req.status().is_success() {
            Ok(())
        } else {
            Err(ShotgunError::Unexpected(format!(
                "Server responded to `DELETE {}` with `{}`",
                &url,
                req.status()
            )))
        }
    }

    /// Revive one field from an entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#revive-one-field-from-an-entity>
    pub async fn schema_field_revive(
        &self,
        token: &str,
        entity_type: &str,
        field_name: &str,
    ) -> Result<()> {
        let url = format!(
            "{}/api/v1/schema/{}/fields/{}?revive=true",
            self.sg_server, entity_type, field_name
        );

        let req = self
            .client
            .post(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;

        if req.status().is_success() {
            Ok(())
        } else {
            Err(ShotgunError::Unexpected(format!(
                "Server responded to `POST {}` with `{}`",
                &url,
                req.status()
            )))
        }
    }

    /// Returns schema information about a specific field on a given entity.
    /// Entity should be a snaked cased version of the entity name.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-one-field-schema-for-an-entity>
    pub async fn schema_field_read(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
        field_name: &str,
    ) -> Result<SchemaFieldResponse> {
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

        handle_response(req.send().await?).await
    }

    /// Update the properties of a field on an entity
    /// <https://developer.shotgunsoftware.com/rest-api/#revive-one-field-from-an-entity>
    pub async fn schema_field_update(
        &self,
        token: &str,
        entity_type: &str,
        field_name: &str,
        data: &UpdateFieldRequest,
    ) -> Result<SchemaFieldResponse> {
        let req = self
            .client
            .put(&format!(
                "{}/api/v1/schema/{}/fields/{}",
                self.sg_server, entity_type, field_name
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!(data));
        handle_response(req.send().await?).await
    }

    /// Provides access to the activity stream of an entity
    /// <https://developer.shotgunsoftware.com/rest-api/#read-entity-activity-stream>
    pub async fn entity_activity_stream_read(
        &self,
        token: &str,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<EntityActivityStreamResponse> {
        let req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/activity_stream",
                self.sg_server, entity_type, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Provides access to the list of users that follow an entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-entity-followers>
    pub async fn entity_followers_read<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        entity_id: i32,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/followers",
                self.sg_server, entity, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");
        handle_response(req.send().await?).await
    }

    /// Allows a user to follow one or more entities
    /// <https://developer.shotgunsoftware.com/rest-api/#follow-an-entity>
    pub async fn entity_follow_update<D: 'static>(
        &self,
        token: &str,
        user_id: i32,
        entities: Vec<EntityIdentifier>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let request = self
            .client
            .post(&format!(
                "{}/api/v1/entity/human_users/{}/follow",
                self.sg_server, user_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!({ "entities": entities }));

        handle_response(request.send().await?).await
    }

    /// Allows a user to unfollow a single entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#unfollow-an-entity>
    pub async fn entity_unfollow_update<D: 'static>(
        &self,
        token: &str,
        user_id: i32,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let request = self
            .client
            .put(&format!(
                "{}/api/v1/entity/{}/{}/unfollow",
                self.sg_server, entity_type, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!({ "user_id": user_id }));

        handle_response(request.send().await?).await
    }

    /// Provides access to the list of entities a user follows.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-user-follows>
    pub async fn user_follows_read<D: 'static>(&self, token: &str, user_id: i32) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/human_users/{}/following",
                self.sg_server, user_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Provides access to the thread content of an entity. Currently only note is supported.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-the-thread-contents-for-a-note>
    pub async fn thread_contents_read<D: 'static>(
        &self,
        token: &str,
        note_id: i32,
        entity_fields: Option<HashMap<String, String>>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/notes/{}/thread_contents",
                self.sg_server, note_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(fields) = entity_fields {
            for (key, value) in fields {
                req = req.query(&[(json!(key), json!(value))]);
            }
        }
        handle_response(req.send().await?).await
    }

    /// Provides access to records related to the current entity record via the entity or multi-entity field.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-record-relationship>
    pub async fn entity_relationship_read<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        entity_id: i32,
        related_field: &str,
        options: Option<OptionsParameter>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/relationships/{}",
                self.sg_server, entity, entity_id, related_field
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");
        if let Some(opts) = options {
            if let Some(val) = opts.include_archived_projects {
                req = req.query(&[("options[include_archived_projects]", val)]);
            }
            if let Some(val) = opts.return_only {
                req = req.query(&[(
                    "options[return_only]",
                    match val {
                        ReturnOnly::Active => "active",
                        ReturnOnly::Retired => "retired",
                    },
                )]);
            }
        }
        handle_response(req.send().await?).await
    }

    /// Provides the information for where an upload should be sent and how to connect the upload
    /// to an entity once it has been uploaded.
    /// <https://developer.shotgunsoftware.com/rest-api/#get-upload-url-for-record>
    pub async fn entity_upload_url_read(
        &self,
        token: &str,
        entity: &str,
        entity_id: i32,
        filename: &str,
        multipart_upload: Option<bool>,
    ) -> Result<UploadInfoResponse> {
        let mut params = vec![("filename", filename)];
        if multipart_upload.unwrap_or(false) {
            params.push(("multipart_upload", "true"));
        }

        let req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/_upload",
                self.sg_server, entity, entity_id
            ))
            .query(&params)
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Provides the information for where an upload should be sent and how to connect the upload
    /// to a field once it has been uploaded.
    /// <https://developer.shotgunsoftware.com/rest-api/#get-upload-url-for-field>
    pub async fn entity_field_upload_url_read<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        entity_id: i32,
        file_name: &str,
        field_name: &str,
        multipart_upload: Option<bool>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let mut params = vec![("filename", file_name)];
        if multipart_upload.unwrap_or(false) {
            params.push(("multipart_upload", "true"));
        }

        let req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/{}/_upload",
                self.sg_server, entity, entity_id, field_name
            ))
            .query(&params)
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Provide access to information about an image or attachment field. You can optionally
    /// use the alt query parameter to download the associated image or attachment (maybe...)
    /// <https://developer.shotgunsoftware.com/rest-api/#read-file-field>
    pub async fn entity_file_field_read(
        &self,
        token: &str,
        entity_type: &str,
        entity_id: i32,
        field_name: &str,
        alt: Option<AltImages>,
        range: Option<String>,
    ) -> Result<FieldHashResponse> {
        let mut req = self
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/{}",
                self.sg_server, entity_type, entity_id, field_name
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(val) = alt {
            req = req.query(&[("alt", val)]);
        }

        if let Some(val) = range {
            req = req.header("Range", &val);
        }

        handle_response(req.send().await?).await
    }

    /// Apparently this is an internal means for interrogating the navigation
    /// system in Shotgun.
    ///
    /// Undocumented in the Python API, in fact the only mention is in the
    /// changelog from years ago:
    /// <https://developer.shotgunsoftware.com/python-api/changelog.html?highlight=hierarchy>
    ///
    /// <https://developer.shotgunsoftware.com/rest-api/#hierarchy-expand>
    pub async fn hierarchy_expand(
        &self,
        token: &str,
        data: HierarchyExpandRequest,
    ) -> Result<HierarchyExpandResponse> {
        let req = self
            .client
            .post(&format!("{}/api/v1/hierarchy/_expand", self.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);
        handle_response(req.send().await?).await
    }

    /// Apparently this is an internal means for interrogating the navigation
    /// system in Shotgun.
    ///
    /// Undocumented in the Python API, in fact the only mention is in the
    /// changelog from years ago:
    /// <https://developer.shotgunsoftware.com/python-api/changelog.html?highlight=hierarchy>
    ///
    /// <https://developer.shotgunsoftware.com/rest-api/#hierarchy-search>
    pub async fn hierarchy_search(
        &self,
        token: &str,
        data: HierarchySearchRequest,
    ) -> Result<HierarchySearchResponse> {
        let req = self
            .client
            .post(&format!("{}/api/v1/hierarchy/_search", self.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);
        handle_response(req.send().await?).await
    }

    /// Batch execute requests
    pub async fn batch(&self, token: &str, data: Value) -> Result<BatchedRequestsResponse> {
        let req = self
            .client
            .post(&format!("{}/api/v1/entity/_batch", self.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);

        handle_response(req.send().await?).await
    }

    /// Create a new entity.
    ///
    /// The `data` field is used the request body, and as such should be an object where the keys
    /// are fields on the entity in question.
    ///
    /// `fields` can be specified to limit the returned fields from the request.
    /// `fields` is an optional comma separated list of field names to return in the response.
    /// Passing `None` will use the default behavior of returning _all fields_.
    pub async fn create<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        data: Value,
        fields: Option<&str>,
    ) -> Result<D>
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
        handle_response(req.send().await?).await
    }

    /// Read the data for a single entity.
    ///
    /// `fields` is an optional comma separated list of field names to return in the response.
    pub async fn read<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        id: i32,
        fields: Option<&str>,
    ) -> Result<D>
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

        handle_response(req.send().await?).await
    }

    /// Modify an existing entity.
    ///
    /// `data` is used as the request body and as such should be an object with keys and values
    /// corresponding to the fields on the given entity.
    pub async fn update<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        id: i32,
        data: Value,
        fields: Option<&str>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let mut req = self
            .client
            .put(&format!(
                "{}/api/v1/entity/{}/{}",
                self.sg_server, entity, id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);

        if let Some(fields) = fields {
            req = req.query(&[("options[fields]", fields)]);
        }

        handle_response(req.send().await?).await
    }

    /// Destroy (delete) an entity.
    pub async fn destroy(&self, token: &str, entity: &str, id: i32) -> Result<()> {
        let url = format!("{}/api/v1/entity/{}/{}", self.sg_server, entity, id,);
        let resp = self
            .client
            .delete(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ShotgunError::Unexpected(format!(
                "Server responded to `DELETE {}` with `{}`",
                &url,
                resp.status()
            )))
        }
    }

    /// Revive an entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#revive-a-record>
    pub async fn revive<D: 'static>(&self, token: &str, entity: &str, entity_id: i32) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let req = self
            .client
            .post(&format!(
                "{}/api/v1/entity/{}/{}?revive=true",
                self.sg_server, entity, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
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
    /// <https://developer.shotgunsoftware.com/rest-api/#searching>
    ///
    pub fn search<'a>(
        &'a self,
        token: &'a str,
        entity: &'a str,
        fields: &'a str,
        filters: &'a Value,
    ) -> Result<SearchBuilder<'a>> {
        Ok(SearchBuilder::new(self, token, entity, fields, filters)?)
    }

    /// Search for entities of the given type(s) and returns a list of basic entity data
    /// that fits the search. Rich filters can be used to narrow down searches to entities
    /// that match the filters.
    ///
    /// > **Important**: performing text searches requires a `HumanUser` and *not
    /// > an `ApiUser`*.
    /// > Either the access token used must belong to a `HumanUser` or must have
    /// > been acquired with the "sudo as" `Shotgun::authenticate_script_as_user()`
    /// > method.
    /// >
    /// > Failing to supply a valid `HumanUser` for this operation will get you
    /// > a 500 response from shotgun, with a 100 "unknown" error code.
    ///
    /// For details on the filter syntax, please refer to the docs:
    ///
    /// <https://developer.shotgunsoftware.com/rest-api/#search-text-entries>
    ///
    pub async fn text_search<D: 'static>(
        &self,
        token: &str,
        filters: &Value,
        pagination: Option<PaginationParameter>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let mut filters = filters.clone();
        {
            if let Some(pagination) = pagination {
                let map = filters.as_object_mut().unwrap();
                map.insert("page".to_string(), json!(pagination));
            }
        }

        let content_type = match get_entity_types_mime(&filters) {
            // early return if the filters are bogus and fail the sniff test
            Err(e) => return Err(e),
            Ok(mime) => mime,
        };

        let req = self
            .client
            .post(&format!("{}/api/v1/entity/_text_search", self.sg_server))
            .header("Content-Type", content_type)
            .header("Accept", "application/json")
            .bearer_auth(token)
            .body(filters.to_string());
        handle_response(req.send().await?).await
    }

    /// Make a summarize request.
    ///
    /// This is similar to the aggregate/grouping mechanism provided by SQL
    /// where you can specify `GROUP BY` and `HAVING` clauses in order to rollup
    /// query results into buckets.
    ///
    /// For more on summary queries, see:
    ///
    /// - <https://developer.shotgunsoftware.com/rest-api/#summarize-field-data>
    /// - <https://developer.shotgunsoftware.com/python-api/reference.html#shotgun_api3.shotgun.Shotgun.summarize>
    pub async fn summarize<D: 'static>(
        &self,
        token: &str,
        entity: &str,
        filters: Option<Value>,
        summary_fields: Option<Vec<SummaryField>>,
        grouping: Option<Vec<Grouping>>,
        options: Option<SummaryOptions>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let content_type = get_filter_mime(filters.as_ref().unwrap_or(&json!([])))?;

        let body = SummarizeRequest {
            filters,
            summary_fields,
            grouping,
            options,
        };

        let req = self
            .client
            .post(&format!(
                "{}/api/v1/entity/{}/_summarize",
                self.sg_server, entity
            ))
            .header("Accept", "application/json")
            .bearer_auth(token)
            .header("Content-Type", content_type)
            // XXX: the content type is being set to shotgun's custom mime types
            //   to indicate the shape of the filter payload. Do not be tempted to use
            //   `.json()` here instead of `.body()` or you'll end up reverting the
            //   header set above.
            .body(json!(body).to_string());
        handle_response(req.send().await?).await
    }

    /// Update the last access time of a project by a user.
    /// <https://developer.shotgunsoftware.com/rest-api/#tocSbatchedrequestsresponse>
    pub async fn project_last_accessed_update(
        &self,
        token: &str,
        project_id: i32,
        user_id: i32,
    ) -> Result<ProjectAccessUpdateResponse> {
        let req = self
            .client
            .put(&format!(
                "{}/api/v1/entity/projects/{}/_update_last_accessed",
                self.sg_server, project_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!({ "user_id": user_id }));

        handle_response(req.send().await?).await
    }

    /// Upload attachments and thumbnails for a given entity.
    ///
    /// The `Shotgun::upload()` method will prepare and return a
    /// `UploadReqBuilder` which can be used to configure some optional aspects
    /// of the process such as linking the upload to tags, or
    /// enabling/configuring multipart support.
    ///
    /// The content of the file to upload can be any type that implements the
    /// [`Read`] trait. This includes [`File`] but also `&[u8]` (aka *a slice
    /// of bytes*).
    ///
    /// > In the Python API, uploading thumbnails is treated as a distinct
    /// > operation from attachments but in the REST API these are treated as the
    /// > same thing.
    /// >
    /// > Thumbnail uploads in this case are handled by specifying `image` as the
    /// > optional `field` parameter.
    ///
    /// # Examples
    ///
    /// Uploading an attachment to a note by setting `field` to None:
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> shotgun_rs::Result<()> {
    /// use shotgun_rs::{Shotgun, TokenResponse};
    /// use std::path::PathBuf;
    /// use std::fs::File;
    ///
    /// let filename = "paranorman-poster.jpg";
    /// let mut file_path = PathBuf::from("/path/to/posters");
    /// file_path.push(filename);
    /// let file = File::open(&file_path)?;
    ///
    /// let sg = Shotgun::new(
    ///     String::from("https://shotgun.example.com"),
    ///     Some("my-shotgun-api-user"),
    ///     Some("**********")
    /// )?;
    ///
    /// let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
    ///
    /// sg.upload(
    ///     &access_token,
    ///     "Note",
    ///     123456,
    ///     // A `None` for the `field` param means this is a attachment upload.
    ///     None,
    ///     &filename,
    ///     file
    /// )
    /// // Non-thumbnail uploads can include some short descriptive text to
    /// // use as the display name (shown in attachment lists, etc).
    /// .display_name(Some(String::from(
    ///     "ParaNorman Poster Art",
    /// )))
    /// .send()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Uploading a thumbnail by specifying the field to upload to as `image`:
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> shotgun_rs::Result<()> {
    /// # use shotgun_rs::{Shotgun, TokenResponse};
    /// # use std::path::PathBuf;
    /// # use std::fs::File;
    /// # let filename = "paranorman-poster.jpg";
    /// # let mut file_path = PathBuf::from("/path/to/posters");
    /// # file_path.push(filename);
    /// # let file = File::open(&file_path)?;
    /// # let sg = Shotgun::new(
    /// #     String::from("https://shotgun.example.com"),
    /// #     Some("my-shotgun-api-user"),
    /// #     Some("**********")
    /// # )?;
    /// # let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
    /// sg.upload(
    ///     &access_token,
    ///     "Asset",
    ///     123456,
    ///     // Setting `field` to "image" means this is a thumbnail upload.
    ///     Some("image"),
    ///     &filename,
    ///     file,
    /// ).send()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Uploading in-memory data instead of using a file on disk:
    ///
    /// ```no_run
    /// # #[tokio::main]
    /// # async fn main() -> shotgun_rs::Result<()> {
    /// # use shotgun_rs::{Shotgun, TokenResponse};
    /// # let sg = Shotgun::new(
    /// #     String::from("https://shotgun.example.com"),
    /// #     Some("my-shotgun-api-user"),
    /// #     Some("**********")
    /// # )?;
    /// # let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
    ///
    /// let movie_script = "
    /// 1. EXT. Mansion On The Hill
    ///
    ///             NARRATOR (V.O.)
    ///     It was a dark and stormy night.
    /// ";
    ///
    /// sg.upload(
    ///     &access_token,
    ///     "Asset",
    ///     123456,
    ///     None,
    ///     "screenplay.txt",
    ///     movie_script.as_bytes(),
    /// )
    /// .display_name(Some(String::from(
    ///     "Spec script for a great new movie.",
    /// )))
    /// .send()
    /// .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Caveats
    ///
    /// ## Multipart Uploads
    ///
    /// Multipart uploads are *only available* if your Shotgun instance is
    /// configured to use *AWS S3 storage*. Setting the `multipart` parameter to
    /// `true` when this not the case will result in a `400` error from Shotgun.
    ///
    /// For the times where *S3 storage is in use* you are **required** to set
    /// `multipart` to `true` for files **500Mb or larger**. For files that are
    /// smaller, you may use multipart *at your discretion*.
    ///
    /// There is currently a bug (**`SG-20292`**) where Shotgun will respond
    /// with a `404` when you attempt to initiate a multipart upload without
    /// also specifying a field name. While it *is legal* to use multipart for
    /// record-level (as opposed to field-level) uploads, it doesn't work today.
    ///
    /// For now, the workaround is to always *specify an appropriate field name*
    /// if you want to use multipart.
    ///
    /// ## Display Name and Tags
    ///
    /// The `display_name` and `tags` parameters are *ignored for thumbnail
    /// uploads*, but are allowed for attachments.
    ///
    /// Also note that `tags` can cause your upload to fail if you supply an
    /// invalid tag id, resulting in a `400` error from Shotgun.
    ///
    /// # See Also:
    ///
    /// - <https://developer.shotgunsoftware.com/python-api/reference.html#shotgun_api3.shotgun.Shotgun.upload>
    /// - <https://developer.shotgunsoftware.com/python-api/reference.html#shotgun_api3.shotgun.Shotgun.upload_thumbnail>
    /// - <https://developer.shotgunsoftware.com/rest-api/#shotgun-rest-api-Uploading-and-Downloading-Files>
    ///
    /// [`File`]: https://doc.rust-lang.org/std/fs/struct.File.html
    /// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
    pub fn upload<'a, R>(
        &'a self,
        token: &'a str,
        entity: &'a str,
        id: i32,
        field: Option<&'a str>,
        filename: &'a str,
        file_content: R,
    ) -> upload::UploadReqBuilder<'a, R>
    where
        R: Read,
    {
        UploadReqBuilder::new(self, token, entity, id, field, filename, file_content)
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
async fn handle_response<D>(resp: Response) -> Result<D>
    where
        D: DeserializeOwned,
{
    let bytes = resp.bytes().await?;
    // There are three (3) potential failure modes here:
    //
    // 1. Connection problems could lead to partial/garbled/non-json payload
    //    resulting in a json parse error. There could also just be no payload for a response, ie 204.
    // 2. The payload could be json, but contain an error message from shotgun about
    //    the filter.
    // 3. The payload might parse as valid json, but the json might not fit the
    //    deserialization target `D`.
    match serde_json::from_slice::<Value>(&bytes) {
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

    #[fail(display = "IO Error - `{}`", _0)]
    IOError(#[fail(cause)] std::io::Error),

    #[fail(display = "Unexpected Error - `{}`", _0)]
    Unexpected(String),

    #[fail(display = "Server Error - `{:?}`", _0)]
    ServerError(Vec<ErrorObject>),

    #[fail(display = "Multipart uploads not supported by storage service.")]
    MultipartNotSupported,

    #[fail(display = "File upload failed - `{}`", _0)]
    UploadError(String),
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

impl From<std::io::Error> for ShotgunError {
    fn from(e: std::io::Error) -> Self {
        ShotgunError::IOError(e)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_entity_types_mime_array_entity_types() {
        let filters = json!({"entity_types":
            {
                "Project": [["is_demo", "is", true]],
                "Asset": [["sg_status", "is", "Hold"]]
            }
        });
        let expected_mime = "application/vnd+shotgun.api3_array+json";
        assert_eq!(get_entity_types_mime(&filters).unwrap(), expected_mime);
    }

    #[test]
    fn test_get_entity_types_mime_object_entity_types() {
        let filters = json!({"entity_types":
            {
                "Project": {"logical_operator": "and", "conditions": [["is_demo", "is", true], ["code", "is", "Foobar"]]},
                "Asset": {"logical_operator": "or", "conditions": [["sg_status", "is", "Hold"], ["code", "is", "FizzBuzz"]]}
            }
        });
        let expected_mime = "application/vnd+shotgun.api3_hash+json";
        assert_eq!(get_entity_types_mime(&filters).unwrap(), expected_mime);
    }

    #[test]
    fn test_get_entity_types_mime_mixed_entity_types_should_fail() {
        let filters = json!({"entity_types":
            {
                "Project": {"logical_operator": "and", "conditions": [["is_demo", "is", true], ["code", "is", "Foobar"]]},
                "Asset": [["sg_status", "is", "Hold"]]
            }
        });

        let result = get_entity_types_mime(&filters);
        match result {
            Err(ShotgunError::InvalidFilters) => assert!(true),
            _ => assert!(false, "Expected ShotgunError::InvalidFilters"),
        }
    }

    #[test]
    fn test_get_invalid_entity_type_should_fail() {
        let filters = json!({"entity_types": ["foobar"]});

        let result = get_entity_types_mime(&filters);
        match result {
            Err(ShotgunError::InvalidFilters) => assert!(true),
            _ => assert!(false, "Expected ShotgunError::InvalidFilters"),
        }
    }
}

#[cfg(test)]
mod mock_tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_login_good_creds() {
        let mock_server = MockServer::start().await;
        let body = r##"
        {
          "token_type": "Bearer",
          "access_token": "$$ACCESS_TOKEN$$",
          "expires_in": 600,
          "refresh_token": "$$REFRESH_TOKEN$$"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;
        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let resp: TokenResponse = sg
            .authenticate_user("nbabcock", "forgot my passwd")
            .await
            .unwrap();

        assert_eq!("$$ACCESS_TOKEN$$", resp.access_token);
        assert_eq!("$$REFRESH_TOKEN$$", resp.refresh_token);
        assert_eq!("Bearer", resp.token_type);
        assert_eq!(600, resp.expires_in);
    }

    #[tokio::test]
    async fn test_login_bad_creds() {
        let mock_server = MockServer::start().await;
        let body = r##"
        {
            "errors": [
               {
                  "id": "xxxxx",
                  "status": 500,
                  "code": 100,
                  "title": "Shotgun Server Error",
                  "source": null,
                  "detail": "Please contact your Shotgun administrator, or contact Shotgun support at: support@shotgunsoftware.com. Please pass on the following information so we can trace what happened: Request: xxxxx Event: .",
                  "meta": null
                }
            ]
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;
        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let resp: Result<TokenResponse> = sg.authenticate_user("nbabcock", "iCdEAD!ppl").await;

        // verify the error response was decoded as expected.
        match resp {
            Err(ShotgunError::ServerError(errors)) => {
                let details = &errors[0];
                assert_eq!("xxxxx", details.id.as_ref().unwrap());
                assert_eq!("Shotgun Server Error", details.title.as_ref().unwrap());
                assert_eq!(500, details.status.unwrap());
                assert_eq!(100, details.code.unwrap());
                assert!(details.source.is_none());
                assert!(details.detail.as_ref().unwrap().contains("Request: xxxxx"));
            }
            _ => unreachable!(),
        }
    }
}
