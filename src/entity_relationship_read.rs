use crate::types::{OptionsParameter, ReturnOnly};
use crate::{handle_response, Result, Session};
use serde::de::DeserializeOwned;

pub struct EntityRelationshipReadReqBuilder<'a> {
    session: &'a Session<'a>,
    entity: &'a str,
    entity_id: i32,
    related_field: &'a str,
    options: OptionsParameter,
}

impl<'a> EntityRelationshipReadReqBuilder<'a> {
    pub fn new(
        session: &'a Session<'a>,
        entity: &'a str,
        entity_id: i32,
        related_field: &'a str,
    ) -> Self {
        Self {
            session,
            entity,
            entity_id,
            related_field,
            options: OptionsParameter::default(),
        }
    }

    pub fn return_only(mut self, value: Option<ReturnOnly>) -> Self {
        self.options.return_only = value;
        self
    }

    pub fn include_archived_projects(mut self, value: Option<bool>) -> Self {
        self.options.include_archived_projects = value;
        self
    }

    pub async fn execute<D>(self) -> Result<D>
    where
        D: DeserializeOwned + 'static,
    {
        let (sg, token) = self.session.get_sg().await?;
        let mut req = sg
            .http
            .get(&format!(
                "{}/api/v1/entity/{}/{}/relationships/{}",
                sg.sg_server, self.entity, self.entity_id, self.related_field
            ))
            .bearer_auth(&token)
            .header("Accept", "application/json");
        if let Some(val) = self.options.include_archived_projects {
            req = req.query(&[("options[include_archived_projects]", val)]);
        }
        if let Some(val) = self.options.return_only {
            req = req.query(&[(
                "options[return_only]",
                match val {
                    ReturnOnly::Active => "active",
                    ReturnOnly::Retired => "retired",
                },
            )]);
        }
        handle_response(req.send().await?).await
    }
}
