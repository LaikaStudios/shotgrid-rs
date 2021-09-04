//! Sessions track the tokens use to authorize requests.
//!
//! The high-level aim is to give the caller an API that doesn't ever ask for an
//! access token. Instead the session will pass the tokens around for the caller,
//! and refresh it as needed, behind the scenes.
use crate::filters::FinalizedFilters;
use crate::text_search::TextSearchBuilder;
use crate::types::{
    AltImages, BatchedRequestsResponse, CreateFieldRequest, CreateUpdateFieldProperty,
    EntityActivityStreamResponse, EntityIdentifier, FieldDataType, FieldHashResponse,
    HierarchyExpandRequest, HierarchyExpandResponse, HierarchySearchRequest,
    HierarchySearchResponse, ProjectAccessUpdateResponse, SchemaEntityResponse,
    SchemaFieldResponse, SchemaFieldsResponse, SummaryField, UpdateFieldRequest,
    UploadInfoResponse,
};
use crate::{
    handle_response, summarize, upload, EntityRelationshipReadReqBuilder, Error, Result,
    SearchBuilder, SummarizeReqBuilder, UploadReqBuilder,
};
use crate::{Shotgun, TokenResponse};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// Note that since each Session holds refresh tokens *which can only be used once*
// This struct should *not* implement `Clone`.
pub struct Session<'sg> {
    last_refresh: u64,
    tokens: tokio::sync::Mutex<TokenResponse>,
    client: &'sg Shotgun,
}

// To account for time elapsed between the auth request and the
// Session instantiation, we cut the last refresh by an arbitrary
// amount.
// This value will be subtracted from the TTL to shorten it.
const TOKEN_REFRESH_SLOP: u64 = 90;

