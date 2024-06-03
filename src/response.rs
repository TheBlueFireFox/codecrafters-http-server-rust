#![allow(dead_code)]
use std::collections::HashMap;

use crate::header;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Forbidden,
    NotFound,
    InternalServerError,
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
    pub headers: HashMap<String, String>,
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

        for (key, value) in &self.headers {
            buf.extend_from_slice(key.as_bytes());
            buf.extend_from_slice(": ".as_bytes());
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(END_LINE.as_bytes());
        }
        buf.extend_from_slice(END_LINE.as_bytes());

        // TODO: body
        if let Some(body) = &self.body {
            buf.extend_from_slice(body);
        }
    }
}

const END_LINE: &str = "\r\n";

#[cfg(test)]
mod test {
    use itertools::Itertools;

    use super::*;

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
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

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
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "text/plain".to_string());

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
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nSomebody once told me!".as_bytes();

        assert_eq!(exp, buffer);
    }
}
