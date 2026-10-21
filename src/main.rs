use std::process;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use clap::Parser;

use showme::{Cli, Renderer, Result};

fn main() {
    if let Err(err) = run() {
        if matches!(&err, showme::RimgError::Io(error) if error.kind() == std::io::ErrorKind::Interrupted)
        {
            process::exit(130);
        }
        eprintln!("error: {err}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let interrupt_flag = Arc::clone(&cancelled);
    ctrlc::set_handler(move || interrupt_flag.store(true, Ordering::Relaxed)).map_err(|error| {
        showme::RimgError::other(format!("failed to install interrupt handler: {error}"))
    })?;
    let cli = Cli::parse();
    let config = cli.into_config()?;
    let renderer = Renderer::build(config)?.with_cancellation(cancelled);
    renderer.run()
}
