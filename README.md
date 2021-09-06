# shotgrid-rs

`shotgrid-rs` is a REST API client for [Autodesk ShotGrid][shotgrid] (formerly
_Shotgun_) built with [reqwest] and [serde_json].

## Usage

The general pattern of usage starts with a `shotgrid_rs::Client`.

```rust,no_run
use shotgrid_rs::Client;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    let server = "https://my-shotgrid.example.com";
    let script_name = "my-api-user";
    let script_key = "********";
    let sg = Client::new(server.to_string(), Some(script_name), Some(script_key))?;
    // ...
    Ok(())
}
```

Once your client is in hand, you'd use one of the authentication methods to
get a `Session`.

```rust,no_run
use shotgrid_rs::Client;

#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {
    let server = "https://my-shotgrid.example.com";
    let script_name = "my-api-user";
    let script_key = "********";
    let sg = Client::new(server.to_string(), Some(script_name), Some(script_key))?;
    // Authenticates using the script name and script key held by the client.
    let session = sg.authenticate_script().await?;
    // ...
    Ok(())
}
```

From there, you can use that `Session` to invoke the various query
methods, either to use ShotGrid's rich filter API to find
records, or to create/update records.

For operations where the schema of the response is *flexible* (based on the
entity type and return fields specified), we use generics to allow the
caller to unpack the response into the type of their choosing. The type just
needs to implement [serde]'s `Deserialize` trait.

A number of structs that are generic over types deeper in the data structure
are provided.
For example, `ResourceArrayResponse` is generic over `R` (the resource) which
is the items in the array portion of the response, and `L` which is the type for
the response's "links" key).

```rust,no_run
use serde_derive::Deserialize;
use shotgrid_rs::types::{PaginationLinks, ResourceArrayResponse, SelfLink};
use shotgrid_rs::Client;
use shotgrid_rs::filters;


/// This struct should match the return fields specified for the search.
#[derive(Debug, Clone, Deserialize)]
struct ProjectAttrs {
    code: Option<String>,
    name: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Project {
    id: Option<i32>,
    r#type: Option<String>,
    attributes: Option<ProjectAttrs>,
    links: Option<SelfLink>,
}


#[tokio::main]
async fn main() -> shotgrid_rs::Result<()> {

    let server = "https://my-shotgrid.example.com";
    let script_name = "my-api-user";
    let script_key = "********";

    let sg = Client::new(server.to_string(), Some(script_name), Some(script_key))?;

    let session = sg.authenticate_script().await?;

    let return_fields = ["id", "code", "name"].join(",");

    // Using type ascription (or a turbofish), we tell search() how to
    // deserialize the response.
    let resp: ResourceArrayResponse<Project, PaginationLinks> = session
        .search("Project", &return_fields, &filters::empty())
        .size(Some(3))
        .execute()
        .await?;

    let items = resp.data.unwrap_or_default();

    for project in items {
        println!("{:?}", project);
    }

    Ok(())
}
```

For times where you don't want to bother defining structs to represent the
response, you can always deserialize to a `serde_json::Value` and interrogate
the value yourself.

## Logging

The `shotgrid_rs` crate offers some logging, though most of it relates to the
internals of the library itself.

If you're interested in logging the HTTP-transport layer, since we're using
[reqwest], you can get some visibility into the transport layer by setting
`reqwest` to `DEBUG`.

Please refer to the docs for your logger crate to see how to adjust log levels
for crates and modules.

## Running Tests

You can run the basic unit test suite via:

```text
$ cargo test
```

In addition to the unit tests, there is a set of end-to-end tests (ie, requires
a live ShotGrid server) which can be run by enabling the `integration-tests`
feature:

```text
$ cargo test --features integration-tests
```

The integration tests require a set of environment vars to be set in order to pass:

- `TEST_SG_SERVER`, the ShotGrid server to connect to.
- `TEST_SG_SCRIPT_NAME`, the name of an ApiUser to connect as.
- `TEST_SG_SCRIPT_KEY`, the API key to go with the name.
- `TEST_SG_HUMAN_USER_LOGIN`, certain tests require a `HumanUser` so this is
  the login to "sudo as" for those tests.
- `TEST_SG_PROJECT_ID`, some tests require a project to filter by.

At the time of writing, these tests read but don't write. This may change in the
future so please take care when setting these vars.

If possible you may want to isolate your test runs to a secondary ShotGrid
server (if you have a spare for development), or at the very least select a
"test" project.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[shotgrid]: https://www.shotgridsoftware.com/
[reqwest]: https://crates.io/crates/reqwest
[serde]: https://crates.io/crates/serde
[serde_json]: https://crates.io/crates/serde_json