impl<'sg> Session<'sg> {
    pub(crate) fn new(sg: &'sg Shotgun, initial_auth: TokenResponse) -> Self {
        log::trace!("New session.");
        Self {
            client: sg,
            tokens: tokio::sync::Mutex::new(initial_auth),
            last_refresh: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Get a client/token pair to use to run queries.
    /// Will attempt to refresh the token if it looks ready to expire.
    ///
    /// This is mostly just a stepping stone to bridge session vs pre-session
    /// code.
    pub(crate) async fn get_sg(&self) -> Result<(&Shotgun, String)> {
        if self.token_expiring().await {
            self.refresh_token().await?;
        }
        Ok((self.client, self.tokens.lock().await.access_token.clone()))
    }

    /// Check to see if we should try to refresh early.
    async fn token_expiring(&self) -> bool {
        let ttl = { self.tokens.lock().await.expires_in };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        (now - self.last_refresh) as i64 > ttl - TOKEN_REFRESH_SLOP as i64
    }

    /// `Session` needs to be able to refresh the auth token when:
    /// - the token has expired.
    /// - (optionally) the token is about to expire.
    ///
    /// Refresh tokens can only be *used once* or the refresh request will be
    /// denied.
    /// In light of this, the tokens field has been wrapped in a mutex to try and
    /// restrict concurrent access.
    ///
    /// This has implications for cloning - we may need to add an Arc that can be
    /// cloned so that all clones of a Session share the same mutex.
    async fn refresh_token(&self) -> Result<()> {
        let mut tokens = self.tokens.lock().await;

        *tokens = self
            .client
            .authenticate(&[
                ("grant_type", "refresh"),
                ("refresh_token", &tokens.refresh_token),
            ])
            .await?;

        Ok(())
    }

    /// Batch execute requests
    pub async fn batch(&self, data: Value) -> Result<BatchedRequestsResponse> {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .post(&format!("{}/api/v1/entity/_batch", sg.sg_server))
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
        entity: &str,
        data: Value,
        fields: Option<&str>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .post(&format!("{}/api/v1/entity/{}", sg.sg_server, entity,))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);

        if let Some(fields) = fields {
            req = req.query(&[("options[fields]", fields)]);
        }
        handle_response(req.send().await?).await
    }

    /// Destroy (delete) an entity.
    pub async fn destroy(&self, entity: &str, id: i32) -> Result<()> {
        let (sg, token) = self.get_sg().await?;
        let url = format!("{}/api/v1/entity/{}/{}", sg.sg_server, entity, id,);
        let resp = sg
            .client
            .delete(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(Error::Unexpected(format!(
                "Server responded to `DELETE {}` with `{}`",
                &url,
                resp.status()
            )))
        }
    }

    /// Provides access to the activity stream of an entity
    /// <https://developer.shotgunsoftware.com/rest-api/#read-entity-activity-stream>
    pub async fn entity_activity_stream_read(
        &self,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<EntityActivityStreamResponse> {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/activity_stream",
                sg.sg_server, entity_type, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Provides the information for where an upload should be sent and how to connect the upload
    /// to a field once it has been uploaded.
    /// <https://developer.shotgunsoftware.com/rest-api/#get-upload-url-for-field>
    pub async fn entity_field_upload_url_read<D: 'static>(
        &self,
        entity: &str,
        entity_id: i32,
        file_name: &str,
        field_name: &str,
        multipart_upload: Option<bool>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;

        let mut params = vec![("filename", file_name)];
        if multipart_upload.unwrap_or(false) {
            params.push(("multipart_upload", "true"));
        }

        let req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/{}/_upload",
                sg.sg_server, entity, entity_id, field_name
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
        entity_type: &str,
        entity_id: i32,
        field_name: &str,
        alt: Option<AltImages>,
        range: Option<String>,
    ) -> Result<FieldHashResponse> {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/{}",
                sg.sg_server, entity_type, entity_id, field_name
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

    /// Provides access to the list of users that follow an entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-entity-followers>
    pub async fn entity_followers_read<D: 'static>(&self, entity: &str, entity_id: i32) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/followers",
                sg.sg_server, entity, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");
        handle_response(req.send().await?).await
    }

    /// Allows a user to follow one or more entities
    /// <https://developer.shotgunsoftware.com/rest-api/#follow-an-entity>
    pub async fn entity_follow_update<D: 'static>(
        &self,
        user_id: i32,
        entities: Vec<EntityIdentifier>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let request = sg
            .client
            .post(&format!(
                "{}/api/v1/entity/human_users/{}/follow",
                sg.sg_server, user_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!({ "entities": entities }));

        handle_response(request.send().await?).await
    }

    /// Provides access to records related to the current entity record via the entity or multi-entity field.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-record-relationship>
    pub fn entity_relationship_read<'a>(
        &'a self,
        entity: &'a str,
        entity_id: i32,
        related_field: &'a str,
    ) -> EntityRelationshipReadReqBuilder<'a> {
        EntityRelationshipReadReqBuilder::new(self, entity, entity_id, related_field)
    }

    /// Allows a user to unfollow a single entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#unfollow-an-entity>
    pub async fn entity_unfollow_update<D: 'static>(
        &self,
        user_id: i32,
        entity_type: &str,
        entity_id: i32,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let request = sg
            .client
            .put(&format!(
                "{}/api/v1/entity/{}/{}/unfollow",
                sg.sg_server, entity_type, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!({ "user_id": user_id }));

        handle_response(request.send().await?).await
    }

