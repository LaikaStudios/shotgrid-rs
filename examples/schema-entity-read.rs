//! Small example program that prints out the schema for a single entity on a given project.
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
//! $ cargo run --example schema-entity-read [project_id] 'task'
//! ```
//!
//! ```text
//! $ cargo run --example schema-entity-read [project_id] 'custom_non_project_entity_01'
//! ```

use shotgun_rs::{Shotgun, TokenResponse};
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();
    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let project_id: Option<i32> = env::args().nth(1).map(|s| s.parse().expect("Project ID"));

    let entity: Option<String> = env::args().nth(2);

    println!("Attempting to read {:?}", entity);
    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let TokenResponse { access_token, .. } = sg.authenticate_script().await?;
    let resp = sg
        .schema_entity_read(&access_token, project_id, &entity.unwrap())
        .await?;

    println!("{:?}", resp);
    Ok(())
}
