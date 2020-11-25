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
//! [native-tls crate]: https://crates.io/crates/native-tls
//! [rustls crate]: https://crates.io/crates/rustls
//! [shotgun]: https://www.shotgunsoftware.com/

use std::env;
use std::fs::File;
use std::io::Read;
#[macro_use]
extern crate serde_derive;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
#[macro_use]
extern crate failure;
use crate::types::UploadInfoResponse;
use log::{debug, error, trace};
use reqwest::Response;
/// This client represents the the http transport layer used by `Shotgun`.
///
/// Should you need to manually configure your client, you can do so then
/// initialize your Shotgun instance via `Shotgun::with_client()`.
pub use reqwest::{Certificate, Client};
use std::borrow::Cow;
pub use upload::{UploadReqBuilder, MAX_MULTIPART_CHUNK_SIZE, MIN_MULTIPART_CHUNK_SIZE};

pub mod types;
mod upload;

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

    /// Return all schema field information for a given entity.
    /// Entity should be a snake cased version of the entity name.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-all-field-schemas-for-an-entity>
    pub async fn schema_fields_read<D: 'static>(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
    ) -> Result<D>
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
        handle_response(req.send().await?).await
    }

    /// Returns schema information about a specific field on a given entity.
    /// Entity should be a snaked cased version of the entity name.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-one-field-schema-for-an-entity>
    pub async fn schema_field_read<D: 'static>(
        &self,
        token: &str,
        project_id: Option<i32>,
        entity: &str,
        field_name: &str,
    ) -> Result<D>
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

        handle_response(req.send().await?).await
    }

    /// Batch execute requests
    pub async fn batch<D: 'static>(&self, token: &str, data: Value) -> Result<D>
        where
            D: DeserializeOwned,
    {
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
    pub async fn search<D: 'static>(
        // FIXME: many parameters here can often be ignored. Switch to builder pattern.
        &self,
        token: &str,
        entity: &str,
        fields: &str,
        filters: &Value,
        sort: Option<String>,
        pagination: Option<PaginationParameter>,
        options: Option<OptionsParameter>,
    ) -> Result<D>
        where
            D: DeserializeOwned,
    {
        let pagination = pagination
            .or_else(|| Some(PaginationParameter::default()))
            .unwrap();

        let content_type = match get_filter_mime(&filters["filters"]) {
            // early return if the filters are bogus and fail the sniff test
            Err(e) => return Err(e),
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

        let req = self
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
            .body(filters.to_string());

        handle_response(req.send().await?).await
    }

    /// Search for entities of the given type(s) and returns a list of basic entity data
    /// that fits the search. Rich filters can be used to narrow down searches to entities
    /// that match the filters.
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
        let pagination = pagination
            .or_else(|| Some(PaginationParameter::default()))
            .unwrap();

        //
        let mut filters = filters.clone();
        {
            let map = filters.as_object_mut().unwrap();
            map.insert("page".to_string(), json!(pagination));
        }

        let req = self
            .client
            .post(&format!("{}/api/v1/entity/_text_search", self.sg_server))
            .header("Content-Type", "application/vnd+shotgun.api3_array+json")
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

/// Request body of a summarize query.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SummarizeRequest {
    /// Filters used to perform the initial search for things you will be
    /// aggregating.
    #[serde(skip_serializing_if = "Option::is_none")]
    filters: Option<Value>,

    /// Summary fields represent the calculated values produced per
    /// grouping.
    #[serde(skip_serializing_if = "Option::is_none")]
    summary_fields: Option<Vec<SummaryField>>,

    /// Groupings for aggregate operations. These are what you are
    /// _aggregating by_.
    #[serde(skip_serializing_if = "Option::is_none")]
    grouping: Option<Vec<Grouping>>,

    /// Options for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<SummaryOptions>,
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

/// Direction to order a summary grouping.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GroupingDirection {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

/// A summary field consists of a concrete field on an entity and a summary
/// operation to use to aggregate it as part of a summary request.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SummaryField {
    pub field: String,
    pub r#type: SummaryFieldType,
}

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

/// Options for a summary request.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SummaryOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_archived_projects: Option<bool>,
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
    //    resulting in a json parse error.
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
