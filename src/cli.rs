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

        #[arg(short, long, default_value_t = 1)]
        red: u8,

        #[arg(short, long, default_value_t = 1)]
        green: u8,

        #[arg(short, long, default_value_t = 1)]
        blue: u8,

        #[arg(short, long, default_value_t = 0)]
        alpha: u8,

        #[arg(short)]
        y_then_x: bool,

        #[arg(short = 'X')]
        x_reverse: bool,

        #[arg(short = 'Y')]
        y_reverse: bool,

        #[arg(short, long, default_value = "rgb")]
        order: ImageStegOrder,

        #[arg(short, long, default_value = "aspect")]
        format: ImageStegFormat,
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

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug, ValueEnum)]
pub enum ImageStegOrder {
    RGB,
    RBG,
    GRB,
    GBR,
    BRG,
    BGR,
}

impl From<ImageStegOrder> for [u8; 4] {
    fn from(value: ImageStegOrder) -> Self {
        match value {
            ImageStegOrder::RGB => [0, 1, 2, 3],
            ImageStegOrder::RBG => [0, 2, 1, 3],
            ImageStegOrder::GRB => [1, 0, 2, 3],
            ImageStegOrder::GBR => [1, 2, 0, 3],
            ImageStegOrder::BRG => [2, 0, 1, 3],
            ImageStegOrder::BGR => [2, 1, 0, 3],
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ImageStegFormat {
    Aspect,
    Bin,
    Utf8,
    #[allow(clippy::upper_case_acronyms)]
    RGBA,
}
