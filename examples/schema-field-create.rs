//! Small example program to create a field on an entity.
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
//! $ cargo run --example schema-field-create task name(property_name) my_value(value)
//! ```
//!
//! For the sake of brevity, this example is only going to create properties with a data type of text.
//! The property_name has to be one of the values underneath schema field record - name, description, etc:
//! <https://developer.shotgunsoftware.com/rest-api/#schemaschemafieldrecord>
//! Also listed in the struct types/SchemaFieldRecord
//!

use serde_json::Value;
use shotgun_rs::types::{
    CreateFieldRequest, CreateUpdateFieldProperty, FieldDataType, SchemaFieldResponse,
};
use shotgun_rs::Shotgun;
use std::env;

#[tokio::main]
async fn main() -> shotgun_rs::Result<()> {
    dotenv::dotenv().ok();

    let server = env::var("SG_SERVER").expect("SG_SERVER is required var.");
    let script_name = env::var("SG_SCRIPT_NAME").expect("SG_SCRIPT_NAME is required var.");
    let script_key = env::var("SG_SCRIPT_KEY").expect("SG_SCRIPT_KEY is required var.");

    let entity_type: Option<String> = env::args().nth(1);
    let property_name: Option<String> = env::args().nth(2);
    let property_value: Option<String> = env::args().nth(3);

    println!(
        "Attempting to add {:?} to {:?} with the type of TEXT and value of {:?}",
        property_name, entity_type, property_value
    );

    let sg = Shotgun::new(server, Some(&script_name), Some(&script_key)).expect("SG Client");

    let token = {
        let resp: Value = sg.authenticate_script().await?;
        resp["access_token"].as_str().unwrap().to_string()
    };

    let properties = CreateUpdateFieldProperty {
        property_name: property_name.unwrap(),
        value: property_value.unwrap(),
    };

    let data = CreateFieldRequest {
        data_type: FieldDataType::Text,
        properties: vec![properties],
    };

    let resp: SchemaFieldResponse = sg
        .schema_field_create(&token, &entity_type.unwrap(), &data)
        .await?;

    println!("{:?}", resp);
    Ok(())
}
