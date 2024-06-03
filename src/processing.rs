use crate::{
    header::{Request, Version},
    response::{ContentType, Headers, Response, Status},
};

pub async fn process(request: &Request) -> Response {
    match &request.header.url.sections[0][..] {
        "/" => root(request).await,
        "echo" => echo(request).await,
        "user-agent" => user_agent(request).await,
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
    ok()
}

async fn echo(request: &Request) -> Response {
    let sections = &request.header.url.sections;

    if sections.len() == 1 {
        return not_found(request).await;
    }

    let mut resp = ok();

    let ct = Headers::ContentType(ContentType::TextPlain);

    resp.headers.insert(ct);
    resp.body = Some(sections[1].as_bytes().to_vec());
    resp
}

async fn user_agent(request: &Request) -> Response {
    let mut resp = ok();
    resp.body = Some(request.header.headers["User-Agent"].as_bytes().to_vec());
    resp
}

fn ok() -> Response {
    Response {
        version: Version::Http11,
        status: Status::Ok,
        headers: Default::default(),
        body: None,
    }
}
