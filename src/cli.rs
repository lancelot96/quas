use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

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
    KeyTraffic {
        #[arg(short = 'i', long = "in")]
        file: String,
    },
    MouseTraffic {
        #[arg(short = 'i', long = "in")]
        file: String,
    },
    ImageSteg {
        #[arg(short = 'i', long = "in")]
        file: String,

        #[arg(short, long, default_value_t = 0)]
        red: u8,

        #[arg(short, long, default_value_t = 0)]
        green: u8,

        #[arg(short, long, default_value_t = 0)]
        blue: u8,

        #[arg(short, long, default_value_t = 0)]
        alpha: u8,

        #[arg(short, long, value_enum)]
        format: Format,
    },
    ImageUtil {
        #[arg(short = 'i', long = "in")]
        file: String,

        #[arg(long)]
        brighten: Option<i32>,

        #[arg(long)]
        contrast: Option<f32>,

        #[arg(long)]
        fliph: bool,

        #[arg(long)]
        flipv: bool,

        #[arg(long)]
        grayscale: bool,

        #[arg(long)]
        huerotate: Option<i32>,

        #[arg(long)]
        invert: bool,
    },
}

#[derive(Clone, Debug, ValueEnum)]
pub enum Format {
    Aspect,
    Bin,
    Utf8,
    RGBA,
}
