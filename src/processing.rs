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
            _ => Self::not_found(),
        }
    }

    fn root(&self, _request: &Request) -> Response {
        Self::ok()
    }

    fn echo(&self, request: &Request) -> Response {
        let sections = &request.header.url.sections;

        if sections.len() == 1 {
            return Self::not_found();
        }

        let mut resp = Self::ok();

        let ct = Headers::ContentType(ContentType::TextPlain);
        resp.headers.insert(ct);
        resp.body = Some(sections[1].as_bytes().to_vec());
        resp
    }

    fn user_agent(&self, request: &Request) -> Response {
        let mut resp = Self::ok();
        let ct = Headers::ContentType(ContentType::TextPlain);

        resp.headers.insert(ct);
        resp.body = Some(request.header.headers["User-Agent"].as_bytes().to_vec());
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
            return Self::not_found();
        }
        match &self.directory {
            None => Self::internal_server_error(),
            Some(directory) => {
                let mut file = PathBuf::from(&directory[..]);
                file.push(&sections[1]);

                match try_exists(&file).await {
                    Err(_) => Self::internal_server_error(),
                    Ok(false) => Self::not_found(),
                    Ok(true) => match read(file).await {
                        Err(_) => Self::internal_server_error(),
                        Ok(content) => {
                            let mut resp = Self::ok();
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
            return Self::not_found();
        }
        match &self.directory {
            None => Self::internal_server_error(),
            Some(directory) => {
                let mut path = PathBuf::from(directory);
                path.push(&sections[1]);

                match &request.body {
                    None => Self::internal_server_error(),
                    Some(content) => {
                        if let Err(err) = tokio::fs::write(path, content).await {
                            eprintln!("error {:?}", err);
                            return Self::internal_server_error();
                        }
                        Self::created()
                    }
                }
            }
        }
    }

    fn created() -> Response {
        Response {
            version: Version::Http11,
            status: Status::Created,
            headers: Default::default(),
            body: None,
        }
    }

    fn ok() -> Response {
        Response {
            version: Version::Http11,
            status: Status::Ok,
            headers: Default::default(),
            body: None,
        }
    }

    fn not_found() -> Response {
        Response {
            version: Version::Http11,
            status: Status::NotFound,
            headers: Default::default(),
            body: None,
        }
    }

    fn internal_server_error() -> Response {
        Response {
            version: Version::Http11,
            status: Status::InternalServerError,
            headers: Default::default(),
            body: None,
        }
    }
}
