use std::process;

use clap::Parser;

use terminal_media::{Cli, Renderer, Result};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = cli.into_config()?;
    let renderer = Renderer::build(config)?;
    renderer.run()
}
