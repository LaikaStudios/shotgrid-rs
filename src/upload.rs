//! Uploading files to Shotgun can happen in a handful of ways.
//!
//! At a high level, it breaks down into a couple different aspects that are
//! mixed/matched depending on:
//!
//! - The thing you are linking your file to.
//! - The size of the file.
//! - The configuration of your Shotgun server (specifically the *storage service* it uses).
//!
//! Uploads that target an entity without specifying a field are thought to be
//! linked to *the record* as opposed to *a field*.
//!
//! Uploads that target the `images` field specifically are thought to be
//! "thumbnail uploads" which are handled slightly differently, ignoring any
//! display name or tag data sent with the request.
//!
//! Uploads that target *other fields* or a *record* are thought to be
//! "attachment uploads" and will accept a display name and tags.
//!
//! Shotgun instances that are configured to use S3 as their storage service
//! offer a *multipart* upload option, which is required for files larger than
//! 500Mb. Effectively *multipart* in this case means breaking up the file into
//! chunks and sending them to S3 individually. After all the chunks have been
//! sent, Shotgun reassembles the file.
//!
//! For Shotgun instances that are configured to use Shotgun (identified as "sg"
//! in JSON payloads) no such *multipart* option is available.
//!
//! For more on this, refer to the Shotgun REST API docs:
//!
//! <https://developer.shotgunsoftware.com/rest-api/#shotgun-rest-api-Uploading-and-Downloading-Files>
use crate::types::{Entity, NextUploadPartResponse, UploadInfoResponse, UploadResponse};
use crate::{handle_response, Result, Shotgun, ShotgunError};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::io::Read;
use std::str::FromStr;

// Per the shotgun docs, multipart uploads should use 5Mb (minimum, save for
// the final part) sized chunks.
// Multipart is *required* for uploads >= 500Mb on S3 storage.
pub const MAX_MULTIPART_CHUNK_SIZE: usize = 500 * 1024 * 1024;
pub const MIN_MULTIPART_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// Configures a file upload request.
///
/// This is the return value from `Shotgun::upload()`, used to configure the
/// behavior of the upload.
// XXX: we could simplify this type by accepting the file_content as a param
// to `send()` instead of holding it in the builder. Food for thought.
pub struct UploadReqBuilder<'a, R: Read> {
    sg: &'a Shotgun,
    token: &'a str,
    entity_type: &'a str,
    entity_id: i32,
    /// This optional field name must be a "file type" field when specified.
    field: Option<&'a str>,
    /// The original filename. This is used by Shotgun to know how to serve the
    /// file in the web UI.
    /// Effectively, this tells Shotgun what content-type header to send
    /// with it.
    filename: &'a str,
    /// The bytes of the file to upload.
    ///
    /// Can be any type that implements `Read`.
    file_content: R,
    // =========================================================================
    // The stuff above this comment is the required point of entry stuff.
    // The stuff below is the truly optional stuff, or stuff we can otherwise
    // provide defaults for.
    // =========================================================================
    display_name: Option<String>,
    tags: Option<Vec<Entity>>,
    multipart: bool,
    multipart_chunk_size: usize,
}