    /// Provides the information for where an upload should be sent and how to connect the upload
    /// to an entity once it has been uploaded.
    /// <https://developer.shotgunsoftware.com/rest-api/#get-upload-url-for-record>
    pub async fn entity_upload_url_read(
        &self,
        entity: &str,
        entity_id: i32,
        filename: &str,
        multipart_upload: Option<bool>,
    ) -> Result<UploadInfoResponse> {
        let (sg, token) = self.get_sg().await?;
        let mut params = vec![("filename", filename)];
        if multipart_upload.unwrap_or(false) {
            params.push(("multipart_upload", "true"));
        }

        let req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/{}/{}/_upload",
                sg.sg_server, entity, entity_id
            ))
            .query(&params)
            .bearer_auth(token)
            .header("Accept", "application/json");

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
        data: HierarchyExpandRequest, // FIXME: callsite ergo
    ) -> Result<HierarchyExpandResponse> {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .post(&format!("{}/api/v1/hierarchy/_expand", sg.sg_server))
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
        data: HierarchySearchRequest, // FIXME: callsite ergo
    ) -> Result<HierarchySearchResponse> {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .post(&format!("{}/api/v1/hierarchy/_search", sg.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);
        handle_response(req.send().await?).await
    }

    /// Provides the values of a subset of site preferences.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-preferences>
    pub async fn preferences_read<D: 'static>(&self) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .get(&format!("{}/api/v1/preferences", sg.sg_server))
            .bearer_auth(token)
            .header("Accept", "application/json");
        handle_response(req.send().await?).await
    }

    /// Update the last access time of a project by a user.
    /// <https://developer.shotgunsoftware.com/rest-api/#tocSbatchedrequestsresponse>
    pub async fn project_last_accessed_update(
        &self,
        project_id: i32,
        user_id: i32,
    ) -> Result<ProjectAccessUpdateResponse> {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .put(&format!(
                "{}/api/v1/entity/projects/{}/_update_last_accessed",
                sg.sg_server, project_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&json!({ "user_id": user_id }));

        handle_response(req.send().await?).await
    }

    /// Read the data for a single entity.
    ///
    /// `fields` is an optional comma separated list of field names to return in the response.
    pub async fn read<D: 'static>(&self, entity: &str, id: i32, fields: Option<&str>) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!("{}/api/v1/entity/{}/{}", sg.sg_server, entity, id))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(fields) = fields {
            req = req.query(&[("fields", fields)]);
        }

        handle_response(req.send().await?).await
    }
    /// Revive an entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#revive-a-record>
    pub async fn revive<D: 'static>(&self, entity: &str, entity_id: i32) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .post(&format!(
                "{}/api/v1/entity/{}/{}?revive=true",
                sg.sg_server, entity, entity_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    pub async fn schema_read<D: 'static>(&self, project_id: Option<i32>) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!("{}/api/v1/schema", sg.sg_server))
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
        project_id: Option<i32>,
        entity: &str,
    ) -> Result<SchemaEntityResponse> {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!("{}/api/v1/schema/{}", sg.sg_server, entity))
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

        project_id: Option<i32>,
        entity: &str,
    ) -> Result<SchemaFieldsResponse> {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!("{}/api/v1/schema/{}/fields", sg.sg_server, entity))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(id) = project_id {
            req = req.query(&[("project_id", id)]);
        }
        handle_response(req.send().await?).await
    }

    /// Create a new field on the given entity
    /// <https://developer.shotgunsoftware.com/rest-api/#create-new-field-on-entity>
    pub async fn schema_field_create<P>(
        &self,
        entity_type: &str,
        data_type: FieldDataType,
        properties: Vec<P>,
    ) -> Result<SchemaFieldResponse>
    where
        P: Into<CreateUpdateFieldProperty>,
    {
        let (sg, token) = self.get_sg().await?;
        let body = CreateFieldRequest {
            data_type,
            properties: properties.into_iter().map(Into::into).collect(),
        };
        let req = sg
            .client
            .post(&format!(
                "{}/api/v1/schema/{}/fields",
                sg.sg_server, entity_type,
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&body);

        handle_response(req.send().await?).await
    }

    /// Delete a field on a given entity
    /// <https://developer.shotgunsoftware.com/rest-api/#delete-one-field-from-an-entity>
    pub async fn schema_field_delete(&self, entity_type: &str, field_name: &str) -> Result<()> {
        let (sg, token) = self.get_sg().await?;
        let url = format!(
            "{}/api/v1/schema/{}/fields/{}",
            sg.sg_server, entity_type, field_name
        );
        let req = sg
            .client
            .delete(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;

        if req.status().is_success() {
            Ok(())
        } else {
            Err(Error::Unexpected(format!(
                "Server responded to `DELETE {}` with `{}`",
                &url,
                req.status()
            )))
        }
    }

    /// Revive one field from an entity.
    /// <https://developer.shotgunsoftware.com/rest-api/#revive-one-field-from-an-entity>
    pub async fn schema_field_revive(&self, entity_type: &str, field_name: &str) -> Result<()> {
        let (sg, token) = self.get_sg().await?;
        let url = format!(
            "{}/api/v1/schema/{}/fields/{}?revive=true",
            sg.sg_server, entity_type, field_name
        );

        let req = sg
            .client
            .post(&url)
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await?;

        if req.status().is_success() {
            Ok(())
        } else {
            Err(Error::Unexpected(format!(
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
        project_id: Option<i32>,
        entity: &str,
        field_name: &str,
    ) -> Result<SchemaFieldResponse> {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!(
                "{}/api/v1/schema/{}/fields/{}",
                sg.sg_server, entity, field_name,
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
    pub async fn schema_field_update<P>(
        &self,
        entity_type: &str,
        field_name: &str,
        properties: Vec<P>,
        project_id: Option<i32>,
    ) -> Result<SchemaFieldResponse>
    where
        P: Into<CreateUpdateFieldProperty>,
    {
        let (sg, token) = self.get_sg().await?;
        let body = UpdateFieldRequest {
            properties: properties.into_iter().map(Into::into).collect(),
            project_id,
        };
        let req = sg
            .client
            .put(&format!(
                "{}/api/v1/schema/{}/fields/{}",
                sg.sg_server, entity_type, field_name
            ))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&body);
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
    pub fn search<'a>(
        &'a self,
        entity: &'a str,
        fields: &'a str,
        filters: &'a FinalizedFilters,
    ) -> SearchBuilder<'a> {
        // FIXME: should return builder, not result
        //  The terminal method can do any needed validation.
        SearchBuilder::new(self, entity, fields, filters)
    }

    /// Make a summarize request.
    ///
    /// This is similar to the aggregate/grouping mechanism provided by SQL
    /// where you can specify `GROUP BY` and `HAVING` clauses in order to rollup
    /// query results into buckets.
    ///
    /// ```no_run
    /// use shotgun_rs::{Shotgun, TokenResponse};
    /// use shotgun_rs::types::{ResourceArrayResponse, SelfLink};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> shotgun_rs::Result<()> {
    /// use shotgun_rs::types::SummaryFieldType;
    /// use shotgun_rs::filters::{self, field, EntityRef};
    ///
    /// let server = String::from("https://shotgun.example.com");
    /// let sg = Shotgun::new(server, Some("my-api-user"), Some("********"))?;
    /// let sess = sg.authenticate_script_as_user("nbabcock").await?;
    ///
    /// let filters = filters::basic(&[
    ///     field("project").is(EntityRef::new("Project", 4))
    /// ]);
    /// let summary_fields = vec![("id", SummaryFieldType::Count).into()];
    ///
    /// let summary = sess
    ///     .summarize("Asset", Some(filters), summary_fields)
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// For more on summary queries, see:
    ///
    /// - <https://developer.shotgunsoftware.com/rest-api/#summarize-field-data>
    /// - <https://developer.shotgunsoftware.com/python-api/reference.html#shotgun_api3.shotgun.Shotgun.summarize>
    pub fn summarize<'a>(
        &'a self,
        entity: &'a str,
        // FIXME: python api treats filters as required (and we fallback to empty array).
        //  Maybe just make it required?
        filters: Option<FinalizedFilters>,
        summary_fields: Vec<SummaryField>,
    ) -> SummarizeReqBuilder<'a> {
        summarize::SummarizeReqBuilder::new(self, entity, filters, summary_fields)
    }

    /// Search for entities of the given type(s) and returns a list of *basic* entity data
    /// that fits the search. Rich filters can be used to narrow down searches to entities
    /// that match the filters.
    ///
    /// The `Shotgun::text_search()` method will prepare and return a
    /// `TextSearchBuilder` which can be used to configure some optional aspects
    /// of the process such as setting pagination parameters or sort order.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use serde_json::Value;
    /// use shotgun_rs::{Shotgun, TokenResponse};
    /// use shotgun_rs::types::{ResourceArrayResponse, SelfLink};
    /// use shotgun_rs::filters::{self, field};
    ///
    /// # #[tokio::main]
    /// # async fn main() -> shotgun_rs::Result<()> {
    /// let server = String::from("https://shotgun.example.com");
    /// let sg = Shotgun::new(server, Some("my-api-user"), Some("********"))?;
    /// let sess = sg.authenticate_script_as_user("nbabcock").await?;
    ///
    /// let entity_filters = vec![
    ///     ("Asset", filters::basic(&[field("sg_status_list").is_not("omt")]))
    /// ]
    /// .into_iter()
    /// .collect();
    ///
    /// let resp: ResourceArrayResponse<Value, SelfLink> = sess
    ///     .text_search(Some("Mr. Penderghast"), entity_filters)
    ///     .size(Some(5))
    ///     .execute()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Caveats
    ///
    /// > **Important**: performing text searches requires a `HumanUser` and *not
    /// > an `ApiUser`*.
    /// > Either the access token used must belong to a `HumanUser` or must have
    /// > been acquired with the "sudo as" `Shotgun::authenticate_script_as_user()`
    /// > method.
    /// >
    /// > Failing to supply a valid `HumanUser` for this operation will get you
    /// > a `500` response from shotgun, with a 100 "unknown" error code.
    ///
    /// For details on the filter syntax, please refer to the docs:
    ///
    /// <https://developer.shotgunsoftware.com/rest-api/#search-text-entries>
    pub fn text_search<'a>(
        &'a self,
        text: Option<&'a str>,
        entity_filters: HashMap<&'a str, FinalizedFilters>,
    ) -> TextSearchBuilder<'a> {
        TextSearchBuilder::new(self, text, entity_filters)
    }

    /// Provides access to the thread content of an entity. Currently only note is supported.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-the-thread-contents-for-a-note>
    pub async fn thread_contents_read<D: 'static>(
        &self,
        note_id: i32,
        entity_fields: Option<HashMap<String, String>>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/notes/{}/thread_contents",
                sg.sg_server, note_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        if let Some(fields) = entity_fields {
            for (key, value) in fields {
                req = req.query(&[(json!(key), json!(value))]); // FIXME: should not be jsonified.
            }
        }
        handle_response(req.send().await?).await
    }

    /// Modify an existing entity.
    ///
    /// `data` is used as the request body and as such should be an object with keys and values
    /// corresponding to the fields on the given entity.
    pub async fn update<D: 'static>(
        &self,
        entity: &str,
        id: i32,
        data: Value,
        fields: Option<&str>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .put(&format!("{}/api/v1/entity/{}/{}", sg.sg_server, entity, id))
            .bearer_auth(token)
            .header("Accept", "application/json")
            .json(&data);

        if let Some(fields) = fields {
            req = req.query(&[("options[fields]", fields)]);
        }

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
    /// let session = sg.authenticate_script().await?;
    ///
    /// session.upload(
    ///     "Note",
    ///     123456,
    ///     // A `None` for the `field` param means this is a attachment upload.
    ///     None,
    ///     &filename,
    /// )
    /// // Non-thumbnail uploads can include some short descriptive text to
    /// // use as the display name (shown in attachment lists, etc).
    /// .display_name(Some(String::from(
    ///     "ParaNorman Poster Art",
    /// )))
    /// .send(file)
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
    /// # let session = sg.authenticate_script().await?;
    /// session.upload(
    ///     "Asset",
    ///     123456,
    ///     // Setting `field` to "image" means this is a thumbnail upload.
    ///     Some("image"),
    ///     &filename,
    /// )
    /// .send(file)
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
    /// # let session = sg.authenticate_script().await?;
    ///
    /// let movie_script = "
    /// 1. EXT. Mansion On The Hill
    ///
    ///             NARRATOR (V.O.)
    ///     It was a dark and stormy night.
    /// ";
    ///
    /// session.upload(
    ///     "Asset",
    ///     123456,
    ///     None,
    ///     "screenplay.txt",
    /// )
    /// .display_name(Some(String::from(
    ///     "Spec script for a great new movie.",
    /// )))
    /// .send(movie_script.as_bytes())
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
    pub fn upload<'a>(
        &'a self,
        entity: &'a str,
        id: i32,
        field: Option<&'a str>,
        filename: &'a str,
    ) -> upload::UploadReqBuilder<'a> {
        UploadReqBuilder::new(self, entity, id, field, filename)
    }

    /// Provides access to the list of entities a user follows.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-user-follows>
    pub async fn user_follows_read<D: 'static>(&self, user_id: i32) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let req = sg
            .client
            .get(&format!(
                "{}/api/v1/entity/human_users/{}/following",
                sg.sg_server, user_id
            ))
            .bearer_auth(token)
            .header("Accept", "application/json");

        handle_response(req.send().await?).await
    }

    /// Read the work day rules for each day specified in the query.
    /// <https://developer.shotgunsoftware.com/rest-api/#read-work-day-rules>
    pub async fn work_days_rules_read<D: 'static>(
        &self,
        start_date: &str,
        end_date: &str,
        project_id: Option<i32>,
        user_id: Option<i32>,
    ) -> Result<D>
    where
        D: DeserializeOwned,
    {
        let (sg, token) = self.get_sg().await?;
        let mut req = sg
            .client
            .get(&format!("{}/api/v1/schedule/work_day_rules", sg.sg_server))
            .query(&[("start_date", start_date), ("end_date", end_date)])
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
}

