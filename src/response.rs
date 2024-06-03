#![allow(dead_code)]
use std::{collections::BTreeSet, io::Write};

use libflate::gzip::Encoder;

use crate::request::{Encoding, Version};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentType {
    TextPlain,
    OctentStream,
}

impl ContentType {
    pub fn text(&self) -> &'static str {
        match self {
            ContentType::TextPlain => "text/plain",
            ContentType::OctentStream => "application/octet-stream",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Headers {
    ContentType(ContentType),
    ContentLength(usize),
    AcceptEncoding(Encoding),
    ContentEncoding(Encoding),
}

impl Headers {
    pub fn text(&self) -> (&'static str, String) {
        match self {
            Headers::ContentType(ct) => ("Content-Type", ct.text().to_string()),
            Headers::ContentLength(size) => ("Content-Length", format!("{}", size)),
            Headers::AcceptEncoding(enc) => ("Accept-Encoding", enc.text().to_string()),
            Headers::ContentEncoding(enc) => ("Content-Encoding", enc.text().to_string()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Created,
    Forbidden,
    NotFound,
    InternalServerError,
}

impl Status {
    pub fn code(&self) -> &str {
        match self {
            Status::Ok => "200",
            Status::Created => "201",
            Status::Forbidden => "403",
            Status::NotFound => "404",
            Status::InternalServerError => "500",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Status::Ok => "OK",
            Status::Created => "Created",
            Status::Forbidden => "Forbidden",
            Status::NotFound => "Not Found",
            Status::InternalServerError => "Internal Server Error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub version: Version,
    pub status: Status,
    pub headers: BTreeSet<Headers>,
    pub accept_encoding: Option<Encoding>,
    pub body: Option<Vec<u8>>,
}

impl Response {
    pub fn write(&self, buf: &mut Vec<u8>) {
        self.handle_response_line(buf);
        self.handle_headers(buf);
        self.handle_body(buf);
    }

    fn insert(key: &str, value: &str, buf: &mut Vec<u8>) {
        buf.extend_from_slice(key.as_bytes());
        buf.extend_from_slice(": ".as_bytes());
        buf.extend_from_slice(value.as_bytes());
        buf.extend_from_slice(END_LINE.as_bytes());
    }

    fn handle_response_line(&self, buf: &mut Vec<u8>) {
        // HTTP/1.1 200 OK\r\n\r\n
        buf.extend_from_slice(self.version.text().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.status.code().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.status.reason().as_bytes());
        buf.extend_from_slice(END_LINE.as_bytes());
    }

    fn handle_headers(&self, buf: &mut Vec<u8>) {
        for header in &self.headers {
            if let Headers::ContentLength(_) = header {
                continue;
            }
            let (key, value) = header.text();
            Self::insert(key, &value, buf);
        }
    }

    fn handle_body(&self, buf: &mut Vec<u8>) {
        let handle_writing = |buf: &mut Vec<u8>, body: &[u8]| {
            let (key, value) = Headers::ContentLength(body.len()).text();
            Self::insert(key, &value, buf);
            buf.extend_from_slice(END_LINE.as_bytes());
            buf.extend_from_slice(body);
        };

        match &self.body {
            None => buf.extend_from_slice(END_LINE.as_bytes()),
            Some(body) => match self.accept_encoding {
                None => handle_writing(buf, body),
                Some(enc @ Encoding::Gzip) => {
                    let (key, value) = Headers::ContentEncoding(enc).text();
                    Self::insert(key, &value, buf);

                    let mut e = Encoder::new(Vec::new()).expect("unable to create encoder");
                    e.write_all(body)
                        .expect("able to correctly write compressed body");
                    let cbody = e.finish().into_result().expect("unable to compress");
                    handle_writing(buf, &cbody);
                }
            },
        }
    }
}

const END_LINE: &str = "\r\n";

#[cfg(test)]
mod test {
    use crate::request::Version;

    use super::*;
    use itertools::Itertools;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_status_line() {
        let res = Response {
            version: Version::Http11,
            status: Status::Ok,
            headers: Default::default(),
            body: None,
            accept_encoding: None,
        };
        let mut buffer = Vec::new();
        res.write(&mut buffer);
        let exp = "HTTP/1.1 200 OK\r\n\r\n".as_bytes();

        assert_eq!(exp, buffer);
    }

    #[test]
    fn test_with_header() {
        let mut headers = BTreeSet::new();
        headers.insert(Headers::ContentType(ContentType::TextPlain));

        let res = Response {
            version: Version::Http11,
            status: Status::Ok,
            headers,
            body: None,

            accept_encoding: None,
        };
        let mut buffer = Vec::new();
        res.write(&mut buffer);
        let exp = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n".as_bytes();

        assert_eq!(exp, buffer);
    }

    #[test]
    fn test_with_body() {
        let mut headers = BTreeSet::new();
        headers.insert(Headers::ContentType(ContentType::TextPlain));

        let res = Response {
            version: Version::Http11,
            status: Status::Ok,
            headers,
            body: Some(
                "Somebody once told me!"
                    .as_bytes()
                    .iter()
                    .copied()
                    .collect_vec(),
            ),
            accept_encoding: None,
        };
        let mut buffer = Vec::new();
        res.write(&mut buffer);
        let exp =
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 22\r\n\r\nSomebody once told me!".as_bytes();

        assert_eq!(exp, buffer);
    }
}
