use futures::future::{self, Future};
use futures::Stream;
use reqwest::r#async::Client;
use std::env;
use std::fmt;
use std::fs::File;
use std::io::Read;
#[macro_use]
extern crate serde_derive;
use serde::de::DeserializeOwned;
use serde_json::Value;
#[macro_use]
extern crate failure;
use log::{debug, error};
use std::borrow::Cow;

/// Get a default http client with ca certs added to it if specified via env var.
fn get_client() -> Result<Client, failure::Error> {
    let builder = Client::builder();

    let builder = if let Ok(fp) = env::var("CA_BUNDLE") {
        debug!("Using ca bundle from: `{}`", fp);
        let mut buf = Vec::new();
        File::open(fp)?.read_to_end(&mut buf)?;
        let cert = reqwest::Certificate::from_pem(&buf)?;
        builder.add_root_certificate(cert)
    } else {
        builder
    };

    builder.build().map_err(failure::Error::from)
}

#[allow(dead_code)]
pub enum Filters<'a> {
    // TODO: specialize the value types to object/array?
    Array(&'a Value),
    Hash(&'a Value),
}

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
        script_name: Option<String>,
        script_key: Option<String>,
    ) -> Result<Self, failure::Error> {
        let client = get_client()?;
        Ok(Self {
            sg_server,
            client,
            script_name,
            script_key,
        })
    }

    /// Create a new Shotgun API Client, but configure the HTTP client yourself.
    ///
    /// This may be the option for you if you need to adjust resource limits, or timeouts, etc on
    /// the HTTP client itself.
    pub fn with_client(
        sg_server: String,
        script_name: Option<String>,
        script_key: Option<String>,
        client: Client,
    ) -> Self {
        Self {
            sg_server,
            client,
            script_name,
            script_key,
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
            .and_then(|mut resp| resp.json::<D>())
            .from_err()
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

    /// Create a new entity.
    ///
    /// The `data` field is used the request body, and as such should be an object where the keys
    /// are fields on the entity in question.
    pub fn create<D: 'static>(
        &self,
        token: &str,
        entity: Entity,
        data: Value,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        self.client
            .post(&format!("{}/api/v1/entity/{}", self.sg_server, entity,))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data)
            .send()
            .and_then(|mut resp| resp.json::<D>())
            .from_err()
    }

    /// Read the data for a single entity.
    ///
    /// `fields` is an optional comma separated list of field names to return in the response.
    pub fn read<D: 'static>(
        &self,
        token: &str,
        entity: Entity,
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

        req.send().and_then(|mut resp| resp.json::<D>()).from_err()
    }

    /// Modify an existing entity.
    ///
    /// `data` is used as the request body and as such should be an object with keys and values
    /// corresponding to the fields on the given entity.
    fn update<D: 'static>(
        &self,
        token: &str,
        entity: Entity,
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
            .and_then(|mut resp| resp.json::<D>())
            .from_err()
    }

    /// Destroy (delete) an entity.
    pub fn destroy(
        &self,
        token: &str,
        entity: Entity,
        id: i32,
    ) -> impl Future<Item = (), Error = ShotgunError> {
        self.client
            .delete(&format!(
                "{}/api/v1/entity/{}/{}",
                self.sg_server, entity, id,
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .from_err()
            .and_then(|_| Ok(()))
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
        &self,
        token: &str,
        entity: Entity,
        fields: &str,
        filters: Filters,
        sort: Option<String>,
        page_size: Option<usize>,
        options: Option<OptionsParameter>,
    ) -> impl Future<Item = D, Error = ShotgunError>
    where
        D: DeserializeOwned,
    {
        let (filters, content_type) = match filters {
            Filters::Array(v) => (v, "application/vnd+shotgun.api3_array+json"),
            Filters::Hash(v) => (v, "application/vnd+shotgun.api3_hash+json"),
        };

        let mut qs: Vec<(&str, Cow<str>)> = vec![("fields", Cow::Borrowed(fields))];

        if let Some(sort) = sort {
            qs.push(("sort", Cow::Owned(sort)));
        }
        if let Some(page_size) = page_size {
            qs.push(("page[size]", Cow::Owned(format!("{}", page_size))));
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

        self.client
            .post(&format!(
                "{}/api/v1/entity/{}/_search",
                self.sg_server, entity
            ))
            .header("Content-Type", content_type)
            .header("Accept", "application/json")
            .bearer_auth(token)
            .query(&qs)
            .body(filters.to_string())
            .send()
            .and_then(|resp| resp.into_body().concat2())
            .from_err()
            .and_then(|bytes| {
                let res = serde_json::from_slice::<D>(&bytes).map_err(ShotgunError::from);
                if let Err(e) = &res {
                    error!("Failed to parse payload: `{}` - `{:?}`", e, &bytes);
                }
                res
            })
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Serialize)]
pub enum Entity {
    Asset,
    Group,
    HumanUser,
    Note,
    NoteLink,
    Project,
    Reply,
    Shot,
    Task,
    Version,
    // ... more to come ...
    CustomEntity01,
    CustomEntity02,
    CustomEntity03,
    CustomEntity04,
    CustomEntity05,
    CustomEntity06,
    CustomEntity07,
    CustomEntity08,
    CustomEntity09,
    CustomEntity10,
    CustomEntity11,
    CustomEntity12,
    CustomEntity13,
    CustomEntity14,
    CustomEntity15,
    CustomEntity16,
    CustomEntity17,
    CustomEntity18,
    CustomEntity19,
    CustomEntity20,
    CustomEntity21,
    CustomEntity22,
    CustomEntity23,
    CustomEntity24,
    CustomEntity25,
    CustomEntity26,
    CustomEntity27,
    CustomEntity28,
    CustomEntity29,
    CustomEntity30,
    CustomEntity31,
    CustomEntity32,
    CustomEntity33,
    CustomEntity34,
    CustomEntity35,
    CustomEntity36,
    CustomEntity37,
    CustomEntity38,
    CustomEntity39,
    CustomEntity40,
    CustomEntity41,
    CustomEntity42,
    CustomEntity43,
    CustomEntity44,
    CustomEntity45,
    CustomEntity46,
    CustomEntity47,
    CustomEntity48,
    CustomEntity49,
    CustomEntity50,

    CustomNonProjectEntity01,
    CustomNonProjectEntity02,
    CustomNonProjectEntity03,
    CustomNonProjectEntity04,
    CustomNonProjectEntity05,
    CustomNonProjectEntity06,
    CustomNonProjectEntity07,
    CustomNonProjectEntity08,
    CustomNonProjectEntity09,
    CustomNonProjectEntity10,
    CustomNonProjectEntity11,
    CustomNonProjectEntity12,
    CustomNonProjectEntity13,
    CustomNonProjectEntity14,
    CustomNonProjectEntity15,
    CustomNonProjectEntity16,
    CustomNonProjectEntity17,
    CustomNonProjectEntity18,
    CustomNonProjectEntity19,
    CustomNonProjectEntity20,
    CustomNonProjectEntity21,
    CustomNonProjectEntity22,
    CustomNonProjectEntity23,
    CustomNonProjectEntity24,
    CustomNonProjectEntity25,
    CustomNonProjectEntity26,
    CustomNonProjectEntity27,
    CustomNonProjectEntity28,
    CustomNonProjectEntity29,
    CustomNonProjectEntity30,
    CustomNonProjectEntity31,
    CustomNonProjectEntity32,
    CustomNonProjectEntity33,
    CustomNonProjectEntity34,
    CustomNonProjectEntity35,
    CustomNonProjectEntity36,
    CustomNonProjectEntity37,
    CustomNonProjectEntity38,
    CustomNonProjectEntity39,
    CustomNonProjectEntity40,
    CustomNonProjectEntity41,
    CustomNonProjectEntity42,
    CustomNonProjectEntity43,
    CustomNonProjectEntity44,
    CustomNonProjectEntity45,
    CustomNonProjectEntity46,
    CustomNonProjectEntity47,
    CustomNonProjectEntity48,
    CustomNonProjectEntity49,
    CustomNonProjectEntity50,
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum ReturnOnly {
    Active,
    Retired,
}

pub struct OptionsParameter {
    pub return_only: Option<ReturnOnly>,
    pub include_archived_projects: Option<bool>,
}

#[derive(Fail, Debug)]
pub enum ShotgunError {
    #[fail(display = "Client Configuration Error: `{}`.", _0)]
    BadClientConfig(String),

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

#[derive(Debug, Deserialize, Clone)]
pub struct ShotgunAuthenticationResponse {
    pub access_token: String,
    pub expires_in: i32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_display() {
        assert_eq!(&format!("{}", Entity::HumanUser), "HumanUser");
        assert_eq!(&format!("{}", Entity::Note), "Note");
        assert_eq!(&format!("{}", Entity::Project), "Project");
        assert_eq!(&format!("{}", Entity::Reply), "Reply");
        assert_eq!(&format!("{}", Entity::Shot), "Shot");
    }
}