#[cfg(test)]
mod mock_tests {

    /// Status 401 response for requests that let their token expire.
    #[allow(unused)]
    const TOKEN_EXPIRED: &str = r##"
    {
        "errors": [
            {
                "code": 102,
                "detail": "Token Expired",
                "id": "xxxxx",
                "meta": null,
                "source": null,
                "status": 401,
                "title": "Unauthorized"
            }
        ]
    }
    "##;

    /// Status 401 response for auth requests using a spent refresh token (and
    /// I'm guessing generally invalid/wrong tokens).
    #[allow(unused)]
    const TOKEN_INVALID: &str = r##"
    {
        "errors": [
            {
                "code": 102,
                "detail": "Token invalid",
                "id": "xxxxx",
                "meta": null,
                "source": null,
                "status": 401,
                "title": "Unauthorized"
            }
        ]
    }"##;

    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_session_can_estimate_expiry_bigger_than_slop() {
        let mock_server = MockServer::start().await;

        // Expiry is set to SLOP + 5, so the token should not be expiring immediately.
        let body = r##"
        {
          "token_type": "Bearer",
          "access_token": "$$ACCESS_TOKEN$$",
          "expires_in": 95,
          "refresh_token": "$$REFRESH_TOKEN$$"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let session = sg
            .authenticate_user("nbabcock", "forgot my passwd")
            .await
            .unwrap();

        assert_eq!(false, session.token_expiring().await);
    }

    #[tokio::test]
    async fn test_session_can_estimate_expiry_smaller_than_slop() {
        let mock_server = MockServer::start().await;

        // Expiry is set to SLOP - 5, so the token should be expiring immediately.
        let body = r##"
        {
          "token_type": "Bearer",
          "access_token": "$$ACCESS_TOKEN$$",
          "expires_in": 85,
          "refresh_token": "$$REFRESH_TOKEN$$"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let session = sg
            .authenticate_user("nbabcock", "forgot my passwd")
            .await
            .unwrap();

        assert_eq!(true, session.token_expiring().await);
    }

    #[tokio::test]
    async fn test_session_can_estimate_negative_expiry() {
        let mock_server = MockServer::start().await;

        // Expiry is set to -5 should be considered 0, expiring immediately.
        let body = r##"
        {
          "token_type": "Bearer",
          "access_token": "$$ACCESS_TOKEN$$",
          "expires_in": -5,
          "refresh_token": "$$REFRESH_TOKEN$$"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let session = sg
            .authenticate_user("nbabcock", "forgot my passwd")
            .await
            .unwrap();

        assert_eq!(true, session.token_expiring().await);
    }

    #[tokio::test]
    async fn test_session_can_estimate_zero_expiry() {
        let mock_server = MockServer::start().await;

        // Expiry is set to 0 should be expiring immediately.
        let body = r##"
        {
          "token_type": "Bearer",
          "access_token": "$$ACCESS_TOKEN$$",
          "expires_in": 0,
          "refresh_token": "$$REFRESH_TOKEN$$"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let session = sg
            .authenticate_user("nbabcock", "forgot my passwd")
            .await
            .unwrap();

        assert_eq!(true, session.token_expiring().await);
    }
}
