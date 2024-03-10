#![warn(missing_debug_implementations)]

use anyhow::Result;
use async_trait::async_trait;
use clap::Parser;
use tracing::level_filters::LevelFilter;

use crate::{
    base64_steg::Base64Steg,
    behinder::BehinderTrafficAnalyse,
    cli::{Cli, CliCommand},
    keyboard_steg::KeyboardTrafficSteg,
    mouse_steg::MouseTrafficSteg,
    png_crc::PngCrc,
    zip_crc::ZipCrc,
};

mod base64_steg;
mod behinder;
mod cli;
mod error;
mod keyboard_steg;
mod mouse_steg;
mod png_crc;
mod zip_crc;

#[async_trait]
pub trait Command: std::fmt::Debug {
    async fn execute(self: Box<Self>) -> Result<()>;
}

impl From<CliCommand> for Box<dyn Command> {
    fn from(cli_command: CliCommand) -> Self {
        match cli_command {
            CliCommand::PngCrc { file } => Box::new(PngCrc::new(file)),
            CliCommand::ZipCrc {
                file,
                size,
                alphabet,
            } => Box::new(ZipCrc::new(file, size, alphabet)),
            CliCommand::Base64Steg { file } => Box::new(Base64Steg::new(file)),
            CliCommand::Behinder { file, outdir, key } => {
                Box::new(BehinderTrafficAnalyse::new(file, outdir, key))
            }
            CliCommand::KeyboardSteg { file } => Box::new(KeyboardTrafficSteg::new(file)),
            CliCommand::MouseSteg { file } => Box::new(MouseTrafficSteg::new(file)),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let Cli {
        verbose,
        command: cli_command,
    } = Cli::parse();
    initialize(verbose);
    tracing::debug!(?cli_command);

    Into::<Box<dyn Command>>::into(cli_command).execute().await
}

fn initialize(verbose: u8) {
    let level_filter = match verbose {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };
    tracing_subscriber::fmt()
        .with_max_level(level_filter)
        .init();
}
