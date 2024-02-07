use std::{
    net::Ipv4Addr,
    panic::{set_hook, take_hook},
    process::exit,
};

use anyhow::{anyhow, Error};
use axum::{serve, Router};
use log::{error, info};
use tokio::net::TcpListener;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use zkscan_api::{api, config::CONFIG, state::STATE};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    let default_panic = take_hook();
    set_hook(Box::new(move |info| {
        error!("Panic: {}", info);
        default_panic(info);
        exit(1);
    }));

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .from_env()?
        .add_directive("tokio_postgres=info".parse()?)
        .add_directive("rustls=info".parse()?)
        .add_directive("h2=info".parse()?)
        .add_directive("hyper=info".parse()?)
        .add_directive("reqwest=info".parse()?)
        .add_directive("tungstenite=info".parse()?);

    info!("Setting up tracing with filter: {}", filter);
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .compact()
        .init();

    let app = Router::new()
        .nest("/api/v1/", api::routes())
        .with_state(STATE.clone());
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, CONFIG.port)).await?;
    info!("Server is listening on http://0.0.0.0:{}", CONFIG.port,);
    serve(listener, app)
        .await
        .map_err(|e| anyhow!("Server error: {}", e))?;

    Ok(())
}
