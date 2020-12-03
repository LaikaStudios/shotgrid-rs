# (Unreleased)

### Breaking Changes

- `Shotgun::schema_field_create()` no longer accepts a `CreateFieldRequest`.
  Instead it takes separate `data_type` and `properties` parameters.
- `Shotgun::schema_field_update()` no longer accepts an `UpdateFieldRequest`.
  Instead it takes separate `properties` and `project_id` parameters.

#### Builders

A number of methods have been updated to use the
[builder pattern](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html)
to streamline usage by allowing the caller to skip setting parameters that have
well-understood defaults available.

- `Shotgun::text_search()` now returns a `TextSearchBuilder`.
- `Shotgun::summarize()` now returns a `SummarizeReqBuilder`.
  - The return value from `SummarizeReqBuilder` is not generic like the old
    `Shotgun::summarize()` method was, instead returning a `SummarizeResponse`.
- `Shotgun::entity_relationship_read()` now returns a (you guessed it)
  `EntityRelationshipReadReqBuilder`.


### Added

- A new `types` module for structs/enums to represent the request/response
  bodies for the Shotgun REST API (based on the OpenApi spec, but lightly
  modified to match reality).
- Added methods to `Shotgun` to represent all endpoints listed in the Shotgun
  OpenApi spec.
- A high-level `Shotgun::upload()` supporting both Shotgun and S3 storage
  services.
- `From` impls added for `CreateUpdateFieldProperty`, `SummaryField`, and `Grouping` so they can be
  conveniently built from tuples.

### Fixed

- `Shotgun::text_search()` no longer panics if given an empty map of entity
  filters.

# [v0.8.2](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.8.1...v0.8.2) (2020-11-25)

### Added

- Backport of `Shotgun::upload()` and related types/functions.


# [v0.8.1](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.8.0...v0.8.1) (2020-09-11)

### Change

 - Add return fields option to update method.

# [v0.8.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.7.0...v0.8.0) (2020-05-14)

### Breaking Changes

- Adopts async/await. Client signatures now use `async fn` instead of futures
  0.1 style `impl Future`.

### Additional

- A `gzip` feature (off by default) has been added to allow dependents to
  enable gzip support for the underlying HTTP client.


# [v0.7.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.6.1...v0.7.0) (2019-11-12)

### Breaking Changes

- `Shotgun::create()` now accepts a new `fields` parameter to control the fields
  returned by shotgun in the response to the request. Pass `None` to default to
  the original behavior which is to return _all fields_.
  
> Note: for now, Shotgun seems to be 
> [ignoring this parameter](https://support.shotgunsoftware.com/hc/en-us/requests/106834?page=1)
> even though it was meant to be supported as of Shotgun 8.5.

### Added

- Added `Shotgun::summarize()` for running aggregate queries.

# [v0.6.1](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.6.0...v0.6.1) (2019-09-20)

- Adds implementation of `Clone` and `Debug` for virtually all public types.

# [v0.6.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.5.0...v0.6.0) (2019-09-03)
- Added `batch` method to client.

# [v0.5.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.4.0...v0.5.0) (2019-07-01)

- Added `schema_fields_read` to read an entity's entire schema.
- Added `schema_field_read` to read an entity's single field schema.

# [v0.4.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.3.4...v0.4.0) (2019-06-21)

### Breaking Changes

- Updated `search` and `text_search` to receive the new `PaginationParameter` struct
  to configure pagination for the given request.

### Additional

- `reqwest::async::Client` (used internally as the http transport) is now
  re-exported allowing users who need to configure their client to do so
  without adding an extra dependency on `reqwest`.

# [v0.3.4](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.3.3...v0.3.4) (2019-06-19)

- Added `Shotgun::text_search()` to do a search of entities that match a given text value.

# [v0.3.3](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.3.2...v0.3.3) (2019-06-17)

- Marks `Shotgun::update()` public so it can be used by callers.


# [v0.3.2](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.3.1...v0.3.2) (2019-06-14)

- 404 status returns from shotgun are now returned as `ShotgunError::NotFound`
  instead of `ShotgunError::ServerError.
- Fields on `ErrorObject` are now public so callers can inspect
  them as needed.

# [v0.3.1](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.3.0...v0.3.1) (2019-06-13)

- Fixed issue where shotgun error payloads might not be correctly parsed and
  returned as a `ShotgunError::ServerError`.

# [v0.3.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.2.2...v0.3.0) (2019-06-11)

- Removed `Entity` enum in favor of plain `&str`. _Feel free to manage enums
  for these in your application code as needed._
- `search()` now accepts a plain `&serde_json::Value` instead of requiring the
  caller to wrap one in the `Filters` enum.
- Removed `Filters` enum in favor of looking at the shape of the `filters`
  json payload.
- Added `ShotgunError::InvalidFilters`, returned by `search()` when the
  `"filters"` key in the `filters` json is not either an array or object.
- Added `Shotgun::schema_read()` to do a read of all entities for a given
  (optional) project.

# [v0.2.2](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.2.1...v0.2.2) (2019-06-07)

- Added `Shotgun::authenticate_script_as_user()` to "sudo as" a given user while still
  authenticating as an api user.

# [v0.2.1](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.2.0...v0.2.1) (2019-05-30)

## Changes

 - Added Department to Entity enum

# [v0.2.0](https://github.com/LaikaStudios/shotgrid-rs/compare/v0.1.0...v0.2.0) (2019-05-28)

## Client initialization

Previously `Shotgun::new()` and `Shotgun::with_client()` accepted api name/key
parameters as `Option<String>`, but now accept a more relaxed `Option<&str>`.

## Revised error handling.

All methods now return results/futures with `ShotgunError`.

The outliers were largely the methods used to initialize the `Shotgun`
struct which previously returned `Result<Shotgun, failure::Error>`
but now use the `ShotgunError` type instead.

In addition to this, internal APIs which result in the processing of HTTP
Responses from Shotgun previously treated the response bodies optimistically,
assuming the deserialization target would match the caller's specified shape.

This often resulted in an error which is less useful when the response from
Shotgun is feedback explaining why a request failed. The reported error would
simply cite that the payload wasn't of the expected shape. Instead we are now
checking the response for an `errors` key, and when present we will parse the
feedback being given by Shotgun and return that as the error.


# [v0.1.0](https://github.com/LaikaStudios/shotgrid-rs/tree/v0.1.0) (2019-04-26)

Initial release.

- Basic crud operations.
- User login and script authentication.
- Search.
