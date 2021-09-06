#![cfg(feature = "integration-tests")]
//! The unfortunate thing for these tests that target an actual ShotGrid server
//! is we can't really assert anything about the records in the database unless
//! we create those records in the test itself (which we'd rather not do).
//!
//! At least the initial focus will to just assert that the response we get is
//! `Ok` vs `Err` and ignore the actual response bodies. Also, we'll avoid writes
//! for now.
//!
//! These tests depend on several env vars being set.
//!
//! - `TEST_SG_SERVER`, the ShotGrid server to connect to.
//! - `TEST_SG_SCRIPT_NAME`, the name of an ApiUser to connect as.
//! - `TEST_SG_SCRIPT_KEY`, the API key to go with the name.
//! - `TEST_SG_HUMAN_USER_LOGIN`, certain tests require a HumanUser so this is
//!   the login to "sudo as" for those tests.
//! - `TEST_SG_PROJECT_ID`, some tests require a project to filter by.

use serde_json::{json, Value};
use shotgrid_rs::filters::{self, field, EntityRef};
use shotgrid_rs::types::{
    Entity, GroupingDirection, GroupingType, HierarchyEntityFields, HierarchyExpandRequest,
    HierarchySearchCriteria, HierarchySearchRequest, SummaryFieldType,
};

mod helpers;

#[tokio::test]
async fn e2e_test_info_read() {
    let sg = helpers::get_test_client();
    sg.info::<Value>().await.unwrap();
}

#[tokio::test]
async fn e2e_test_preferences_read() {
    let sg = helpers::get_test_client();
    let session = sg.authenticate_script().await.expect("ApiUser auth");
    session.preferences_read::<Value>().await.unwrap();
}

