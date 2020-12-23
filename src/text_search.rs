use crate::filters::FinalizedFilters;
use crate::types::PaginationParameter;
use crate::{handle_response, Session, ShotgunError};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::collections::HashMap;

pub type EntityFilters<'a> = HashMap<&'a str, FinalizedFilters>;

fn get_entity_filters_mime(entity_filters: &EntityFilters) -> crate::Result<&'static str> {
    // If there are no filters at all, the mime doesn't really matter.
    if entity_filters.is_empty() {
        return Ok(crate::filters::MIME_FILTER_ARRAY);
    }

    let mut filters = entity_filters.values();
    if entity_filters.len() > 1 {
        let first = filters.next().unwrap().get_mime();
        for filter in filters {
            if first != filter.get_mime() {
                return Err(ShotgunError::InvalidFilters);
            }
        }
        Ok(first)
    } else {
        Ok(filters.next().unwrap().get_mime())
    }
}

pub struct TextSearchBuilder<'a> {
    session: &'a Session<'a>,
    /// A map of entity type -> filters
    entity_filters: EntityFilters<'a>,
    text: Option<&'a str>,
    sort: Option<String>,
    pagination: Option<PaginationParameter>,
}

impl<'a> TextSearchBuilder<'a> {
    pub fn new(
        session: &'a Session<'a>,
        text: Option<&'a str>,
        entity_filters: EntityFilters<'a>,
    ) -> TextSearchBuilder<'a> {
        TextSearchBuilder {
            session,
            entity_filters,
            text,
            sort: None,
            pagination: None,
        }
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

    pub async fn execute<D: 'static>(self) -> crate::Result<D>
    where
        D: DeserializeOwned,
    {
        let mut body = HashMap::new();

        body.insert("entity_types", json!(self.entity_filters));

        if let Some(text) = self.text {
            body.insert("text", json!(text));
        }
        if let Some(pagination) = self.pagination {
            body.insert("page", json!(pagination));
        }

        if let Some(sort) = self.sort {
            body.insert("sort", json!(sort));
        }

        let content_type = get_entity_filters_mime(&self.entity_filters)?;

        body.insert("entity_filters", json!(self.entity_filters));

        let (sg, token) = self.session.get_sg().await?;
        let req = sg
            .client
            .post(&format!("{}/api/v1/entity/_text_search", sg.sg_server))
            .header("Content-Type", content_type)
            .header("Accept", "application/json")
            .bearer_auth(&token)
            .body(json!(body).to_string());
        handle_response(req.send().await?).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{self, field};

    #[test]
    fn test_get_entity_filters_mime_array_entity_types() {
        let filters = vec![
            ("Project", filters::basic(&[field("is_demo").is(true)])),
            ("Asset", filters::basic(&[field("sg_status").is("Hold")])),
        ]
        .into_iter()
        .collect();

        let expected_mime = "application/vnd+shotgun.api3_array+json";
        assert_eq!(get_entity_filters_mime(&filters).unwrap(), expected_mime);
    }

    #[test]
    fn test_get_entity_filters_mime_object_entity_types() {
        let filters = vec![
            (
                "Project",
                filters::complex(filters::and(&[
                    field("is_demo").is(true),
                    field("code").is("Foobar"),
                ]))
                .unwrap(),
            ),
            (
                "Asset",
                filters::complex(filters::or(&[
                    field("sg_status").is("Hold"),
                    field("code").is("FizzBuzz"),
                ]))
                .unwrap(),
            ),
        ]
        .into_iter()
        .collect();

        let expected_mime = "application/vnd+shotgun.api3_hash+json";
        assert_eq!(get_entity_filters_mime(&filters).unwrap(), expected_mime);
    }

    #[test]
    fn test_get_entity_filters_mime_mixed_entity_types_should_fail() {
        let filters = vec![
            (
                "Project",
                filters::complex(filters::and(&[
                    field("is_demo").is(true),
                    field("code").is("Foobar"),
                ]))
                .unwrap(),
            ),
            ("Asset", filters::basic(&[field("sg_status").is("Hold")])),
        ]
        .into_iter()
        .collect();

        let result = get_entity_filters_mime(&filters);
        match result {
            Err(ShotgunError::InvalidFilters) => assert!(true),
            _ => assert!(false, "Expected ShotgunError::InvalidFilters"),
        }
    }

    #[test]
    fn test_get_entity_filters_mime_empty_filters_ok() {
        let filters = vec![].into_iter().collect();
        assert!(get_entity_filters_mime(&filters).is_ok());
    }
}
