//! A minimal HTTP/1.1 client over a TCP stream, for the `ClickHouse` HTTP interface only.
//!
//! The runner adds no HTTP crate (issue #589): it sends one `POST` per connection with
//! `Connection: close` and reads the response to the end of the stream, so it never needs
//! keep-alive or pipelining. Only plain `http://` endpoints are supported: the CI server runs
//! as a local service.

use std::fmt::Write as _;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// How long the runner waits for the server to accept, read or answer a request.
const TIMEOUT: Duration = Duration::from_mins(2);

/// The host and port of an `http://host:port` endpoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    /// The endpoint of `url`, `http://host:port` with an optional final `/`.
    ///
    /// # Errors
    ///
    /// Why `url` is not such an endpoint.
    pub(crate) fn parse(url: &str) -> Result<Self, String> {
        let invalid = || format!("`{url}` is not an endpoint `http://host:port`");
        let rest = url.strip_prefix("http://").ok_or_else(invalid)?;
        let authority = rest.strip_suffix('/').unwrap_or(rest);
        let (host, port) = authority.rsplit_once(':').ok_or_else(invalid)?;
        if host.is_empty() || host.contains('/') {
            return Err(invalid());
        }
        let port = port.parse().map_err(|_| invalid())?;
        Ok(Self {
            host: host.to_owned(),
            port,
        })
    }
}

/// The status and body of a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Response {
    /// The status code, such as 200.
    pub(crate) status: u16,
    /// The body, without transfer encoding.
    pub(crate) body: Vec<u8>,
}

/// Sends `POST /?<query>` with `body` to `endpoint` and reads the whole response.
///
/// # Errors
///
/// Why the request could not be sent or the response could not be read.
pub(crate) fn post(
    endpoint: &Endpoint,
    query: &[(&str, &str)],
    body: &[u8],
) -> Result<Response, String> {
    let transport = |error: std::io::Error| {
        format!(
            "the request to {}:{} failed: {error}",
            endpoint.host, endpoint.port
        )
    };
    let mut stream =
        TcpStream::connect((endpoint.host.as_str(), endpoint.port)).map_err(transport)?;
    stream.set_read_timeout(Some(TIMEOUT)).map_err(transport)?;
    stream.set_write_timeout(Some(TIMEOUT)).map_err(transport)?;
    let head = format!(
        "POST /?{} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        encode_query(query),
        endpoint.host,
        endpoint.port,
        body.len()
    );
    stream.write_all(head.as_bytes()).map_err(transport)?;
    stream.write_all(body).map_err(transport)?;
    stream.flush().map_err(transport)?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).map_err(transport)?;
    parse_response(&bytes)
}

/// The query string of `pairs`, each name and value percent-encoded.
fn encode_query(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{}={}", percent_encode(name), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// `text` with every byte outside the unreserved characters of RFC 3986 written `%XX`.
fn percent_encode(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

/// The status and decoded body of the raw response `bytes`.
fn parse_response(bytes: &[u8]) -> Result<Response, String> {
    let end = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "the response has no end of headers".to_owned())?;
    let head = String::from_utf8_lossy(&bytes[..end]);
    let mut lines = head.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse().ok())
        .ok_or_else(|| "the response has no status line".to_owned())?;
    let chunked = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("transfer-encoding")
                && value.trim().eq_ignore_ascii_case("chunked")
        })
    });
    let raw = &bytes[end + 4..];
    let body = if chunked {
        decode_chunked(raw)?
    } else {
        raw.to_vec()
    };
    Ok(Response { status, body })
}

/// The body of a `chunked` transfer encoding (RFC 9112 section 7.1); trailers are ignored.
fn decode_chunked(mut raw: &[u8]) -> Result<Vec<u8>, String> {
    let malformed = || "the chunked response body is malformed".to_owned();
    let mut body = Vec::new();
    loop {
        let line_end = raw
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(malformed)?;
        let size_text = String::from_utf8_lossy(&raw[..line_end]);
        let size_text = size_text.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| malformed())?;
        raw = &raw[line_end + 2..];
        if size == 0 {
            return Ok(body);
        }
        let chunk = raw.get(..size).ok_or_else(malformed)?;
        body.extend_from_slice(chunk);
        raw = raw.get(size + 2..).ok_or_else(malformed)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e6_7_10_reads_an_http_endpoint() {
        assert_eq!(
            Endpoint::parse("http://localhost:8123/"),
            Ok(Endpoint {
                host: "localhost".to_owned(),
                port: 8123
            })
        );
        assert!(Endpoint::parse("https://localhost:8123").is_err());
        assert!(Endpoint::parse("http://localhost").is_err());
    }

    #[test]
    fn e6_7_3_percent_encodes_every_reserved_byte_of_a_parameter() {
        assert_eq!(
            encode_query(&[("param_p1", "a b&c=\\N/é")]),
            "param_p1=a%20b%26c%3D%5CN%2F%C3%A9"
        );
    }

    #[test]
    fn e6_7_10_reads_a_response_with_a_length() {
        let response =
            parse_response(b"HTTP/1.1 500 Internal\r\nContent-Length: 3\r\n\r\nbad").unwrap();
        assert_eq!(response.status, 500);
        assert_eq!(response.body, b"bad");
    }

    #[test]
    fn e6_7_10_decodes_a_chunked_response() {
        let raw = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\
                    4\r\n1\t2\n\r\n3;x=y\r\nabc\r\n0\r\n\r\n";
        let response = parse_response(raw).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"1\t2\nabc");
    }

    #[test]
    fn e6_7_10_rejects_a_truncated_chunk() {
        assert!(decode_chunked(b"9\r\nabc").is_err());
    }
}