impl<'a, R> UploadReqBuilder<'a, R>
where
    R: Read,
{
    pub(crate) fn new(
        sg: &'a Shotgun,
        token: &'a str,
        entity_type: &'a str,
        entity_id: i32,
        field: Option<&'a str>,
        filename: &'a str,
        file_content: R,
    ) -> Self {
        Self {
            sg,
            token,
            entity_type,
            entity_id,
            field,
            filename,
            file_content,
            // Optional stuff
            display_name: None,
            tags: None,
            multipart: false,
            multipart_chunk_size: 10 * 1024 * 1024, // 10Mb
        }
    }

    /// Sets the text label for the attachment.
    ///
    /// Ignored when uploading to the "images" field since this means we're
    /// uploading a thumbnail instead of an attachment.
    pub fn display_name(mut self, display_name: Option<String>) -> Self {
        self.display_name = display_name;
        self
    }

    /// Tags to link to the attachment.
    ///
    /// Ignored when uploading to the "images" field since this means we're
    /// uploading a thumbnail instead of an attachment.
    pub fn tags(mut self, tags: Option<Vec<Entity>>) -> Self {
        self.tags = tags;
        self
    }

    /// When set to `true`, breaks the file up into chunks which are each
    /// uploaded to the server separately.
    ///
    /// Note: multipart support is *only available* when your Shotgun instance
    /// is configured to use **S3** as its **storage service**.
    pub fn multipart(mut self, multipart: bool) -> Self {
        self.multipart = multipart;
        self
    }

    /// When performing a multipart upload, this controls how many bytes each
    /// "part" will be.
    ///
    /// Legal values are **between 5Mb and 500Mb**.
    ///
    /// Default is *10Mb*.
    ///
    /// This value is validated prior to the execution of the request(s), so
    /// setting the chunk size to an *out of bounds* value will cause terminal
    /// methods such as `send()` to return an `Err`, short-circuiting the
    /// requests that would follow (and fail).
    pub fn chunk_size(mut self, bytes_per_chunk: usize) -> Self {
        self.multipart_chunk_size = bytes_per_chunk;
        self
    }

    /// Helper to manage the complexities of the multipart flow.
    ///
    /// > Multipart uploads are only possible if your shotgun instance is
    /// > configured to use S3 storage.
    ///
    /// Multipart uploads involve splitting the file into chunks and making a
    /// PUT request for each.
    ///
    /// Each put request will respond with an ETag header which is used to
    /// identify each chunk so shotgun can and reassemble the file once the
    /// entire operation has been completed.
    ///
    /// Each time you PUT bytes to the storage service, you must then return to
    /// shotgun to request the next url to PUT to.
    ///
    /// The result of this method is either a vec of etag values (one per chunk).
    /// In the event that any of the requests for this flow fail, the result
    /// will be the Err from the failed request, but in addition to this, an
    /// *abort request* will be sent to signal to shotgun that it should not
    /// expect any more chunks. If the *abort request fails* the Err for that
    /// failure will be logged as a warning (not an error).
    async fn do_multipart_upload(
        sg: &Shotgun,
        token: &str,
        file_content: R,
        upload_url: String,
        get_next_part: String,
        chunk_size: usize,
    ) -> Result<Vec<String>>
    where
        R: Read,
    {
        let mut file_content = file_content;

        let mut upload_url = upload_url;
        let mut get_next_part = get_next_part;
        let mut etags: Vec<String> = vec![];

        // Per the docs, multipart uploads should use 5Mb (minimum, save for
        // the final part) sized chunks.
        // Multipart is *required* for uploads >= 500Mb on S3 storage, so I
        // assume failing to set the bool to true with a large file will get
        // you a bad response during the PUT of your non-multipart attempt.
        //
        // While multipart is *required* for files exceeding 500Mb, it is
        // still desirable for smaller or even medium sized files since this
        // flow implicitly makes it possible to "checkpoint" your upload, as
        // well as making it possible to add per-chunk retries.
        //
        // It also means if your file content is coming from some sort of
        // stream or perhaps from a `File` read from disk, it means you only
        // need to hold a portion of the bytes in memory at any given time.

        // With large values for this buffer size you can see a stack overflow,
        // which will panic. Starting out at 4k. It might be possible to provide
        // a way for folks to customize the size, but it'll probably be tricky
        // since the size has to be specified as a const.
        // Would need to be via a feature flag or some other macro like `env!()`.
        let mut read_buf = [0_u8; 4 * 1024];
        let mut body_buf = Vec::with_capacity(chunk_size);

        let mut uploaded_bytes: usize = 0;

        // XXX: loops seem fair for this, but the signature of this method sort
        // of nods towards a recursive solution.
        // I think we should stick with the loops for now, but focus on cleanup
        // for clarity, only attempting to refactor for recursion if we cannot
        // arrive at something more readable with another pass.
        //
        // One advantage of loops versus recursion is it may be possible to run
        // several of these requests in parallel (though I'm unsure if the GET
        // requests that hand out upload urls are really equipped for this or if
        // they expect things to happen in a strict sequence).

        loop {
            // This loop runs for each chunk of the file we're uploading.
            //
            // There's some preamble to it, but the flow is like:
            //
            // - Fill the body buffer up to `chunk_size` in length or until the
            //   reader is empty.
            // - PUT the bytes in the body buffer up to the upload url (saving
            //   the ETag header from each response).
            // - GET a new upload/get_next_part url pair.
            // - repeat until the reader is exhausted...

            loop {
                // This inner loop is all about pulling bytes out of the reader and
                // loading them up into a vec of a particular size, ie: `chunk_size`.

                let len = file_content.read(&mut read_buf)?;
                if len == 0 {
                    break;
                }
                body_buf.extend_from_slice(&read_buf[0..len]);
                if body_buf.len() >= chunk_size {
                    break;
                }
            }

            if body_buf.is_empty() {
                break;
            }

            let buf_len = body_buf.len();
            let upload_resp = {
                let upload_req = sg
                    .client
                    .put(&upload_url)
                    .header("Content-Length", buf_len)
                    .body(
                        // It's possible that `body_buf` could be larger than
                        // `chunk_size`. When `chunk_size` is set close to the
                        // max, this could mean the request body would be too
                        // large and could be rejected by the storage service.
                        // Only take *at most* `chunk_size` worth of bytes,
                        // leaving the rest in the buffer for the next iteration
                        // of the loop.
                        if buf_len > chunk_size {
                            body_buf.drain(0..chunk_size)
                        } else {
                            body_buf.drain(..)
                        }
                        .collect::<Vec<_>>(),
                    )
                    .header("Accept", "application/json");
                // TODO: add some retries to this
                upload_req.send().await?.error_for_status().map_err(|e| {
                    let reason = if let Some(status) = e.status() {
                        format!(
                            "Failed to upload chunk. Storage service responded: `{}`",
                            status
                        )
                    } else {
                        format!("Failed to upload chunk. Cause: `{}`", e)
                    };
                    ShotgunError::UploadError(reason)
                })?
            };

            let etag = upload_resp
                .headers()
                .get(reqwest::header::ETAG)
                .ok_or_else(|| {
                    ShotgunError::UploadError(String::from(
                        "Multipart upload response missing ETag header.",
                    ))
                })?;

            // Note that for some reason the etag header value will include
            // double quotes in the string. This is apparently fine and/or
            // expected. Don't worry about it if you see it in the json
            // payloads.
            // My initial assumption was something wrong was happening, but
            // no... it's fine.
            etags.push(etag.to_str().unwrap().to_string());

            uploaded_bytes += buf_len;
            log::trace!("Uploaded {} ({}) bytes.", buf_len, uploaded_bytes);

            // XXX: used to force a multi-part upload to fail
            // if uploaded_bytes > buf_len {
            //     return Err(ShotgunError::UploadError(String::from("Oops!!")));
            // }

            let next: NextUploadPartResponse = handle_response(
                sg.client
                    .get(&format!("{}{}", sg.sg_server, get_next_part))
                    .header("Accept", "application/json")
                    .bearer_auth(token)
                    .send()
                    .await?,
            )
            .await
            .map_err(|e| {
                ShotgunError::UploadError(format!(
                    "Failed to get next upload info. Cause: `{:?}`.",
                    e,
                ))
            })?;

            get_next_part = next
                .links
                .as_ref()
                .and_then(|links| links.get_next_part.clone())
                .ok_or_else(|| {
                    ShotgunError::UploadError(String::from(
                        "Get Next Part response missing get_next_part key.",
                    ))
                })?;
            upload_url = next
                .links
                .as_ref()
                .and_then(|links| links.upload.clone())
                .ok_or_else(|| {
                    ShotgunError::UploadError(String::from(
                        "Get Next Part response missing upload key.",
                    ))
                })?;
        }

        Ok(etags)
    }

    async fn abort_multipart_upload(
        sg: &Shotgun,
        token: &str,
        completion_url: &str,
        completion_body: &Value,
    ) {
        let abort_url = format!("{}/multipart_abort", completion_url);
        match sg
            .client
            .post(&abort_url)
            // The Shotgun REST API spec says the body should
            // include the "upload_info" key at the top-level of
            // by object, but in reality this gets you a 400
            // response where the error payload lists all the
            // fields as missing.
            .json(&completion_body["upload_info"])
            .bearer_auth(token)
            .header("Accept", "application/json")
            .send()
            .await
        {
            Ok(res) if !res.status().is_success() => {
                log::warn!(
                    "Failed to properly abort multipart upload. Got status: `{}`",
                    res.status()
                );
            }
            Err(err) => {
                log::warn!("Failed to properly abort multipart upload: `{}`", err);
            }
            Ok(_) => {}
        }
    }

    pub async fn send(self) -> Result<()> {
        let Self {
            sg,
            token,
            entity_type,
            entity_id,
            field,
            filename,
            mut file_content,
            display_name,
            tags,
            multipart,
            multipart_chunk_size,
        } = self;

        if multipart {
            if !(MAX_MULTIPART_CHUNK_SIZE >= multipart_chunk_size
                && multipart_chunk_size >= MIN_MULTIPART_CHUNK_SIZE)
            {
                return Err(ShotgunError::UploadError(format!(
                    "Multipart chunk size must be between `{}` and `{}`",
                    MIN_MULTIPART_CHUNK_SIZE, MAX_MULTIPART_CHUNK_SIZE
                )));
            }
        }

        // This multi-step flow performs the following requests in order:
        //
        // - initiate the upload (gets you the a url to send bytes to, and misc data about the upload).
        // - PUT bytes using the url you receive in the response from the first
        //   request (gets you the ID of the upload operation).
        // - POST a "completion" request to finalize the operation using pieces
        //   of the responses from *both previous requests*.
        //
        // Some extra metadata can be set in the 3rd and final step, such as
        // setting the human readable name or associating tags with the attachment.

        let init_resp: UploadInfoResponse = match field {
            None => {
                sg.entity_upload_url_read(&token, entity_type, entity_id, filename, Some(multipart))
                    .await
            }
            Some(field) => {
                sg.entity_field_upload_url_read(
                    &token,
                    entity_type,
                    entity_id,
                    filename,
                    field,
                    Some(multipart),
                )
                .await
            }
        }?;

        // We need to merge the data from the initial "upload info" request
        // with the fields from the actual upload.
        let upload_info = init_resp.data.ok_or_else(|| {
            ShotgunError::UploadError(String::from("Upload info missing in server response."))
        })?;

        let upload_type: UploadType = upload_info
            .upload_type
            .as_ref()
            .map(|s| s.parse())
            .unwrap_or_else(|| {
                Err(ShotgunError::UploadError(String::from(
                    "Upload type missing from server response.",
                )))
            })?;

        let storage_service: StorageService = upload_info
            .storage_service
            .as_ref()
            .map(|s| s.parse())
            .unwrap_or_else(|| {
                Err(ShotgunError::UploadError(String::from(
                    "Storage service missing from server response.",
                )))
            })?;

        let upload_url = init_resp
            .links
            .as_ref()
            .and_then(|links| links.upload.as_ref())
            .ok_or_else(|| {
                ShotgunError::UploadError(String::from("Upload URL missing in server response."))
            })?;

        let completion_url = format!(
            "{}{}",
            sg.sg_server,
            init_resp
                .links
                .as_ref()
                .and_then(|links| links.complete_upload.as_ref())
                .ok_or_else(|| {
                    ShotgunError::UploadError(String::from(
                        "Completion URL missing in server response.",
                    ))
                })?
        );

        let mut completion_body = json!({
            "upload_info": &upload_info,
            "upload_data": {}
        });

        match (storage_service, multipart) {
            (StorageService::Shotgun, false) => {
                let mut body = vec![];
                file_content.read_to_end(&mut body)?;
                let upload_req = sg
                    .client
                    .put(upload_url)
                    .body(body)
                    .header("Accept", "application/json")
                    .bearer_auth(token);

                let upload_resp: UploadResponse = handle_response(upload_req.send().await?).await?;

                let upload_data = upload_resp.data.ok_or_else(|| {
                    ShotgunError::UploadError(String::from(
                        "Upload Response data missing in server response.",
                    ))
                })?;

                if let Some(original_filename) = upload_data.original_filename {
                    completion_body["upload_info"]["original_filename"] = json!(original_filename);
                }
                if let Some(upload_id) = upload_data.upload_id {
                    completion_body["upload_info"]["upload_id"] = json!(upload_id);
                }
            }
            (StorageService::S3, false) => {
                let mut body = vec![];
                file_content.read_to_end(&mut body)?;
                // S3 uses tokens in the query string instead of auth headers.
                let upload_resp = sg
                    .client
                    .put(upload_url)
                    .body(body)
                    .header("Accept", "application/json")
                    .send()
                    .await?;
                // This should be a 200, but just in case AWS change their mind
                // about signalling, we'll look for any 2xx.
                if !upload_resp.status().is_success() {
                    return Err(ShotgunError::UploadError(String::from("S3 upload failed.")));
                }
            }
            (StorageService::S3, true) => {
                let get_next_part = init_resp
                    .links
                    .as_ref()
                    .and_then(|links| links.get_next_part.clone())
                    .ok_or_else(|| {
                        ShotgunError::UploadError(String::from(
                            "Init response missing get_next_part key.",
                        ))
                    })?;

                let maybe_etags: Result<Vec<String>> = Self::do_multipart_upload(
                    &sg,
                    token,
                    file_content,
                    upload_url.clone(),
                    get_next_part,
                    multipart_chunk_size,
                )
                .await;

                // Either we get a mess of etags (one per chunk) or something
                // went wrong during the upload.
                match maybe_etags {
                    Ok(etags) => {
                        completion_body["upload_info"]["etags"] = json!(etags);
                    }

                    Err(err) => {
                        log::error!("{}", err);
                        Self::abort_multipart_upload(&sg, token, &completion_url, &completion_body)
                            .await;
                        return Err(err); // Bail with the original cause
                    }
                }
            }
            (_, true) => {
                // Multipart uploads are only supported for S3 storage.
                // In truth, the very first request made to initiate the upload
                // should have been rejected with a 400 so if we're here without
                // S3 storage being active, there's been some programmer error.
                return Err(ShotgunError::MultipartNotSupported);
            }
        }

        // The `upload_data` key should be left as empty object for "thumbnail uploads."
        // <https://developer.shotgunsoftware.com/rest-api/#completing-an-upload>
        //
        // In practice, it seems safe to send data in this key, but it might be
        // ignored. We may as well elect to not send the extra bytes if the
        // caller somehow decides to supply these params.
        //
        // XXX: seems like the upload type will be "Thumbnail" when you select
        // the "image" field as the upload target.
        // <https://gist.github.com/daigles/ff958b8b3ed695329d371e5d500acb0a#file-rest_upload_download_sample-py-L451-L454>
        match upload_type {
            UploadType::Thumbnail => {}
            _ => {
                if let Some(display_name) = display_name {
                    completion_body["upload_data"]["display_name"] = json!(display_name);
                }

                if let Some(tags) = tags {
                    completion_body["upload_data"]["tags"] = json!(tags);
                }
            }
        }

        let completion_resp = match sg
            .client
            .post(&completion_url)
            .json(&completion_body)
            .bearer_auth(token)
            .send()
            .await
        {
            // If the upload was multipart and the completion request fails, we
            // abort the whole thing.
            Ok(resp) if multipart && !resp.status().is_success() => {
                Self::abort_multipart_upload(&sg, token, &completion_url, &completion_body).await;

                return Err(ShotgunError::UploadError(format!(
                    "Got a bad status ({}) from completion endpoint. Upload aborted.",
                    resp.status()
                )));
            }
            // If there was a connection failure (or some other interruption to
            // prevent the completion request from happening, try to abort.
            Err(err) if multipart => {
                Self::abort_multipart_upload(&sg, token, &completion_url, &completion_body).await;

                return Err(ShotgunError::UploadError(format!(
                    "Failed to complete multipart upload `{}`. Upload aborted.",
                    err
                )));
            }
            // For the rest of the cases, we should be able to `?` since no extra
            // cleanup steps should required.
            other => other?,
        };

        let completion_status = completion_resp.status();

        match completion_status {
            // The docs mention the completion status being 204 in the narrative
            // text, but the endpoint specs all say 201 is the good one.
            StatusCode::CREATED | StatusCode::NO_CONTENT => {} // Good
            StatusCode::BAD_REQUEST | StatusCode::UNAUTHORIZED | _ => {
                // If the status is 400/401, the request was rejected for some
                // expected-by-shotgun reason.
                // If it's anything *other than 201/204*, the way to handle it
                // will be the same, really: hand it off to `handle_response()`
                // to get the `Err` it should inevitably produce.
                let _ = handle_response::<Value>(completion_resp).await?;
                // If we didn't get an `Err` from `handle_response()`, then what
                // on earth is happening?!
                return Err(ShotgunError::UploadError(format!(
                    "Unexpected status `{}` for upload complete request.",
                    completion_status
                )));
            }
        }

        Ok(())
    }
}

