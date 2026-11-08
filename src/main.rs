use std::{env, io};

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("showcase on http://{addr}");
    axum::serve(listener, design_showcase::app(static_dir)).await
}