#[tokio::test]
async fn e2e_test_list_projects() {
    let sg = helpers::get_test_client();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session
        .search(
            "Project",
            &["id", "code", "name"].join(","),
            &filters::empty(),
        )
        .size(Some(3))
        .number(Some(1))
        .execute::<Value>()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_schema_entity_read() {
    let sg = helpers::get_test_client();
    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session.schema_entity_read(None, "Asset").await.unwrap();
}
#[tokio::test]
async fn e2e_test_schema_entity_read_for_project() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();
    let session = sg.authenticate_script().await.expect("ApiUser auth");
    session
        .schema_entity_read(Some(project_id), "Asset")
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_schema_read() {
    let sg = helpers::get_test_client();
    let session = sg.authenticate_script().await.expect("ApiUser auth");
    session.schema_read::<Value>(None).await.unwrap();
}

#[tokio::test]
async fn e2e_test_schema_read_for_project() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();
    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session
        .schema_read::<Value>(Some(project_id))
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_summarize_project_assets() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session
        .summarize(
            "Asset",
            Some(filters::basic(&[
                field("project").is(EntityRef::new("Project", project_id))
            ])),
            vec![("id", SummaryFieldType::Count).into()],
        )
        .grouping(Some(vec![(
            "sg_asset_type",
            GroupingType::Exact,
            GroupingDirection::Asc,
        )
            .into()]))
        .execute()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_summarize_no_filters() {
    let sg = helpers::get_test_client();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session
        .summarize("Asset", None, vec![("id", SummaryFieldType::Count).into()])
        .grouping(Some(vec![(
            "sg_asset_type",
            GroupingType::Exact,
            GroupingDirection::Asc,
        )
            .into()]))
        .execute()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_summarize_empty_summary_fields() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session
        .summarize(
            "Asset",
            Some(filters::basic(&[
                field("project").is(EntityRef::new("Project", project_id))
            ])),
            vec![],
        )
        .grouping(Some(vec![(
            "sg_asset_type",
            GroupingType::Exact,
            GroupingDirection::Asc,
        )
            .into()]))
        .execute()
        .await
        .unwrap();
}
#[tokio::test]
async fn e2e_test_summarize_no_groupings() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    session
        .summarize(
            "Asset",
            Some(filters::basic(&[
                field("project").is(EntityRef::new("Project", project_id))
            ])),
            vec![("id", SummaryFieldType::Count).into()],
        )
        .execute()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_read_user_follows() {
    let sg = helpers::get_test_client();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    let user_id = helpers::get_api_user_id(&session).await;

    session.user_follows_read::<Value>(user_id).await.unwrap();
}

#[tokio::test]
async fn e2e_test_text_search() {
    let login = helpers::get_human_user_login();
    let sg = helpers::get_test_client();
    let session = sg
        // Text search only works for human users for some reason.
        // Using a "sudo as" user to get around the limitation for now.
        // <https://support.shotgunsoftware.com/hc/en-us/requests/114649>
        // *(fixed in ShotGrid v8.16).*
        .authenticate_script_as_user(&login)
        .await
        .expect("Sudo As auth");

    session
        .text_search(
            Some("foobar"),
            vec![("Asset", filters::empty())].into_iter().collect(),
        )
        .execute::<Value>()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_text_search_empty_filters() {
    let login = helpers::get_human_user_login();
    let sg = helpers::get_test_client();
    let session = sg
        // Text search only works for human users for some reason.
        // Using a "sudo as" user to get around the limitation for now.
        // <https://support.shotgunsoftware.com/hc/en-us/requests/114649>
        // *(fixed in ShotGrid v8.16).*
        .authenticate_script_as_user(&login)
        .await
        .expect("Sudo As auth");

    session
        .text_search(Some("foobar"), vec![].into_iter().collect())
        .execute::<Value>()
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_read_work_schedule_for_user() {
    let sg = helpers::get_test_client();
    let session = sg.authenticate_script().await.expect("ApiUser auth");

    let user_id = helpers::get_api_user_id(&session).await;
    // Great Scott!
    let start_date = "1985-10-26";
    let end_date = "1985-10-27";

    session
        .work_days_rules_read::<Value>(&start_date, &end_date, Some(user_id), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_crud_kitchen_sink() {
    let sg = helpers::get_test_client();

    let session = sg
        .authenticate_script_as_user(&helpers::get_human_user_login())
        .await
        .expect("Sudo As auth");

    let author_id = helpers::get_human_user_id(&session).await;
    let project_id = helpers::get_project_id();

    let note: Value = session
        .create(
            "Note",
            json!({
                "subject": "shotgrid-rs test",
                "content": "this is a test",
                "project": { "type": "Project", "id": project_id }
            }),
            Some(&["id", "created_by"].join(",")),
        )
        .await
        .unwrap();

    let note_id = note["data"]["id"].as_i64().unwrap() as i32;

    assert_eq!(
        author_id,
        note["data"]["relationships"]["created_by"]["data"]["id"]
            .as_i64()
            .unwrap() as i32
    );

    // The created/updated timestamps only have precision to the second, so in
    // order to ensure the timestamps are visibly different we need to wait at
    // least 1 sec.
    std::thread::sleep(std::time::Duration::from_millis(1_010));

    let updated_note: Value = session
        .update(
            "Note",
            note_id,
            json!({"content": "test test test"}),
            Some(&["id", "content", "created_at", "updated_at"].join(",")),
        )
        .await
        .unwrap();

    assert_eq!(
        updated_note["data"]["attributes"]["content"]
            .as_str()
            .unwrap(),
        "test test test"
    );

    assert_ne!(
        updated_note["data"]["attributes"]["created_at"]
            .as_str()
            .unwrap(),
        updated_note["data"]["attributes"]["updated_at"]
            .as_str()
            .unwrap()
    );

    helpers::cleanup_entities(&session, &[("Note", note_id)]).await;
}

#[tokio::test]
async fn e2e_test_hierarchy_expand() {
    let sg = helpers::get_test_client();

    let session = sg.authenticate_script().await.expect("ApiUser auth");

    let data = HierarchyExpandRequest {
        // Not sure what I can pass as entity fields to change the
        // response we get from ShotGrid, but at least the server accepts
        // this payload. It just doesn't seem to have any effect.
        entity_fields: Some(vec![HierarchyEntityFields {
            entity: Some("Project".to_string()),
            fields: Some(vec!["tags"].into_iter().map(String::from).collect()),
        }]),
        path: "/".to_string(),
        seed_entity_field: None,
    };

    session.hierarchy_expand(data).await.unwrap();
}

#[tokio::test]
async fn e2e_test_hierarchy_search_by_string() {
    let sg = helpers::get_test_client();
    let login = helpers::get_human_user_login();
    let session = sg
        .authenticate_script_as_user(&login)
        .await
        .expect("Sudo As auth");

    let data = HierarchySearchRequest {
        root_path: None,
        search_criteria: HierarchySearchCriteria::SearchString("Something".to_string()),
        seed_entity_field: None,
    };

    session.hierarchy_search(data).await.unwrap();
}

#[tokio::test]
async fn e2e_test_hierarchy_search_by_entity() {
    let sg = helpers::get_test_client();
    let login = helpers::get_human_user_login();
    let session = sg
        .authenticate_script_as_user(&login)
        .await
        .expect("Sudo As auth");

    let data = HierarchySearchRequest {
        root_path: None,
        search_criteria: HierarchySearchCriteria::Entity(Entity {
            // If the entity doesn't exist you'll get an empty result set, but
            // that's fine for this test.
            id: 123_456,
            r#type: "Asset".to_string(),
        }),
        seed_entity_field: None,
    };

    session.hierarchy_search(data).await.unwrap();
}
