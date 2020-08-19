//! Small example program that fetches information about an image or attachment field.
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
//! $ cargo run --example entity-file-field-read asset 123456 image [original (alt)] [bytes=0-100 (range)]
//! ```

use serde_json::Value;
use shotgun_rs::types::{AltImages, EntityIdentifier, FieldHashResponse};
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let entity_type: Option<String> = env::args().nth(1);
    let entity_id: Option<i32> = env::args()
        .nth(2)
        .and_then(|s| Some(s.parse().expect("Entity ID")));
    let field_name: Option<String> = env::args().nth(3);
    let alt: Option<String> = env::args().nth(4);
    let range: Option<String> = env::args().nth(5);

    println!(
        "Attempting to read the file field {:?} on {:?} {:?}: alt: {:?} range: {:?}",
        field_name, entity_type, entity_id, alt, range
    );

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let mut alt_option: Option<AltImages> = None;

    if let Some(val) = alt {
        let alt_option = match &*val {
            "original" => Some(AltImages::Original),
            "thumbnail" => Some(AltImages::Thumbnail),
            _ => None,
        };
    }

    let resp: FieldHashResponse = sg
        .entity_file_field_read(
            &token,
            &entity_type.unwrap(),
            entity_id.unwrap(),
            &field_name.unwrap(),
            alt_option,
            range,
        )
        .await?;

    println!("Data: {:?}", resp.data);
    println!("Links: {:?}", resp.links);

    Ok(())
}
