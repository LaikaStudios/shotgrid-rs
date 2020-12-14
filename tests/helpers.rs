#![cfg(feature = "integration-tests")]
//! Some utils for the integration tests.

use futures::future;
use serde_json::{json, Value};
use shotgun_rs::{Session, Shotgun};

pub fn get_human_user_login() -> String {
    dotenv::dotenv().ok();
    std::env::var("TEST_SG_HUMAN_USER_LOGIN").expect("TEST_SG_HUMAN_USER_LOGIN")
}

pub async fn get_human_user_id(sess: &Session<'_>) -> i32 {
    let resp: Value = sess
        .search(
            "HumanUser",
            "id",
            &json!([[
                "login",
                "is",
                std::env::var("TEST_SG_HUMAN_USER_LOGIN").expect("TEST_SG_HUMAN_USER_LOGIN")
            ]]),
        )
        .unwrap()
        .size(Some(1))
        .execute()
        .await
        .unwrap();

    let data = resp["data"].as_array().unwrap();
    data[0]["id"].as_i64().unwrap() as i32
}

pub fn get_project_id() -> i32 {
    dotenv::dotenv().ok();
    std::env::var("TEST_SG_PROJECT_ID")
        .map(|s| s.parse().unwrap())
        .expect("TEST_SG_PROJECT_ID")
}

pub fn get_test_client() -> Shotgun {
    dotenv::dotenv().ok();
    let sg_server: String = std::env::var("TEST_SG_SERVER").expect("TEST_SG_SERVER");
    let sg_script_name: String = std::env::var("TEST_SG_SCRIPT_NAME").expect("TEST_SG_SCRIPT_NAME");
    let sg_script_key: String = std::env::var("TEST_SG_SCRIPT_KEY").expect("TEST_SG_SCRIPT_KEY");
    Shotgun::new(sg_server, Some(&sg_script_name), Some(&sg_script_key)).expect("client init")
}

pub async fn get_api_user_id(sess: &Session<'_>) -> i32 {
    let resp: Value = sess
        .search(
            "ApiUser",
            "id",
            &json!([[
                "firstname",
                "is",
                std::env::var("TEST_SG_SCRIPT_NAME").unwrap()
            ]]),
        )
        .unwrap()
        .size(Some(1))
        .execute()
        .await
        .unwrap();

    let data = resp["data"].as_array().unwrap();
    data[0]["id"].as_i64().unwrap() as i32
}

/// Attempt to delete entities - print to stderr when there are problems.
pub async fn cleanup_entities(session: &Session<'_>, keys: &[(&str, i32)]) {
    let errors = future::join_all(keys.iter().map(|(typ, id)| session.destroy(typ, *id)))
        .await
        .into_iter()
        .filter(|r| r.is_err());
    for err in errors {
        eprintln!("{:?}", err.unwrap_err());
    }
}
