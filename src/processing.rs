use std::path::PathBuf;

use tokio::fs::{read, try_exists};

use crate::{
    request::{Method, Request, Version},
    response::{ContentType, Headers, Response, Status},
};

pub struct Router {
    pub directory: Option<String>,
}

impl Router {
    pub async fn process(&self, request: &Request) -> Response {
        match &request.header.url.sections[0][..] {
            "/" => self.root(request),
            "echo" => self.echo(request),
            "user-agent" => self.user_agent(request),
            "files" => self.files(request).await,
            _ => Self::not_found(request),
        }
    }

    fn root(&self, request: &Request) -> Response {
        Self::ok(request)
    }

    fn echo(&self, request: &Request) -> Response {
        let sections = &request.header.url.sections;

        if sections.len() == 1 {
            return Self::not_found(request);
        }

        let mut resp = Self::ok(request);

        let ct = Headers::ContentType(ContentType::TextPlain);
        resp.headers.insert(ct);
        resp.body = Some(sections[1].as_bytes().to_vec());
        resp
    }

    fn user_agent(&self, request: &Request) -> Response {
        let mut resp = Self::ok(request);
        let ct = Headers::ContentType(ContentType::TextPlain);

        resp.headers.insert(ct);
        resp.body = Some(request.header.headers["user-agent"].as_bytes().to_vec());
        resp
    }

    async fn files(&self, request: &Request) -> Response {
        match request.header.method {
            Method::Get => self.files_get(request).await,
            Method::Post => self.files_post(request).await,
        }
    }

    async fn files_get(&self, request: &Request) -> Response {
        let sections = &request.header.url.sections;

        if sections.len() == 1 {
            return Self::not_found(request);
        }
        match &self.directory {
            None => Self::internal_server_error(request),
            Some(directory) => {
                let mut file = PathBuf::from(&directory[..]);
                file.push(&sections[1]);

                match try_exists(&file).await {
                    Err(_) => Self::internal_server_error(request),
                    Ok(false) => Self::not_found(request),
                    Ok(true) => match read(file).await {
                        Err(_) => Self::internal_server_error(request),
                        Ok(content) => {
                            let mut resp = Self::ok(request);
                            let ct = Headers::ContentType(ContentType::OctentStream);
                            resp.headers.insert(ct);
                            resp.body = Some(content);

                            resp
                        }
                    },
                }
            }
        }
    }

    async fn files_post(&self, request: &Request) -> Response {
        let sections = &request.header.url.sections;

        if sections.len() == 1 {
            return Self::not_found(request);
        }
        match &self.directory {
            None => Self::internal_server_error(request),
            Some(directory) => {
                let mut path = PathBuf::from(directory);
                path.push(&sections[1]);

                match &request.body {
                    None => Self::internal_server_error(request),
                    Some(content) => {
                        if let Err(err) = tokio::fs::write(path, content).await {
                            eprintln!("error {:?}", err);
                            return Self::internal_server_error(request);
                        }
                        Self::created(request)
                    }
                }
            }
        }
    }

    fn created(request: &Request) -> Response {
        Response {
            version: Version::Http11,
            status: Status::Created,
            headers: Default::default(),
            accept_encoding: request.header.accept_encoding,
            body: None,
        }
    }

    fn ok(request: &Request) -> Response {
        Response {
            version: Version::Http11,
            status: Status::Ok,
            headers: Default::default(),
            accept_encoding: request.header.accept_encoding,
            body: None,
        }
    }

    fn not_found(request: &Request) -> Response {
        Response {
            version: Version::Http11,
            status: Status::NotFound,
            headers: Default::default(),
            accept_encoding: request.header.accept_encoding,
            body: None,
        }
    }

    fn internal_server_error(request: &Request) -> Response {
        Response {
            version: Version::Http11,
            status: Status::InternalServerError,
            headers: Default::default(),
            accept_encoding: request.header.accept_encoding,
            body: None,
        }
    }
}
