// BASED on https://github.com/actix/examples/tree/master/http-proxy
use actix_web::client::Client;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use env_logger::Env;
use url::Url;

async fn forward(
    req: HttpRequest,
    body: web::Bytes,
    url: web::Data<Url>,
    client: web::Data<Client>
) -> Result<HttpResponse, Error> {
    let mut new_url = url.get_ref().clone();
    new_url.set_path(req.uri().path());
    new_url.set_query(req.uri().query());

    let mut forwarded_req = client
        .request_from(new_url.as_str(), req.head())
        .no_decompress();

    forwarded_req.headers_mut().remove("Host");
    
    let mut res = forwarded_req.send_body(body).await.map_err(Error::from)?;

    let mut client_resp = HttpResponse::build(res.status());

    // Remove `Connection` as per
    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Connection#Directives
    for (header_name, header_value) in
        res.headers().iter().filter(|(h, _)| (*h != "Connection" || *h != "Host"))
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

    let backend_url = Url::parse("https://inct-zovin-development.ngrok.io")
                    .unwrap();

    HttpServer::new(move || {
        App::new()
            .data(Client::new())
            .data(backend_url.clone())
            .wrap(middleware::Logger::default())
            .default_service(web::route().to(forward))
    })
    .bind((listen_addr, listen_port))?
    .run()
    .await
}