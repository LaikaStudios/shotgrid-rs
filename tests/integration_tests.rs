#![cfg(feature = "integration-tests")]
//! The unfortunate thing for these tests that target an actual shotgun server
//! is we can't really assert anything about the records in the database unless
//! we create those records in the test itself (which we'd rather not do).
//!
//! At least the initial focus will to just assert that the response we get is
//! `Ok` vs `Err` and ignore the actual response bodies. Also, we'll avoid writes
//! for now.
//!
//! These tests depend on several env vars being set.
//!
//! - `TEST_SG_SERVER`, the shotgun server to connect to.
//! - `TEST_SG_SCRIPT_NAME`, the name of an ApiUser to connect as.
//! - `TEST_SG_SCRIPT_KEY`, the API key to go with the name.
//! - `TEST_SG_HUMAN_USER_LOGIN`, certain tests require a HumanUser so this is
//!   the login to "sudo as" for those tests.
//! - `TEST_SG_PROJECT_ID`, some tests require a project to filter by.

use serde_json::{json, Value};
use shotgun_rs::types::{
    Entity, Grouping, GroupingDirection, GroupingType, HierarchyEntityFields,
    HierarchyExpandRequest, HierarchySearchCriteria, HierarchySearchRequest, SummaryField,
    SummaryFieldType,
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
    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };
    sg.preferences_read::<Value>(&token).await.unwrap();
}

#[tokio::test]
async fn e2e_test_list_projects() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.search(
        &token,
        "Project",
        &["id", "code", "name"].join(","),
        &json!([]),
    )
    .unwrap()
    .size(Some(3))
    .number(Some(1))
    .execute::<Value>()
    .await
    .unwrap();
}

#[tokio::test]
async fn e2e_test_schema_entity_read() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.schema_entity_read(&token, None, "Asset").await.unwrap();
}
#[tokio::test]
async fn e2e_test_schema_entity_read_for_project() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.schema_entity_read(&token, Some(project_id), "Asset")
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_schema_read() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.schema_read::<Value>(&token, None).await.unwrap();
}

#[tokio::test]
async fn e2e_test_schema_read_for_project() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.schema_read::<Value>(&token, Some(project_id))
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_summarize_project_assets() {
    let sg = helpers::get_test_client();
    let project_id = helpers::get_project_id();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.summarize::<Value>(
        &token,
        "Asset",
        Some(json!([["project", "is", {"type": "Project", "id": project_id}]])),
        Some(vec![SummaryField {
            field: "id".to_string(),
            r#type: SummaryFieldType::Count,
        }]),
        Some(vec![Grouping {
            field: "sg_asset_type".to_string(),
            r#type: GroupingType::Exact,
            direction: Some(GroupingDirection::Asc),
        }]),
        None,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn e2e_test_read_user_follows() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    let user_id = helpers::get_api_user_id(&sg, &token).await;

    sg.user_follows_read::<Value>(&token, user_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_text_search() {
    let login = helpers::get_human_user_login();
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg
            // Text search only works for human users for some reason.
            // Using a "sudo as" user to get around the limitation for now.
            // <https://support.shotgunsoftware.com/hc/en-us/requests/114649>
            // *(fixed in shotgun v8.16).*
            .authenticate_script_as_user(&login)
            .await
            .expect("ApiUser auth as HumanUser");
        resp["access_token"].as_str().unwrap().to_string()
    };

    sg.text_search::<Value>(
        &token,
        &json!({
            "text": "foobar",
            "entity_types": {
                "Asset": []
            },
        }),
        None,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn e2e_test_read_work_schedule_for_user() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    let user_id = helpers::get_api_user_id(&sg, &token).await;
    // Great Scott!
    let start_date = "1985-10-26";
    let end_date = "1985-10-27";

    sg.work_days_rules_read::<Value>(&token, &start_date, &end_date, Some(user_id), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn e2e_test_crud_kitchen_sink() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg
            .authenticate_script_as_user(&helpers::get_human_user_login())
            .await
            .expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    let author_id = helpers::get_human_user_id(&sg, &token).await;
    let project_id = helpers::get_project_id();

    let note: Value = sg
        .create(
            &token,
            "Note",
            json!({
                "subject": "shotgun-rs test",
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

    let updated_note: Value = sg
        .update(
            &token,
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

    helpers::cleanup_entities(&sg, &token, &[("Note", note_id)]).await;
}

#[tokio::test]
async fn e2e_test_hierarchy_expand() {
    let sg = helpers::get_test_client();

    let token = {
        let resp: Value = sg.authenticate_script().await.expect("ApiUser auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    let data = HierarchyExpandRequest {
        // Not sure what I can pass as entity fields to change the
        // response we get from shotgun, but at least the server accepts
        // this payload. It just doesn't seem to have any effect.
        entity_fields: Some(vec![HierarchyEntityFields {
            entity: Some("Project".to_string()),
            fields: Some(vec!["tags"].into_iter().map(String::from).collect()),
        }]),
        path: "/".to_string(),
        seed_entity_field: None,
    };

    sg.hierarchy_expand(&token, data).await.unwrap();
}

#[tokio::test]
async fn e2e_test_hierarchy_search_by_string() {
    let sg = helpers::get_test_client();
    let login = helpers::get_human_user_login();
    let token = {
        let resp: Value = sg
            .authenticate_script_as_user(&login)
            .await
            .expect("Sudo As auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

    let data = HierarchySearchRequest {
        root_path: None,
        search_criteria: HierarchySearchCriteria::SearchString("Something".to_string()),
        seed_entity_field: None,
    };

    sg.hierarchy_search(&token, data).await.unwrap();
}

#[tokio::test]
async fn e2e_test_hierarchy_search_by_entity() {
    let sg = helpers::get_test_client();
    let login = helpers::get_human_user_login();
    let token = {
        let resp: Value = sg
            .authenticate_script_as_user(&login)
            .await
            .expect("Sudo As auth");
        resp["access_token"].as_str().unwrap().to_string()
    };

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

    sg.hierarchy_search(&token, data).await.unwrap();
}
