use std::path::PathBuf;

use anyhow::Result;
use async_trait::async_trait;
use image::{io::Reader as ImageReader, GenericImageView};

use crate::Command;

#[derive(Debug)]
pub struct ImageUtil {
    file: String,
    brighten: Option<i32>,
    contrast: Option<f32>,
    fliph: bool,
    flipv: bool,
    grayscale: bool,
    huerotate: Option<i32>,
    invert: bool,
}

impl ImageUtil {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        file: String,
        brighten: Option<i32>,
        contrast: Option<f32>,
        fliph: bool,
        flipv: bool,
        grayscale: bool,
        huerotate: Option<i32>,
        invert: bool,
    ) -> Self {
        Self {
            file,
            brighten,
            contrast,
            fliph,
            flipv,
            grayscale,
            huerotate,
            invert,
        }
    }
}

#[async_trait]
impl Command for ImageUtil {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self {
            file,
            brighten,
            contrast,
            fliph,
            flipv,
            grayscale,
            huerotate,
            invert,
        } = *self;
        let mut image = ImageReader::open(&file)?.with_guessed_format()?.decode()?;

        let (width, height) = image.dimensions();
        tracing::info!(file, width, height);

        if let Some(brighten) = brighten {
            image = image.brighten(brighten);
        }
        if let Some(contrast) = contrast {
            image = image.adjust_contrast(contrast);
        }
        if fliph {
            image = image.fliph();
        }
        if flipv {
            image = image.flipv();
        }
        if grayscale {
            image = image.grayscale();
        }
        if let Some(huerotate) = huerotate {
            image = image.huerotate(huerotate);
        }
        if invert {
            image.invert();
        }

        let path = PathBuf::from(file)
            .file_stem()
            .and_then(|x| x.to_str())
            .map(|x| format!("{}-modified.png", x))
            .unwrap();
        tracing::info!(path);

        image.save(&path).map_err(Into::into)
    }
}
