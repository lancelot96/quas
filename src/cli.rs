use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Subcommand, Debug)]
#[command(rename_all = "lower")]
pub enum CliCommand {
    PngCrc {
        #[arg(short = 'i', long = "in")]
        file: String,
    },
    ZipCrc {
        #[arg(short = 'i', long = "in")]
        file: String,

        #[arg(short, long)]
        size: u64,

        #[arg(
            short,
            long,
            default_value = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~ \t\n\r\x0b\x0c"
        )]
        alphabet: String,
    },
    Base64Steg {
        #[arg(short = 'i', long = "in")]
        file: String,
    },
    Behinder {
        #[arg(short = 'i', long = "in")]
        file: String,

        #[arg(short, long = "out", default_value = "behinder/")]
        outdir: PathBuf,

        #[arg(short, long)]
        key: Option<String>,
    },
    KeyboardSteg {
        #[arg(short = 'i', long = "in")]
        file: String,
    },
    MouseSteg {
        #[arg(short = 'i', long = "in")]
        file: String,
    },
}
