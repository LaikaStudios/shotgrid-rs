//! Small example program that updates the last accessed user of a project.
//!
//! For this to work you must set 3 env
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
//! $ cargo run --example project-last-accessed-update 123(project id) 1048 (user id)
//! ```

use shotgrid_rs::types::ProjectAccessUpdateResponse;
use shotgrid_rs::Client;
use std::env;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let project_id: Option<i32> = env::args()
        .nth(1)
        .and_then(|s| Some(s.parse().expect("Project ID")));
    let user_id: Option<i32> = env::args()
        .nth(2)
        .and_then(|s| Some(s.parse().expect("User ID")));

    println!(
        "Attempting to set project {:?} last accessed property to this user: {:?}",
        project_id, user_id
    );

    let sg = Client::new(server, Some(&script_name), Some(&script_key)).expect("SG client");
    let sess = sg.authenticate_script().await?;

    let resp: ProjectAccessUpdateResponse = sess
        .project_last_accessed_update(project_id.unwrap(), user_id.unwrap())
        .await?;
    println!("Data: {:?}", resp.data);
    println!("Links: {:?}", resp.links);

    Ok(())
}
