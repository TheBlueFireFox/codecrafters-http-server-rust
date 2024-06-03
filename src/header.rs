#![allow(dead_code)]
// // Request line
// GET
// /user-agent
// HTTP/1.1
// \r\n
//
// // Headers
// Host: localhost:4221\r\n
// User-Agent: foobar/1.2.3\r\n  // Read this value
// Accept: */*\r\n
// \r\n
//
// // Request body (empty)

use std::{collections::HashMap, fmt::Debug};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

impl From<&str> for Method {
    fn from(value: &str) -> Self {
        match value {
            "GET" => Self::Get,
            "POST" => Self::Post,
            _ => unimplemented!("This method type has not been implemented"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Http11,
}

impl Debug for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http11 => write!(f, "HTTP/1.1"),
        }
    }
}

impl Version {
    pub fn text(&self) -> &str {
        match self {
            Version::Http11 => "HTTP/1.1",
        }
    }
}

impl From<&str> for Version {
    fn from(value: &str) -> Self {
        match value {
            "HTTP/1.1" => Self::Http11,
            _ => unimplemented!("This version type has not been implemented"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
    pub sections: Vec<String>,
    pub query: Option<String>,
}

impl From<&str> for Url {
    fn from(value: &str) -> Self {
        let (uri, query) = match value.split_once('?') {
            None => (value, None),
            Some((uri, query)) => (uri, Some(query.to_string())),
        };

        let mut parts = vec![];
        if uri == "/" {
            parts.push("/".to_string());
            return Self {
                sections: parts,
                query,
            };
        }
        for sections in uri.split('/') {
            // this is root
            if sections.is_empty() {
                continue;
            }
            parts.push(sections.to_string());
        }

        Self {
            sections: parts,
            query,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub method: Method,
    pub url: Url,
    pub version: Version,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub header: Header,
    pub body: Option<Vec<u8>>,
}

pub fn parse(buf: &[u8]) -> anyhow::Result<(Request, &[u8])> {
    match parsing::parse(buf) {
        Ok((res, req)) => Ok((req, res)),
        Err(err) => Err(anyhow::format_err!("{:?}", err)),
    }
}

mod parsing {
    use super::*;
    use nom::{
        branch::alt,
        bytes::complete::{tag_no_case, take_till, take_until},
        character::complete::char,
        error::{context, VerboseError},
        multi::fold_many0,
        sequence::{terminated, tuple},
    };

    pub type Result<T, V> = nom::IResult<T, V, VerboseError<T>>;

    pub fn parse(buf: &[u8]) -> Result<&[u8], Request> {
        let (mut res, header) = parse_header(buf)?;

        // get body
        let mut body = None;
        if res.starts_with("\r\n".as_bytes()) {
            res = &res[2..];
            body = Some(res.to_vec());
            res = &[];
        }

        Ok((res, Request { header, body }))
    }

    fn parse_header(buf: &[u8]) -> Result<&[u8], Header> {
        let (buf, ((method, url, version), headers)) = context(
            "header",
            tuple((
                terminated(parse_request_line, parse_new_line),
                parse_header_lines,
            )),
        )(buf)?;

        Ok((
            buf,
            Header {
                method,
                url,
                version,
                headers,
            },
        ))
    }

    fn parse_header_lines(buf: &[u8]) -> Result<&[u8], HashMap<String, String>> {
        context(
            "header lines",
            fold_many0(
                terminated(parse_header_line, parse_new_line),
                HashMap::new,
                |mut map: HashMap<_, _>, (k, v)| {
                    map.insert(k, v);
                    map
                },
            ),
        )(buf)
    }

    fn parse_header_line(buf: &[u8]) -> Result<&[u8], (String, String)> {
        let (res, (key, _, _, value)) = context(
            "header line",
            tuple((
                take_till(is_colon),
                char(':'),
                char(' '),
                take_till(|c| c == b'\r'),
            )),
        )(buf)?;

        let to_string = |s| std::str::from_utf8(s).expect("unable to parse").to_string();

        Ok((res, (to_string(key), to_string(value))))
    }

    fn parse_new_line(buf: &[u8]) -> Result<&[u8], &[u8]> {
        let (res, _) = context("new line", tuple((char('\r'), char('\n'))))(buf)?;

        Ok((res, &buf[2..]))
    }

    fn is_colon(c: u8) -> bool {
        c == b':'
    }

    fn parse_request_line(buf: &[u8]) -> Result<&[u8], (Method, Url, Version)> {
        let (res, (method, _, url, _, version)) = context(
            "request line",
            tuple((parse_method, char(' '), parse_url, char(' '), parse_version)),
        )(buf)?;

        Ok((res, (method, url, version)))
    }

    fn parse_url(buf: &[u8]) -> Result<&[u8], Url> {
        let (res, url) = take_until(" ")(buf)?;
        Ok((res, std::str::from_utf8(url).expect("url not valid").into()))
    }

    fn parse_version(buf: &[u8]) -> Result<&[u8], Version> {
        let (res, s) = context("Version", alt((tag_no_case("HTTP/1.1".as_bytes()),)))(buf)?;

        let s = std::str::from_utf8(s)
            .expect("unable to parse into utf8")
            .to_uppercase();

        Ok((res, s[..].into()))
    }

    fn parse_method(buf: &[u8]) -> Result<&[u8], Method> {
        let (res, s) = context(
            "Method",
            alt((
                tag_no_case("GET".as_bytes()),
                tag_no_case("POST".as_bytes()),
            )),
        )(buf)?;

        let s = std::str::from_utf8(s)
            .expect("unable to parse into utf8")
            .to_uppercase();

        Ok((res, s[..].into()))
    }

    #[cfg(test)]
    mod test {

        use super::*;

        #[test]
        fn parse_method() {
            let all = [
                ("GET", Method::Get),
                ("GeT", Method::Get),
                ("POST", Method::Post),
                ("PoST", Method::Post),
            ];

            for (input, exp) in all {
                let (_, m) = super::parse_method(input.as_bytes()).expect("unable to parse");
                assert_eq!(m, exp);
            }
        }

        #[test]
        fn parse_version() {
            let all = [("HTTP/1.1", Version::Http11), ("hTTP/1.1", Version::Http11)];

            for (input, exp) in all {
                let (_, m) = super::parse_version(input.as_bytes()).expect("unable to parse");
                assert_eq!(m, exp);
            }
        }

        #[test]
        fn parse_request_line() {
            let s = "GET / HTTP/1.1\r\n";
            let (_, Request { header, body }) = parse(s.as_bytes()).expect("able to parse");
            assert_eq!(body, None);

            assert_eq!(header.method, Method::Get);
            assert_eq!(header.version, Version::Http11);
            assert_eq!(header.url, "/".into());
            assert!(header.headers.is_empty());
        }

        #[test]
        fn parse_request_line_with_query() {
            let s = "GET /something?foo=2 HTTP/1.1\r\n";
            let (res, Request { header, body }) = parse(s.as_bytes()).expect("able to parse");

            assert!(res.is_empty());
            assert_eq!(body, None);

            assert_eq!(header.method, Method::Get);
            assert_eq!(header.version, Version::Http11);
            assert_eq!(header.url, "/something?foo=2".into());
            assert!(header.headers.is_empty());
        }

        #[test]
        fn parse_header_single_line() {
            let input = "Host: localhost:4221\r\n";
            let (_, (host, localhost)) =
                super::parse_header_line(input.as_bytes()).expect("able to parse");

            assert_eq!("Host", host);
            assert_eq!("localhost:4221", localhost);
        }

        #[test]
        fn parse_header_lines() {
            let input = "Host: localhost:4221\r\nUser-Agent: foobar/1.2.3\r\nAccept: */*\r\n\r\n";
            let (_, headers) = super::parse_header_lines(input.as_bytes()).expect("able to parse");

            assert_eq!(Some(&"localhost:4221".to_string()), headers.get("Host"));
            assert_eq!(Some(&"*/*".to_string()), headers.get("Accept"));
        }

        #[test]
        fn parse_full_request() {
            let input = "GET /user-agent HTTP/1.1\r\nHost: localhost:4221\r\nUser-Agent: foobar/1.2.3\r\nAccept: */*\r\n\r\nSome Body";
            let (res, Request { header, body }) = parse(input.as_bytes()).expect("able to parse");
            assert_eq!(res.len(), 0);

            assert_eq!(Some("Some Body".as_bytes().to_vec()), body);

            assert_eq!(header.method, Method::Get);
            assert_eq!(header.version, Version::Http11);
            assert_eq!(header.url, "/user-agent".into());

            assert_eq!(
                Some(&"localhost:4221".to_string()),
                header.headers.get("Host")
            );
        }
    }
}
