//! Small example program that retrieves the information for an entity's upload.
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
//! $ cargo run --example upload note 12345 tester path/to/file.ext [optional display name]
//! ```

use shotgun_rs::Shotgun;
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY");

    let entity = env::args().nth(1).unwrap();
    let entity_id: i32 = env::args()
        .nth(2)
        .map(|s| s.parse().expect("Entity ID"))
        .expect("Entity ID");
    let file_path: PathBuf = env::args().nth(3).expect("File Path").into();
    let display_name = env::args().nth(4);

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");
    let session = sg.authenticate_script().await?;
    let fh = std::fs::OpenOptions::new()
        .read(true)
        .open(&file_path)
        .unwrap();

    let filename = file_path.file_name().as_ref().unwrap().to_string_lossy();

    session
        .upload(&entity, entity_id, Some("attachments"), &filename, fh)
        .display_name(display_name)
        .send()
        .await?;

    Ok(())
}
