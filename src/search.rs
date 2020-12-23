use crate::filters::FinalizedFilters;
use crate::types::{OptionsParameter, PaginationParameter, ReturnOnly};
use crate::Session;
use serde::de::DeserializeOwned;
use serde_json::json;
use std::borrow::Cow;

pub struct SearchBuilder<'a> {
    session: &'a Session<'a>,
    entity: &'a str,
    fields: &'a str,
    filters: &'a FinalizedFilters,
    sort: Option<String>,
    pagination: Option<PaginationParameter>,
    options: Option<OptionsParameter>,
}

impl<'a> SearchBuilder<'a> {
    pub fn new(
        session: &'a Session<'a>,
        entity: &'a str,
        fields: &'a str,
        filters: &'a FinalizedFilters,
    ) -> SearchBuilder<'a> {
        SearchBuilder {
            session,
            entity,
            fields,
            filters,
            sort: None,
            pagination: None,
            options: None,
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

    pub async fn execute<D: 'static>(self) -> crate::Result<D>
    where
        D: DeserializeOwned,
    {
        let mut query: Vec<(&str, Cow<str>)> = vec![("fields", Cow::Borrowed(self.fields))];
        if let Some(pag) = self.pagination {
            if let Some(number) = pag.number {
                query.push(("page[number]", Cow::Owned(format!("{}", number))));
            }

            // The page size is optional so we don't have to hard code
            // shotgun's *current* default of 500 into the library.
            //
            // If/when shotgun changes their default, folks who haven't
            // specified a page size should get whatever shotgun says, not *our*
            // hard-coded default.
            if let Some(size) = pag.size {
                query.push(("page[size]", Cow::Owned(format!("{}", size))));
            }
        }

        if let Some(sort) = self.sort {
            query.push(("sort", Cow::Owned(sort)));
        }

        if let Some(opts) = self.options {
            if let Some(return_only) = opts.return_only {
                query.push((
                    "options[return_only]",
                    Cow::Borrowed(match return_only {
                        ReturnOnly::Active => "active",
                        ReturnOnly::Retired => "retired",
                    }),
                ));
            }
            if let Some(include_archived_projects) = opts.include_archived_projects {
                query.push((
                    "options[include_archived_projects]",
                    Cow::Owned(format!("{}", include_archived_projects)),
                ));
            }
        }
        let (sg, token) = self.session.get_sg().await?;
        let req = sg
            .client
            .post(&format!(
                "{}/api/v1/entity/{}/_search",
                sg.sg_server, self.entity
            ))
            .query(&query)
            .header("Accept", "application/json")
            .bearer_auth(&token)
            .header("Content-Type", self.filters.get_mime())
            // XXX: the content type is being set to shotgun's custom mime types
            //   to indicate the shape of the filter payload. Do not be tempted to use
            //   `.json()` here instead of `.body()` or you'll end up reverting the
            //   header set above.
            .body(json!({"filters": self.filters}).to_string());

        crate::handle_response(req.send().await?).await
    }
}
