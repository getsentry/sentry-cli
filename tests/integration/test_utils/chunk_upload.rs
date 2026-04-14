//! Utilities for chunk upload tests.
use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, Read as _, Write as _};
use std::str;
use std::sync::LazyLock;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use mockito::Request;
use regex::bytes::Regex;
use sha1_smol::Sha1;

/// This regex is used to extract the boundary from the content-type header.
/// We need to match the boundary, since it changes with each request.
/// The regex matches the format as specified in
/// https://www.w3.org/Protocols/rfc1341/7_2_Multipart.html.
static CONTENT_TYPE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"^multipart\/form-data; boundary=(?<boundary>[\w'\(\)+,\-\.\/:=? ]{0,69}[\w'\(\)+,\-\.\/:=?])$"#
    )
    .expect("Regex is valid")
});

/// A trait which abstracts over accessing headers from a mock request.
/// Allows future compatibility in case we switch to a different mock library.
pub trait HeaderContainer {
    fn header(&self, header_name: &str) -> Vec<&[u8]>;
}

impl HeaderContainer for Request {
    fn header(&self, header_name: &str) -> Vec<&[u8]> {
        self.header(header_name)
            .iter()
            .map(|h| h.as_bytes())
            .collect()
    }
}

/// Split a multipart/form-data body into its constituent chunks.
///
/// The returned vector preserves duplicate chunks. Callers that do not care
/// about multiplicity can collect into a set explicitly.
pub fn split_chunk_body<'b>(
    body: &'b [u8],
    boundary: &str,
) -> Result<Vec<&'b [u8]>, Box<dyn Error>> {
    let escaped_boundary = regex::escape(boundary);

    let inner_body = entire_body_regex(&escaped_boundary)
        .captures(body)
        .ok_or("body does not match multipart form regex")?
        .name("body")
        .expect("the regex has a \"body\" capture group which should always match")
        .as_bytes();

    Ok(boundary_regex(&escaped_boundary)
        .split(inner_body)
        .collect())
}

/// Extract the boundary from a multipart/form-data request content-type header.
/// Returns an error if the content-type header is not present exactly once,
/// if the content-type does not match the multipart/form-data regex, or if the
/// boundary is not valid UTF-8.
pub fn boundary_from_request(request: &impl HeaderContainer) -> Result<&str, Box<dyn Error>> {
    let content_type_headers = request.header("content-type");

    if content_type_headers.len() != 1 {
        return Err(format!(
            "content-type header should be present exactly once, found {} times",
            content_type_headers.len()
        )
        .into());
    }

    let content_type = content_type_headers[0];

    let boundary = CONTENT_TYPE_REGEX
        .captures(content_type)
        .ok_or("content-type does not match multipart/form-data regex")?
        .name("boundary")
        .expect("if the regex matches, the boundary should match as well.")
        .as_bytes();

    Ok(str::from_utf8(boundary)?)
}

/// Given the regex-escaped boundary of a multipart form, return a regex which
/// should match the entire body of the form. The regex includes a named capture
/// group for the body (named "body"), which includes everything from the first starting
/// boundary to the final ending boundary (non-inclusive of the boundaries).
/// May panic if the boundary is not regex-escaped.
fn entire_body_regex(regex_escaped_boundary: &str) -> Regex {
    Regex::new(&format!(
        r#"^--{regex_escaped_boundary}(?<body>(?s-u:.*?))--{regex_escaped_boundary}--\s*$"#
    ))
    .expect("This regex should be valid")
}

/// Given the regex-escaped boundary of a multipart form, return a regex which
/// matches the start of a section of the form.
fn boundary_regex(regex_escaped_boundary: &str) -> Regex {
    Regex::new(&format!(r#"--{regex_escaped_boundary}"#)).expect("This regex should be valid")
}

/// Regex to separate multipart headers from the body (separated by \r\n\r\n).
static HEADER_BODY_SEPARATOR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\r\n\r\n").expect("Regex is valid"));

/// Extract and decompress the file contents from a multipart chunk upload request.
/// Each chunk part has headers followed by a gzip-compressed body. This function
/// strips the multipart headers, decompresses each chunk, preserves duplicate
/// chunk contents, and errors if any multipart part is malformed.
pub fn decompress_chunks(body: &[u8], boundary: &str) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let parts = split_chunk_body(body, boundary)?;
    let total_parts = parts.len();
    let mut decompressed = Vec::with_capacity(total_parts);

    for (index, part) in parts.into_iter().enumerate() {
        // Each part is: \r\nHeaders\r\n\r\n<gzip body>
        // Split on the first \r\n\r\n to separate headers from body.
        let separator = HEADER_BODY_SEPARATOR.find(part).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!(
                    "multipart part {} of {total_parts} is missing \\r\\n\\r\\n header/body separator",
                    index + 1,
                ),)
        })?;

        let compressed = &part[separator.end()..];
        let mut decoder = GzDecoder::new(compressed);
        let mut content = Vec::new();
        decoder.read_to_end(&mut content)?;
        decompressed.push(content);
    }

    Ok(decompressed)
}

/// Count chunks by SHA1 digest while preserving duplicate occurrences.
pub fn chunk_digest_counts<T, I>(chunks: I) -> BTreeMap<String, usize>
where
    I: IntoIterator<Item = T>,
    T: AsRef<[u8]>,
{
    let mut counts = BTreeMap::new();

    for chunk in chunks {
        let mut sha = Sha1::new();
        sha.update(chunk.as_ref());
        *counts.entry(sha.digest().to_string()).or_default() += 1;
    }

    counts
}

fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(data)
        .expect("should write data into gzip encoder");
    encoder.finish().expect("should finish gzip encoding")
}

fn multipart_body(boundary: &str, parts: &[&[u8]]) -> Vec<u8> {
    let mut body = Vec::new();

    for part in parts {
        body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        body.extend_from_slice(
            b"content-disposition: form-data; name=\"file\"; filename=\"chunk\"\r\n",
        );
        body.extend_from_slice(b"content-type: application/octet-stream\r\n\r\n");
        body.extend_from_slice(part);
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    body
}

#[test]
fn decompress_chunks_preserves_duplicate_parts() {
    let boundary = "boundary";
    let chunk = gzip(b"duplicate chunk");
    let body = multipart_body(boundary, &[&chunk, &chunk]);

    let decompressed =
        decompress_chunks(&body, boundary).expect("multipart body should decompress");

    assert_eq!(
        decompressed,
        vec![b"duplicate chunk".to_vec(), b"duplicate chunk".to_vec()]
    );
}

#[test]
fn decompress_chunks_errors_on_part_missing_header_body_separator() {
    let boundary = "boundary";
    let chunk = gzip(b"chunk");
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"content-disposition: form-data; name=\"file\"; filename=\"chunk\"\r\n",
    );
    body.extend_from_slice(b"content-type: application/octet-stream\r\n");
    body.extend_from_slice(&chunk);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let error = decompress_chunks(&body, boundary)
        .expect_err("multipart part without header/body separator should error");

    assert_eq!(
        error.to_string(),
        r"multipart part 1 of 1 is missing \r\n\r\n header/body separator"
    );
}
