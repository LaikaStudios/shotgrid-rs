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

use serde_json::json;
use shotgun_rs::types::{GroupingType, SummaryFieldType};
use shotgun_rs::{Shotgun, TokenResponse};
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
    let TokenResponse { access_token, .. } = sg.authenticate_script().await?;

    let resp = sg
        .summarize(
            &access_token,
            "Task",
            Some(json!([["sg_parent_task.Task.id", "in", &parent_tasks]])),
            vec![("id", SummaryFieldType::Count).into()],
        )
        .grouping(Some(
            vec![
                ("sg_parent_task.Task.id", GroupingType::Exact),
                ("sg_status_list", GroupingType::Exact),
            ]
            .into_iter()
            // This leverages `Grouping::from` to convert the two-tuples into `Grouping`s.
            .map(Into::into)
            .collect(),
        ))
        .execute()
        .await?;

    for group in resp.data.groups.unwrap() {
        println!("Parent Task: {}", group.group_value.unwrap_or_default());

        for status_count in group.groups.unwrap() {
            println!(
                "{:>10}: {:>6}",
                status_count.group_value.unwrap(),
                status_count
                    // This is an arbitrary JSON object, represented as a
                    // `HashMap<String, serde_json::Value>`.
                    .summaries
                    .unwrap()
                    .get("id")
                    // attempt to cast the `serde_json::Value` to an integer
                    .and_then(|s| s.as_i64())
                    .unwrap_or(0)
            );
        }
    }
    Ok(())
}
