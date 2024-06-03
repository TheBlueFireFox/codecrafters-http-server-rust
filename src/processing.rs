use std::collections::BTreeSet;

use crate::{
    header::{Request, Version},
    response::{ContentType, Headers, Response, Status},
};

pub async fn process(request: &Request) -> Response {
    match &request.header.url.sections[0][..] {
        "/" => root(request).await,
        "echo" => echo(request).await,
        _ => not_found(request).await,
    }
}

async fn not_found(_request: &Request) -> Response {
    Response {
        version: Version::Http11,
        status: Status::NotFound,
        headers: Default::default(),
        body: None,
    }
}

async fn root(_request: &Request) -> Response {
    Response {
        version: Version::Http11,
        status: Status::Ok,
        headers: Default::default(),
        body: None,
    }
}

async fn echo(request: &Request) -> Response {
    let sections = &request.header.url.sections;

    if sections.len() == 1 {
        return not_found(request).await;
    }

    let mut headers = BTreeSet::new();
    headers.insert(Headers::ContentType(ContentType::TextPlain));

    Response {
        version: Version::Http11,
        status: Status::Ok,
        headers,
        body: Some(sections[1].as_bytes().to_vec()),
    }
}
