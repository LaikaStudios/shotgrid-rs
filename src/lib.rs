//! # Welcome to shotgrid-rs!
//!
//! `shotgrid-rs` is a REST API client for [Autodesk ShotGrid][shotgrid]
//! (formerly _Shotgun_) built with [reqwest] and [serde_json].
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
//! The general pattern of usage starts with a [`Client`].
//!
//! ```no_run
//! # use shotgrid_rs::Client;
//! # #[tokio::main]
//! # async fn main() -> shotgrid_rs::Result<()> {
//! let server = "https://my-shotgrid.example.com";
//! let script_name = "my-api-user";
//! let script_key = "********";
//!
//! let sg = Client::new(server.to_string(), Some(script_name), Some(script_key))?;
//! # Ok(())
//! # }
//! ```
//!
//! Once your client is in hand, you'd use one of the authentication methods to
//! get a [`Session`].
//!
//! ```no_run
//! # use shotgrid_rs::Client;
//! # #[tokio::main]
//! # async fn main() -> shotgrid_rs::Result<()> {
//! #    let server = "https://my-shotgrid.example.com";
//! #    let script_name = "my-api-user";
//! #    let script_key = "********";
//! #    let sg = Client::new(server.to_string(), Some(script_name), Some(script_key))?;
//! // Authenticates using the script name and script key held by the client.
//! let session = sg.authenticate_script().await?;
//! # Ok(())
//! # }
//! ```
//!
//! From there, you can use that [`Session`] to invoke the various query
//! methods, either to use ShotGrid's [rich filter API](`filters`) to find
//! records, or to create/update records.
//!
//! For operations where the schema of the response is *flexible* (based on the
//! entity type and return fields specified), we use generics to allow the
//! caller to unpack the response into the type of their choosing. The type just
//! needs to implement [serde]'s `Deserialize` trait.
//!
//! A number of structs that are generic over types deeper in the data structure
//! are provided.
//! For example, [`ResourceArrayResponse`](`types::ResourceArrayResponse`) is
//! generic over `R` (the resource) which is the items in the array portion of
//! the response, and `L` which is the type for the response's "links" key).
//!
//! ```no_run
//! use serde_derive::Deserialize;
//! use shotgrid_rs::types::{PaginationLinks, ResourceArrayResponse, SelfLink};
//! use shotgrid_rs::Client;
//! use shotgrid_rs::filters;
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
//! async fn main() -> shotgrid_rs::Result<()> {
//!
//!     let server = "https://my-shotgrid.example.com";
//!     let script_name = "my-api-user";
//!     let script_key = "********";
//!
//!     let sg = Client::new(server.to_string(), Some(script_name), Some(script_key))?;
//!
//!     let session = sg.authenticate_script().await?;
//!
//!     let return_fields = ["id", "code", "name"].join(",");
//!
//!     // Using type ascription (or a turbofish), we tell search() how to
//!     // deserialize the response.
//!     let resp: ResourceArrayResponse<Project, PaginationLinks> = session
//!         .search("Project", &return_fields, &filters::empty())
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
//! The `shotgrid_rs` crate offers some logging, though most of it relates to the
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
//! [shotgrid]: https://www.shotgridsoftware.com/
//! [reqwest]: https://crates.io/crates/reqwest
//! [serde]: https://crates.io/crates/serde
//! [serde_json]: https://crates.io/crates/serde_json

use std::env;
use std::fs::File;
use std::io::Read;
#[macro_use]
extern crate serde_derive;
use crate::types::{ErrorObject, ErrorResponse};
use log::{debug, error, trace};
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde_json::Value;
mod entity_relationship_read;
pub mod filters;
mod schema;
mod search;
mod session;
mod summarize;
mod text_search;
pub mod types;
mod upload;
pub use crate::entity_relationship_read::EntityRelationshipReadReqBuilder;
pub use crate::session::Session;
pub use crate::summarize::SummarizeReqBuilder;
pub use search::SearchBuilder;
pub use upload::{UploadReqBuilder, MAX_MULTIPART_CHUNK_SIZE, MIN_MULTIPART_CHUNK_SIZE};

pub type Result<T> = std::result::Result<T, Error>;

pub mod transport {
    /// Re-export to provide access in case callers need to manually configure
    /// the HTTP Client via [`crate::Client::with_transport()`].
    pub use reqwest;
}

type HttpClient = transport::reqwest::Client;

