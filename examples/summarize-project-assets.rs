//! Small example program that prints out a table of projects. For this to work you must set 3 env
//! vars, `SG_SERVER`, `SG_SCRIPT_NAME`, and `SG_SCRIPT_KEY`.
//!
//! Set the `SG_SERVER` environment variable to the url for your ShotGrid server, eg:
//!
//! ```text
//! export SG_SERVER=https://shotgrid.example.com
//! ```
//!
//! `shotgrid_rs` also looks at the `CA_BUNDLE` environment variable for when
//! you need a custom CA loaded to access your ShotGrid server, for example:
//!
//! ```text
//! export CA_BUNDLE=/etc/ssl/my-ca-certs.crt
//! ```
//!
//! Usage:
//!
//! ```text
//! $ cargo run --example summarize-project-assets <project id>
//! ```

use shotgrid_rs::filters::{self, field, EntityRef};
use shotgrid_rs::types::{GroupingDirection, GroupingType, SummaryFieldType};
use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let project_id: i32 = env::args()
        .nth(1)
        .expect("must specify a project id")
        .parse()
        .expect("invalid project id");

    let sg = Client::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");
    let sess = sg.authenticate_script().await?;

    let summary = sess
        .summarize(
            "Asset",
            Some(filters::basic(&[
                field("project").is(EntityRef::new("Project", project_id))
            ])),
            vec![("id", SummaryFieldType::Count).into()],
        )
        .grouping(Some(
            vec![("sg_asset_type", GroupingType::Exact, GroupingDirection::Asc)]
                .into_iter()
                .map(Into::into)
                .collect(),
        ))
        .execute()
        .await?;

    // The `SummaryResponse` we get from a summarize() call can be serialized as
    // a json object via serde_json.
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
