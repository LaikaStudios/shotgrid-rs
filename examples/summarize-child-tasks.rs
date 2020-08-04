//! Small example program that prints out a table of projects. For this to work you must set 3 env
//! vars, `SG_SERVER`, `SG_SCRIPT_NAME`, and `SG_SCRIPT_KEY`.
//!
//! Set the `SG_SERVER` environment variable to the url for your shotgun server, eg:
//!
//! ```text
//! export SG_SERVER=https://shotgun.example.com
//! ```
//!
//! `shotgun_rs` also looks at the `CA_BUNDLE` environment variable for when you need a custom CA
//! loaded to access your shotgun server, for example:
//!
//! ```text
//! export CA_BUNDLE=/etc/ssl/my-ca-certs.crt
//! ```
//!
//! Usage:
//!
//! ```text
//! $ cargo run --example summarize-child-tasks <task ids...>
//! ```

use serde_json::{json, Value};
use shotgun_rs::structs::{Grouping, GroupingType, SummaryField, SummaryFieldType};
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let parent_tasks: Vec<i32> = env::args()
        .skip(1)
        .map(|s| s.parse().expect("invalid task id"))
        .collect();

    if parent_tasks.is_empty() {
        panic!("must specify one or more parent task ids");
    }

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");
    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let resp: Value = sg
        .summarize(
            &token,
            "Task",
            Some(json!([["sg_parent_task.Task.id", "in", &parent_tasks]])),
            Some(vec![SummaryField {
                field: "id".to_string(),
                r#type: SummaryFieldType::Count,
            }]),
            Some(vec![
                Grouping {
                    field: "sg_parent_task.Task.id".to_string(),
                    r#type: GroupingType::Exact,
                    direction: None,
                },
                Grouping {
                    field: "sg_status_list".to_string(),
                    r#type: GroupingType::Exact,
                    direction: None,
                },
            ]),
            None,
        )
        .await?;

    for group in resp["data"]["groups"].as_array().unwrap() {
        println!("Parent Task: {}", group["group_value"]);
        for status_count in group["groups"].as_array().unwrap() {
            println!(
                "{:>10}: {:>6}",
                status_count["group_value"].as_str().unwrap(),
                status_count["summaries"]["id"].as_i64().unwrap_or(0)
            );
        }
    }
    Ok(())
}