/// Uploads can either be direct to shotgun or to AWS S3.
enum StorageService {
    Shotgun,
    S3,
}

impl FromStr for StorageService {
    type Err = ShotgunError;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "sg" => Ok(StorageService::Shotgun),
            "s3" => Ok(StorageService::S3),
            other => Err(ShotgunError::UploadError(format!(
                "Unexpected storage service `{:?}`.",
                other,
            ))),
        }
    }
}

enum UploadType {
    Attachment,
    Thumbnail,
}

impl FromStr for UploadType {
    type Err = ShotgunError;

    fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "Attachment" => Ok(UploadType::Attachment),
            "Thumbnail" => Ok(UploadType::Thumbnail),
            other => Err(ShotgunError::UploadError(format!(
                "Unexpected upload type `{:?}`.",
                other,
            ))),
        }
    }
}

#[cfg(test)]
mod mock_tests {
    use super::*;
    use crate::{Shotgun, TokenResponse};
    use std::io::Cursor;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_upload_attachment_sg() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": null,
            "storage_service": "sg",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": false
          }},
          "links": {{
            "upload": "{}/api/v1/entity/notes/123456/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "complete_upload": "/api/v1/entity/notes/123456/_upload"
          }}
        }}
        "##,
            mock_server.uri()
        );
        let upload_body = r##"
        {
          "data": {
            "upload_id": "00000000-0000-0000-0000-000000000000",
            "original_filename": "paranorman-poster.jpg"
          },
          "links": {
            "complete_upload": "/api/v1/entity/notes/123456/_upload"
          }
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(upload_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/entity/notes/123456/_upload"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        sg.upload(
            &access_token,
            "Note",
            123456,
            None,
            "paranorman-poster.jpg",
            Cursor::new(file_content),
        )
        .display_name(Some(String::from(
            "Poster art from the release of ParaNorman.",
        )))
        .send()
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_upload_attachment_s3() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": null,
            "storage_service": "s3",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": false
          }},
          "links": {{
            "upload": "{}/aws/bucket/path?long-string-of-aws-stuff=1",
            "complete_upload": "/api/v1/entity/notes/123456/_upload"
          }}
        }}
        "##,
            mock_server.uri()
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/aws/bucket/path"))
            // The AWS flow gives an empty body on the upload step for some reason.
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/entity/notes/123456/_upload"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        sg.upload(
            &access_token,
            "Note",
            123456,
            None,
            "paranorman-poster.jpg",
            Cursor::new(file_content),
        )
        .display_name(Some(String::from(
            "Poster art from the release of ParaNorman.",
        )))
        .send()
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_upload_attachment_sg_bad_tag() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": null,
            "storage_service": "sg",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": false
          }},
          "links": {{
            "upload": "{}/api/v1/entity/notes/123456/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "complete_upload": "/api/v1/entity/notes/123456/_upload"
          }}
        }}
        "##,
            mock_server.uri()
        );
        let upload_body = r##"
        {
          "data": {
            "upload_id": "00000000-0000-0000-0000-000000000000",
            "original_filename": "paranorman-poster.jpg"
          },
          "links": {
            "complete_upload": "/api/v1/entity/notes/123456/_upload"
          }
        }
        "##;

        let complete_body = r##"
        {
          "errors": [
            {
              "id": "00000000000000000000000000000000",
              "status": 400,
              "code": 104,
              "title": "Update failed for [Attachment.tags]: Value is not legal.",
              "source": null,
              "detail": null,
              "meta": null
            }
          ]
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(upload_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/entity/notes/123456/_upload"))
            .respond_with(
                ResponseTemplate::new(400).set_body_raw(complete_body, "application/json"),
            )
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];
        let tags = vec![crate::types::Entity::new("Tag", 666)];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                None,
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .tags(Some(tags))
            .send()
            .await
        {
            Err(ShotgunError::ServerError(errors)) => assert_eq!(errors[0].status, Some(400)),
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    #[tokio::test]
    async fn test_upload_sg_multipart_is_err() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = r##"
        {
          "errors": [
            {
              "id": "00000000000000000000000000000000",
              "status": 400,
              "code": 103,
              "title": "Multi-part is not supported for this upload.",
              "source": {
                "multipart_upload": "not supported for this storage service (sg)."
              },
              "detail": null,
              "meta": null
            }
          ]
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                None,
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .send()
            .await
        {
            Err(ShotgunError::ServerError(errors)) => {
                assert_eq!(errors[0].status, Some(400));
                assert!(errors[0]
                    .source
                    .as_ref()
                    .unwrap()
                    .get("multipart_upload")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .contains("not supported"));
            }

            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    #[tokio::test]
    async fn test_upload_s3_multipart() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": "xxxx",
            "storage_service": "s3",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": true
          }},
          "links": {{
            "complete_upload": "/api/v1/entity/notes/123456/attachments/_upload",
            "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=paranorman-poster.jpg&part_number=2&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=xxxx&upload_type=Attachment"
          }}
        }}
        "##,
            mock_server.uri()
        );

        let get_next_body = format!(
            r##"
        {{
            "links": {{
                "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=2020-09-24_14-17-00.mp4&part_number=3&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=Wp.HwD2uVolDbye8ns2NtUW81ElvVQGTnk7dbs66dambqnb3G30_YcfsiFGWIHFdpFLTKAyDxCYWAxU6A_6mjDXRZdz0tina3pM18NJ9hsqWsmObnkkXp.4yK_nSXf97CkErsZeKqpWCvsYls9p5ew--&upload_type=Attachment",
                "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            }}
        }}
        "##,
            mock_server.uri()
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/attachments/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // No body
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", r##""abc""##))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(get_next_body, "application/json"),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            .respond_with(ResponseTemplate::new(201))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart_abort",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(0) // a good upload should not be aborted.
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        sg.upload(
            &access_token,
            "Note",
            123456,
            // It is not currently possible to do a multipart upload without
            // specifying a field name.
            // This should be possible once SG-20292 has been closed in some
            // future release of Shotgun.
            // <https://support.shotgunsoftware.com/hc/en-us/requests/117070>
            Some("attachments"),
            "paranorman-poster.jpg",
            Cursor::new(file_content),
        )
        .multipart(true)
        .send()
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn test_upload_s3_multipart_abort_next_part_unavailable_is_err() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": "xxxx",
            "storage_service": "s3",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": true
          }},
          "links": {{
            "complete_upload": "/api/v1/entity/notes/123456/attachments/_upload",
            "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=paranorman-poster.jpg&part_number=2&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=xxxx&upload_type=Attachment"
          }}
        }}
        "##,
            mock_server.uri()
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/attachments/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // No body
            .respond_with(ResponseTemplate::new(200).insert_header("etag", r##""abc""##))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart",
            ))
            .respond_with(
                // Simulating shotgun going AWOL part of the way through the flow
                ResponseTemplate::new(503),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart_abort",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        // We need a block of bytes large enough to span 2 chunks
        let file_content: Vec<u8> = vec![0; (5 * 1024 * 1024) + 100 * 1024];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                // It is not currently possible to do a multipart upload without
                // specifying a field name.
                // This should be possible once SG-20292 has been closed in some
                // future release of Shotgun.
                // <https://support.shotgunsoftware.com/hc/en-us/requests/117070>
                Some("attachments"),
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .chunk_size(5 * 1024 * 1024)
            .send()
            .await
        {
            Err(ShotgunError::UploadError(msg))
                if msg.contains("Failed to get next upload info") => {}
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    #[tokio::test]
    async fn test_upload_s3_multipart_abort_upload_unavailable_is_err() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": "xxxx",
            "storage_service": "s3",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": true
          }},
          "links": {{
            "complete_upload": "/api/v1/entity/notes/123456/attachments/_upload",
            "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=paranorman-poster.jpg&part_number=2&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=xxxx&upload_type=Attachment"
          }}
        }}
        "##,
            mock_server.uri()
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/attachments/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // Simulating AWS being unavailable
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart",
            ))
            .respond_with(
                // Simulating shotgun going AWOL part of the way through the flow
                ResponseTemplate::new(503),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart_abort",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        // We need a block of bytes large enough to span 2 chunks
        let file_content: Vec<u8> = vec![0; (5 * 1024 * 1024) + 100 * 1024];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                // It is not currently possible to do a multipart upload without
                // specifying a field name.
                // This should be possible once SG-20292 has been closed in some
                // future release of Shotgun.
                // <https://support.shotgunsoftware.com/hc/en-us/requests/117070>
                Some("attachments"),
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .chunk_size(5 * 1024 * 1024)
            .send()
            .await
        {
            Err(ShotgunError::UploadError(msg)) if msg.contains("Failed to upload chunk") => {}
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    #[tokio::test]
    async fn test_upload_s3_multipart_abort_complete_unavailable_is_err() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": "xxxx",
            "storage_service": "s3",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": true
          }},
          "links": {{
            "complete_upload": "/api/v1/entity/notes/123456/attachments/_upload",
            "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=paranorman-poster.jpg&part_number=2&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=xxxx&upload_type=Attachment"
          }}
        }}
        "##,
            mock_server.uri()
        );

        let get_next_body = format!(
            r##"
        {{
            "links": {{
                "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=2020-09-24_14-17-00.mp4&part_number=3&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=Wp.HwD2uVolDbye8ns2NtUW81ElvVQGTnk7dbs66dambqnb3G30_YcfsiFGWIHFdpFLTKAyDxCYWAxU6A_6mjDXRZdz0tina3pM18NJ9hsqWsmObnkkXp.4yK_nSXf97CkErsZeKqpWCvsYls9p5ew--&upload_type=Attachment",
                "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            }}
        }}
        "##,
            mock_server.uri()
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/attachments/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // No body
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", r##""abc""##))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(get_next_body, "application/json"),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // Simulate Shotgun being unavailable for the "complete" request.
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart_abort",
            ))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                // It is not currently possible to do a multipart upload without
                // specifying a field name.
                // This should be possible once SG-20292 has been closed in some
                // future release of Shotgun.
                // <https://support.shotgunsoftware.com/hc/en-us/requests/117070>
                Some("attachments"),
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .send()
            .await
        {
            Err(ShotgunError::UploadError(msg)) if msg.contains("aborted") => {}
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    /// This test is identical to
    /// `test_upload_multipart_abort_complete_unavailable_is_err()` except that
    /// *the abort endpoint is also unavailable*.
    /// If the abort fails, it is not great but it shouldn't change the error we
    /// report - we still want the caller to get the error that lead up to the
    /// abort attempt.
    #[tokio::test]
    async fn test_upload_s3_multipart_abort_endpoint_unavailable_does_not_change_outcome() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;
        let init_body = format!(
            r##"
        {{
          "data": {{
            "timestamp": "2020-11-17T03:01:01Z",
            "upload_type": "Attachment",
            "upload_id": "xxxx",
            "storage_service": "s3",
            "original_filename": "paranorman-poster.jpg",
            "multipart_upload": true
          }},
          "links": {{
            "complete_upload": "/api/v1/entity/notes/123456/attachments/_upload",
            "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=paranorman-poster.jpg&part_number=2&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=xxxx&upload_type=Attachment"
          }}
        }}
        "##,
            mock_server.uri()
        );

        let get_next_body = format!(
            r##"
        {{
            "links": {{
                "get_next_part": "/api/v1/entity/notes/123456/attachments/_upload/multipart?filename=2020-09-24_14-17-00.mp4&part_number=3&timestamp=2020-11-22T01%3A28%3A51Z&upload_id=Wp.HwD2uVolDbye8ns2NtUW81ElvVQGTnk7dbs66dambqnb3G30_YcfsiFGWIHFdpFLTKAyDxCYWAxU6A_6mjDXRZdz0tina3pM18NJ9hsqWsmObnkkXp.4yK_nSXf97CkErsZeKqpWCvsYls9p5ew--&upload_type=Attachment",
                "upload": "{}/api/v1/entity/notes/123456/attachments/_upload?expiration=1605582076&filename=paranorman-poster.jpg&signature=xxxx&user_id=0000&user_type=ApiUser",
            }}
        }}
        "##,
            mock_server.uri()
        );

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            // Worth noting shotgun will normalize the entity name into
            // lower-case plural in the urls it generates but this first "init"
            // request uses the entity name we pass into `upload()` as-is.
            .and(path("/api/v1/entity/Note/123456/attachments/_upload"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(init_body, "application/json"))
            .mount(&mock_server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // No body
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", r##""abc""##))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_raw(get_next_body, "application/json"),
            )
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/entity/notes/123456/attachments/_upload"))
            // Simulate Shotgun being unavailable for the "complete" request.
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path(
                "/api/v1/entity/notes/123456/attachments/_upload/multipart_abort",
            ))
            // Shotgun is still in distress.
            .respond_with(ResponseTemplate::new(503))
            .expect(1)
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                // It is not currently possible to do a multipart upload without
                // specifying a field name.
                // This should be possible once SG-20292 has been closed in some
                // future release of Shotgun.
                // <https://support.shotgunsoftware.com/hc/en-us/requests/117070>
                Some("attachments"),
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .send()
            .await
        {
            Err(ShotgunError::UploadError(msg)) if msg.contains("aborted") => {}
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    #[tokio::test]
    async fn test_upload_s3_multipart_small_chunk_size_is_err() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                Some("attachments"),
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .chunk_size((5 * 1024 * 1024) - 1) // Too small
            .send()
            .await
        {
            Err(ShotgunError::UploadError(msg)) if msg.contains("chunk size must be between") => {}
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }

    #[tokio::test]
    async fn test_upload_s3_multipart_large_chunk_size_is_err() {
        let mock_server = MockServer::start().await;

        let auth_body = r##"
        {
          "token_type": "Bearer",
          "access_token": "xxxx",
          "expires_in": 600,
          "refresh_token": "xxxx"
        }
        "##;

        Mock::given(method("POST"))
            .and(path("/api/v1/auth/access_token"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(auth_body, "application/json"))
            .mount(&mock_server)
            .await;

        let sg = Shotgun::new(mock_server.uri(), None, None).unwrap();

        let TokenResponse { access_token, .. }: TokenResponse = sg
            .authenticate_user("nbabcock", "iCdEAD!ppl")
            .await
            .unwrap();

        let file_content: Vec<u8> = vec![];

        match sg
            .upload(
                &access_token,
                "Note",
                123456,
                Some("attachments"),
                "paranorman-poster.jpg",
                Cursor::new(file_content),
            )
            .multipart(true)
            .chunk_size((500 * 1024 * 1024) + 1) // Too big
            .send()
            .await
        {
            Err(ShotgunError::UploadError(msg)) if msg.contains("chunk size must be between") => {}
            other => {
                println!("{:?}", other);
                unreachable!()
            }
        }
    }
}