/// Get a default http client with ca certs added to it if specified via env var.
fn get_http_client() -> Result<HttpClient> {
    let builder = HttpClient::builder();

    let builder = if let Ok(fp) = env::var("CA_BUNDLE") {
        debug!("Using ca bundle from: `{}`", fp);
        let mut buf = Vec::new();
        File::open(fp)
            .map_err(|e| Error::BadClientConfig(e.to_string()))?
            .read_to_end(&mut buf)
            .map_err(|e| Error::BadClientConfig(e.to_string()))?;
        let cert = transport::reqwest::Certificate::from_pem(&buf)
            .map_err(|e| Error::BadClientConfig(e.to_string()))?;
        builder.add_root_certificate(cert)
    } else {
        builder
    };
    builder
        .build()
        .map_err(|e| Error::BadClientConfig(e.to_string()))
}
#[derive(Clone, Debug)]
pub struct Client {
    /// Base url for the ShotGrid server.
    sg_server: String,
    /// HTTP Client used internally to make requests to ShotGrid.
    http: HttpClient,
    /// API User (aka "script") name, used to generate API Tokens.
    script_name: Option<String>,
    /// API User (aka "script") secret key, used to generate API Tokens.
    script_key: Option<String>,
}

impl Client {
    /// Create a new ShotGrid API Client using all defaults.
    ///
    /// By default, the HTTP Client initialized while looking to a
    /// `CA_BUNDLE` environment var for a file path to a TLS cert.
    ///
    /// This will `Err` when:
    ///
    /// - `CA_BUNDLE` is set, but the file path it points to is invalid.
    pub fn new(
        sg_server: String,
        script_name: Option<&str>,
        script_key: Option<&str>,
    ) -> Result<Self> {
        let client = get_http_client()?;
        Ok(Self {
            sg_server,
            http: client,
            script_name: script_name.map(Into::into),
            script_key: script_key.map(Into::into),
        })
    }

    /// Create a new ShotGrid API Client, but configure the HTTP client yourself.
    ///
    /// This may be the option for you if you need to adjust resource limits, or
    /// timeouts, etc on the HTTP client itself.
    ///
    /// For your convenience, the [`transport::reqwest`] module has a re-export
    /// of the entire [`reqwest`] crate so you have access to all the types
    /// required for configuring the client.
    pub fn with_transport(
        sg_server: String,
        script_name: Option<&str>,
        script_key: Option<&str>,
        http_client: HttpClient,
    ) -> Self {
        Self {
            sg_server,
            http: http_client,
            script_name: script_name.map(Into::into),
            script_key: script_key.map(Into::into),
        }
    }

    /// Handles running authentication requests.
    async fn authenticate(&self, form_data: &[(&str, &str)]) -> Result<TokenResponse> {
        let resp = self
            .http
            .post(&format!("{}/api/v1/auth/access_token", self.sg_server))
            .form(form_data)
            .header("Accept", "application/json")
            .send()
            .await?;
        handle_response(resp).await
    }

    /// Run a credential (human user logging in) challenge.
    pub async fn authenticate_user(&self, username: &str, password: &str) -> Result<Session<'_>> {
        Ok(Session::new(
            self,
            self.authenticate(&[
                ("grant_type", "password"),
                ("username", username),
                ("password", password),
            ])
            .await?,
        ))
    }

    /// Get an access token payload for a given Api User aka "script."
    ///
    /// This function relies on the script key and name fields being set and
    /// will fail with a [`Error::BadClientConfig`] if either is missing.
    pub async fn authenticate_script(&self) -> Result<Session<'_>> {
        if let (Some(script_name), Some(script_key)) =
            (self.script_name.as_ref(), self.script_key.as_ref())
        {
            Ok(Session::new(
                self,
                self.authenticate(&[
                    ("grant_type", "client_credentials"),
                    ("client_id", script_name),
                    ("client_secret", script_key),
                ])
                .await?,
            ))
        } else {
            Err(Error::BadClientConfig("Missing script name or key.".into()))
        }
    }

    /// The same as `authenticate_script()` except it also allows you to pass a
    /// username to "sudo" as.
    ///
    /// This function relies on the script key and name fields being set and
    /// will fail with a `Error::BadClientConfig` if either is missing.
    pub async fn authenticate_script_as_user(&self, login: &str) -> Result<Session<'_>> {
        if let (Some(script_name), Some(script_key)) =
            (self.script_name.as_ref(), self.script_key.as_ref())
        {
            Ok(Session::new(
                self,
                self.authenticate(&[
                    ("grant_type", "client_credentials"),
                    ("client_id", script_name),
                    ("client_secret", script_key),
                    ("scope", &format!("sudo_as_login:{}", login)),
                ])
                .await?,
            ))
        } else {
            Err(Error::BadClientConfig("Missing script name or key.".into()))
        }
    }

    /// Provides version information about the ShotGrid server.
    ///
    /// Does not require authentication
    pub async fn info<D: 'static>(&self) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let req = self
            .http
            .get(&format!("{}/api/v1/", self.sg_server))
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }
}

