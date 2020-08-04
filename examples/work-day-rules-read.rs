//! Small example program that prints out the work day rules for the schedule and optionally user and project.
//!
//! For this to work you must set 3 env
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
//! $ cargo run --example work-day-rules-read 2020-07-06 2020-07-31 [1600] [23]
//! ```

use serde_json::Value;
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let start_date: Option<String> = env::args().skip(1).next().and_then(|s| Some(s));
    let end_date: Option<String> = env::args().skip(2).next().and_then(|s| Some(s));
    let user_id: Option<i32> = env::args()
        .skip(3)
        .next()
        .and_then(|s| Some(s.parse().expect("User ID")));
    let project_id: Option<i32> = env::args()
        .skip(4)
        .next()
        .and_then(|s| Some(s.parse().expect("Project ID")));

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let resp: Value = sg
        .work_days_rules_read(
            &token,
            &start_date.unwrap(),
            &end_date.unwrap(),
            Some(user_id.unwrap()),
            Some(project_id.unwrap()),
        )
        .await?;

    for entry in resp["data"].as_array().expect("response decode") {
        println!("{}", entry);
    }

    Ok(())
}
