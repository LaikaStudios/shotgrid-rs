//! Small example program that runs a login using a username and password and prints out the
//! resulting response from shotgun.
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
//! $ cargo run --example login -- <username>
//! ```

extern crate shotgun_rs;
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    // Get a username from argv.
    let username = env::args()
        .nth(1)
        .expect("Please specify a user to login as");

    // Prompt the user for a password.
    let password = rpassword::read_password_from_tty(Some("Password: ")).unwrap();

    let sg = Shotgun::new(
        env::var("SG_SERVER").expect("SG_SERVER is required."),
        None,
        None,
    )
        .expect("SG Client");

    let _sess = sg.authenticate_user(&username, &password).await?;
    println!("\nLogin Succeeded!");
    Ok(())
}
