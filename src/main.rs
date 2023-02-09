
use actix_web::{http, middleware, web, App, HttpResponse, HttpServer, Error};
use reqwest::Client;

use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Mutex;
use lazy_static::lazy_static;
use tokio::{time, sync::RwLock};


lazy_static! {
    static ref CACHE: Mutex<HashMap<String, (String, std::time::Instant)>> = Mutex::new(HashMap::new());
}

async fn clear_cache(duration: Duration) {
    loop {
        time::sleep(duration).await;
        println!("Clearing cache");
        let mut cache = CACHE.lock().unwrap();
        let now = std::time::Instant::now();
        cache.retain(|_, (_, time)| now.duration_since(*time) < duration);
    }
}

async fn proxy(
    url: web::Path<String>,
    client: web::Data<Client>,
) -> Result<HttpResponse, Error> {
    let mut cache = CACHE.lock().unwrap();

    let formatted_url = format!("https://{}", url.as_str());

    if let Some((data, timestamp)) = cache.get(&formatted_url) {
    
        return Ok(HttpResponse::Ok()
            .header(http::header::CONTENT_TYPE, "application/json")
            .body(data.to_string()));
        
    }

    let resp = match client.get(formatted_url.clone()).send().await {
        Ok(resp) => resp,
        Err(e) => {
            println!("An error occurred while sending the request: {}", e);
            return Err(actix_web::error::ErrorInternalServerError(e));
        },
    };

    let status = resp.status();
    let headers = resp.headers().clone();

    let body = match resp.text().await {
        Ok(body) => body,
        Err(e) => {
            println!("An error occurred while reading the response body: {}", e);
            return Err(actix_web::error::ErrorInternalServerError(e));
        },
    };

    println!("{:?}", body.clone());
    cache.insert(formatted_url, (body.clone(), Instant::now()));

    Ok(HttpResponse::build(status)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(body))
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client = Client::new();

    let clear_cache_duration = Duration::from_secs(30);

    tokio::spawn(clear_cache(clear_cache_duration));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(client.clone())
            .route("/{url:.*}", web::get().to(proxy))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}




