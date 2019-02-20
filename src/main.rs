#[allow(unused_imports)]
#[macro_use]
extern crate diesel;
extern crate actix_web;
extern crate dotenv;

// use diesel::prelude::*;
// use diesel::pg::PgConnection;
// use dotenv::dotenv;
// use std::env;

// Need to fix schema enum types and then we can enable it
// pub mod schema;
pub mod models;

use actix_web::{http, server, App, HttpRequest, HttpResponse};

mod upload {
    use futures::future;
    use futures::{Future, Stream};

    use actix_web::{
        dev, error, multipart, Error, FutureResponse, HttpMessage, HttpRequest,
        HttpResponse,
    };

    use std::fs;
    use std::io::Write;

    /// from payload, save file
    pub fn save_file(
        field: multipart::Field<dev::Payload>,
    ) -> Box<Future<Item = i64, Error = Error>> {
        use std::ffi::OsStr;
        use std::path::Path;

        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename().map(|st| st.to_string()))
            .unwrap_or("upload".to_string());
        let fileext = Path::new(&filename)
            .extension()
            .and_then(OsStr::to_str)
            .map_or("".to_string(), |ext| format!(".{}", ext));

        println!(
            "Saving file: filename {:?}; extension: {:?}",
            filename, fileext
        );
        let file_path_string = format!("upload{}", fileext);
        let mut file = match fs::File::create(file_path_string) {
            Ok(file) => file,
            Err(e) => return Box::new(future::err(error::ErrorInternalServerError(e))),
        };
        Box::new(
            field
                .fold(0i64, move |acc, bytes| {
                    let rt = file
                        .write_all(bytes.as_ref())
                        .map(|_| acc + bytes.len() as i64)
                        .map_err(|e| {
                            println!("file.write_all failed: {:?}", e);
                            error::MultipartError::Payload(error::PayloadError::Io(e))
                        });
                    future::result(rt)
                })
                .map_err(|e| {
                    println!("save_file failed, {:?}", e);
                    error::ErrorInternalServerError(e)
                }),
        )
    }

    pub fn handle_multipart_item(
        item: multipart::MultipartItem<dev::Payload>,
    ) -> Box<Stream<Item = i64, Error = Error>> {
        match item {
            multipart::MultipartItem::Field(field) => Box::new(save_file(field).into_stream()),
            multipart::MultipartItem::Nested(mp) => Box::new(
                mp.map_err(error::ErrorInternalServerError)
                    .map(handle_multipart_item)
                    .flatten(),
            ),
        }
    }

    pub fn upload(req: HttpRequest) -> FutureResponse<HttpResponse> {
        Box::new(
            req.multipart()
                .map_err(error::ErrorInternalServerError)
                .map(handle_multipart_item)
                .flatten()
                .collect()
                .map(|sizes| HttpResponse::Ok().json(sizes))
                .map_err(|e| {
                    println!("failed: {}", e);
                    e
                }),
        )
    }
}

fn index(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .header(http::header::CONTENT_TYPE, "text/html")
        .body(r#"<html>
<head>
    <title>Collect</title>
    <link rel="stylesheet" href="/static/app.css"/>
</head>
<body class="page">
    <h1>Collect</h1>
    <a href="/static/app.html">Upload</a>
</body>
</html>"#)
}

fn index2(_req: &HttpRequest) -> &'static str {
    println!("Hello world2---");
    "Hello world 2!"
}

fn main() {
    server::new(|| {
        vec![
            App::new().prefix("/static").handler(
                "/",
                actix_web::fs::StaticFiles::new("./static")
                    .unwrap()
                    .show_files_listing(),
            ),
            App::new()
                .resource("/app", |r| r.f(index2))
                .resource("/upload", |r| r.method(http::Method::POST).with(upload::upload))
                .resource("/", |r| r.f(index)),
        ]
    })
    .bind("127.0.0.1:8088")
    .unwrap()
    .run();
}
