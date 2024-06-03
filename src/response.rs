#![allow(dead_code)]
use std::collections::BTreeSet;

use crate::header;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Forbidden,
    NotFound,
    InternalServerError,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentType {
    TextPlain,
}

impl ContentType {
    pub fn text(&self) -> &'static str {
        match self {
            ContentType::TextPlain => "text/plain",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Headers {
    ContentType(ContentType),
    ContentLength(usize),
}

impl Headers {
    pub fn text(&self) -> (&'static str, String) {
        match self {
            Headers::ContentType(ct) => ("Content-Type", ct.text().to_string()),
            Headers::ContentLength(size) => ("Content-Length", format!("{}", size)),
        }
    }
}

impl Status {
    pub fn code(&self) -> &str {
        match self {
            Status::Ok => "200",
            Status::Forbidden => "403",
            Status::NotFound => "404",
            Status::InternalServerError => "500",
        }
    }

    pub fn reason(&self) -> &str {
        match self {
            Status::Ok => "OK",
            Status::Forbidden => "Forbidden",
            Status::NotFound => "Not Found",
            Status::InternalServerError => "Internal Server Error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub version: header::Version,
    pub status: Status,
    pub headers: BTreeSet<Headers>,
    pub body: Option<Vec<u8>>,
}

impl Response {
    pub fn write(&self, buf: &mut Vec<u8>) {
        // HTTP/1.1 200 OK\r\n\r\n
        buf.extend_from_slice(self.version.text().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.status.code().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.status.reason().as_bytes());
        buf.extend_from_slice(END_LINE.as_bytes());

        let insert = |key: &str, value: &str, buf: &mut Vec<u8>| {
            buf.extend_from_slice(key.as_bytes());
            buf.extend_from_slice(": ".as_bytes());
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(END_LINE.as_bytes());
        };

        for header in &self.headers {
            if let Headers::ContentLength(_) = header {
                continue;
            }
            let (key, value) = header.text();
            insert(key, &value, buf);
        }

        match &self.body {
            None => buf.extend_from_slice(END_LINE.as_bytes()),
            Some(body) => {
                let (key, value) = Headers::ContentLength(body.len()).text();
                insert(key, &value, buf);
                buf.extend_from_slice(END_LINE.as_bytes());
                buf.extend_from_slice(body);
            }
        }
    }
}

const END_LINE: &str = "\r\n";

#[cfg(test)]
mod test {
    use super::*;
    use itertools::Itertools;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_status_line() {
        let res = Response {
            version: header::Version::Http11,
            status: Status::Ok,
            headers: Default::default(),
            body: None,
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
            version: header::Version::Http11,
            status: Status::Ok,
            headers,
            body: None,
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
            version: header::Version::Http11,
            status: Status::Ok,
            headers,
            body: Some(
                "Somebody once told me!"
                    .as_bytes()
                    .iter()
                    .copied()
                    .collect_vec(),
            ),
        };
        let mut buffer = Vec::new();
        res.write(&mut buffer);
        let exp =
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 22\r\n\r\nSomebody once told me!".as_bytes();

        assert_eq!(exp, buffer);
    }
}