/// Checks to see if the `Value` is an object with a top level "errors" key.
fn contains_errors(value: &Value) -> bool {
    value
        .as_object()
        .map(|obj| obj.contains_key("errors"))
        .unwrap_or(false)
}

/// Converts a response body from ShotGrid into something more meaningful.
///
/// There are a handful of ways requests can be fulfilled:
///
/// - Good! _You got a payload that matches your expected shape_.
/// - Bad! _The payload is legit, but doesn't conform to your expectations_.
/// - More Bad! _The request you sent didn't make sense to ShotGrid, so it
///   replied with some error details_.
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
    //    resulting in a json parse error. There could also just be no payload
    //    for a response, ie 204.
    // 2. The payload could be json, but contain an error message from ShotGrid
    //    about the filter.
    // 3. The payload might parse as valid json, but the json might not fit the
    //    deserialization target `D`.
    match serde_json::from_slice::<Value>(&bytes) {
        Err(e) => {
            // case 1 - non-valid json
            error!("Failed to parse payload: `{}` - `{:?}`", e, &bytes);
            // if we can't parse the json at all, bail as-is
            Err(Error::from(e))
        }
        Ok(v) => {
            if contains_errors(&v) {
                trace!("Got error response from ShotGrid:\n{}", &v.to_string());
                // case 2 - server response has error feedback.
                match serde_json::from_value::<ErrorResponse>(v) {
                    Ok(resp) => {
                        let maybe_not_found = resp
                            .errors
                            .iter()
                            .find(|ErrorObject { status, .. }| status == &Some(404));

                        if let Some(ErrorObject { detail, .. }) = maybe_not_found {
                            Err(Error::NotFound(detail.clone().unwrap_or_else(|| "".into())))
                        } else {
                            Err(Error::ServerError(resp.errors))
                        }
                    }
                    // also, a non-valid json/shape sub-case if the response doesn't
                    // look as expected.
                    Err(err) => Err(Error::from(err)),
                }
            } else {
                // case 3 - either we get the shape we want or we get an error
                serde_json::from_value::<D>(v).map_err(Error::from)
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Client Configuration Error: `{0}`.")]
    BadClientConfig(String),

    #[error("Invalid Filters: expected `filters` key to be array or object; was neither.")]
    InvalidFilters,

    #[error("Client Error: `{0}`.")]
    ClientError(#[from] reqwest::Error),

    #[error("JSON Parse Error: `{0}`.")]
    JsonParse(#[from] serde_json::Error),

    #[error("Entity Not Found - `{0}`")]
    NotFound(String),

    #[error("Authentication Failed - `{0}`")]
    Unauthorized(#[source] reqwest::Error),

    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error("Unexpected Error - `{0}`")]
    Unexpected(String),

    #[error("Server Error - `{0:?}`")]
    ServerError(Vec<ErrorObject>),

    #[error("Multipart uploads not supported by storage service.")]
    MultipartNotSupported,

    #[error("File upload failed - `{0}`")]
    UploadError(String),
}

/// Response from ShotGrid after a successful auth challenge.
#[derive(Clone, Debug, Deserialize)]
pub struct TokenResponse {
    pub token_type: String,
    pub access_token: String,
    pub expires_in: i64,
    pub refresh_token: String,
}

#[cfg(doctest)]
mod readme_tests {
    use doc_comment::doctest;
    doctest!("../README.md");
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
        let sg = Client::new(mock_server.uri(), None, None).unwrap();

        let _sess = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_login_bad_creds() {
        let mock_server = MockServer::start().await;
        let body = r##"
        {
            "errors": [
                {
                    "code": 102,
                    "detail": null,
                    "id": "xxxxx",
                    "meta": null,
                    "source": {},
                    "status": 400,
                    "title": "Can't authenticate user 'nbabcock'."
                }
            ]
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(400).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;
        let sg = Client::new(mock_server.uri(), None, None).unwrap();

        let maybe_sess = sg.authenticate_user("nbabcock", "forgot my passwd").await;

        // verify the error response was decoded as expected.
        match maybe_sess {
            Err(Error::ServerError(errors)) => {
                let details = &errors[0];
                assert_eq!("xxxxx", details.id.as_ref().unwrap());
                assert!(details
                    .title
                    .as_ref()
                    .unwrap()
                    .contains("Can't authenticate user"));
                assert_eq!(400, details.status.unwrap());
                assert_eq!(102, details.code.unwrap());
            }
            _ => unreachable!(),
        }
    }
}
