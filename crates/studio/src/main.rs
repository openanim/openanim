use std::path::PathBuf;

use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut port = 8080;
    let mut dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--port" => {
                let Some(value) = args.next() else {
                    anyhow::bail!("--port requires a value");
                };
                port = value.parse().context("Failed to parse --port")?;
            }
            "--dir" => {
                let Some(value) = args.next() else {
                    anyhow::bail!("--dir requires a value");
                };
                dir = PathBuf::from(value);
            }
            "--help" | "-h" => {
                println!("Usage: cargo run -p studio -- [--port 8080] [--dir .]");
                return Ok(());
            }
            other => anyhow::bail!("Unknown argument: {}", other),
        }
    }

    let workspace_dir = std::fs::canonicalize(&dir)
        .with_context(|| format!("Failed to canonicalize {}", dir.display()))?;
    studio::start_server(port, workspace_dir).await
}
