//! Small example program that retrieves the information for an entity's upload for a field.
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
//! $ cargo run --example entity-field-upload-url-read asset 12345 tester image [0]
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

    let entity: Option<String> = env::args().nth(1);
    let entity_id: Option<i32> = env::args().nth(2).map(|s| s.parse().expect("Entity ID"));
    let filename: Option<String> = env::args().nth(3);
    let field_name: Option<String> = env::args().nth(4);
    let multipart_upload: Option<i32> = env::args()
        .nth(5)
        .map(|s| s.parse::<i32>().expect("1 or 0 for multipart"));

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let resp: Value = sg
        .entity_field_upload_url_read(
            &token,
            &entity.unwrap(),
            entity_id.unwrap(),
            &filename.unwrap(),
            &field_name.unwrap(),
            Some(match multipart_upload.unwrap() {
                0 => false,
                _ => true,
            }),
        )
        .await?;

    for key in resp["data"]
        .as_object()
        .expect("response decode data")
        .keys()
    {
        println!("{}: {}", key, resp["data"][key]);
    }

    for key in resp["links"]
        .as_object()
        .expect("response decode links")
        .keys()
    {
        println!("{}: {}", key, resp["links"][key]);
    }

    Ok(())
}
