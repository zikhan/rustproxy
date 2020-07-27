#[macro_use]
extern crate log;
extern crate env_logger;

use actix_web::client::Client;
use actix_web::error::{ErrorBadRequest, ErrorInternalServerError};
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use env_logger::Env;
use rusoto_core::Region;
use rusoto_dynamodb::{AttributeValue, DynamoDb, DynamoDbClient, GetItemInput};
use std::collections::HashMap;
use std::str;
use url::{ParseError, Url};

fn path_segments(path: &str) -> Option<str::Split<char>> {
    if path.starts_with('/') {
        Some(path[1..].split('/'))
    } else {
        None
    }
}

async fn gethost(tenant: &str) -> Result<Url, ParseError> {
    let client = DynamoDbClient::new(Region::Custom {
        name: "us-west-2".to_owned(),
        endpoint: "http://dynamodb:8000".to_owned(),
    });
    let tenant_owned = tenant.to_owned();
    let attr_value = AttributeValue {
        s: Some(tenant_owned),
        ..Default::default()
    };
    let key_map: HashMap<String, AttributeValue> =
        [("PKey".to_string(), attr_value)].iter().cloned().collect();
    let get_input: GetItemInput = GetItemInput {
        table_name: "Backends".to_string(),
        key: key_map,
        ..Default::default()
    };

    let itemmap: Option<HashMap<String, AttributeValue>> = match client.get_item(get_input).await {
        Ok(output) => output.item,
        Err(_err) => {
            println!("{:?}", _err);
            None
        }
    };

    let attr: Option<AttributeValue> = match itemmap {
        Some(value) => value.get("location").cloned(),
        None => None,
    };

    let location: String = match attr {
        Some(att) => {
            if let Some(st) = att.s {
                st.to_string()
            } else {
                "".to_string()
            }
        }
        None => "".to_string(),
    };
    Url::parse(&location)
}

// BASED on https://github.com/actix/examples/tree/master/http-proxy
async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    let mut segs =
        path_segments(req.path()).ok_or_else(|| ErrorInternalServerError("Path error"))?;
    let tenant: &str = segs
        .next()
        .ok_or_else(|| ErrorBadRequest("Missing Tenant"))?;

    if tenant.is_empty() {
        return Err(ErrorBadRequest("Missing Tenant"));
    }

    let forward_path: Vec<&str> = segs.collect();
    let forward_path = forward_path.join("/");

    let url = gethost(tenant)
        .await
        .map_err(|e| ErrorInternalServerError(e))?;

    let mut new_url = url.clone();
    new_url.set_path(&forward_path);
    new_url.set_query(req.uri().query());

    let mut forwarded_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress();

    forwarded_req.headers_mut().remove("Host");

    info!("Sending Request to {0}", new_url);

    let mut res = forwarded_req.send_body(body).await.map_err(Error::from)?;

    let mut client_resp = HttpResponse::build(res.status());

    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in res
        .headers()
        .iter()
        .filter(|(h, _)| (*h != "Connection" || *h != "Host"))
    {
        client_resp.header(header_name.clone(), header_value.clone());
    }

    Ok(client_resp.body(res.body().await?))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    let listen_addr = "0.0.0.0";
    let listen_port = 8080u16;

    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
    .bind((listen_addr, listen_port))?
    .run()
    .await
}
