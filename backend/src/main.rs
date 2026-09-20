use actix_web::{web::{self, Data}, App, HttpServer};
use clap::Parser;

use crate::ytcheck::{check_youtube, YtChecker};

mod args;
mod ytcheck;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stdout)
        .init();
    let args = crate::args::Args::parse();
    let checker = Data::new(YtChecker::new(args.urls, args.min_h, args.timeout, args.v6, args.strategy));
    log::warn!("Service started listen at {}", args.listen);
    HttpServer::new(move || {
        App::new().route(&args.url_handler, web::get().to(check_youtube))
        .app_data(checker.clone())
    })
    .bind(args.listen)?
    .run()
    .await
}