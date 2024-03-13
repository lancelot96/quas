use std::{fs::File, path::PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use image::{io::Reader as ImageReader, Rgba, RgbaImage};
use png::{BitDepth, ColorType, Encoder as PngEncoder};
use tokio::fs::{create_dir_all, write};

use crate::{cli::Format, Command};

#[derive(Debug)]
pub struct ImageSteg {
    file: String,
    mask: [u8; 4],
    format: Format,
}

impl ImageSteg {
    pub fn new(file: String, mask: [u8; 4], format: Format) -> Self {
        Self { file, mask, format }
    }

    fn filter_bits(image: &mut RgbaImage, mask: [u8; 4]) {
        image.pixels_mut().for_each(|rgba| {
            rgba.0
                .iter_mut()
                .zip(mask)
                .filter(|&(_, m)| m != u8::MAX)
                .for_each(|(x, m)| *x &= m)
        });
    }

    // TODO: iterator order
    fn extract_bits(image: &RgbaImage, mask: [u8; 4]) -> Vec<u8> {
        image
            .pixels()
            .flat_map(|Rgba(rgba)| {
                rgba.into_iter()
                    .zip(mask)
                    .filter(|&(_, m)| m != 0)
                    .flat_map(|(x, m)| {
                        (0..u8::BITS)
                            .rev()
                            .filter(move |i| m >> i & 1 == 1)
                            .map(move |i| x >> i & 1)
                    })
            })
            .collect()
    }

    fn bits2bytes(bits: &[u8]) -> Vec<u8> {
        bits.chunks(u8::BITS as usize)
            .map(|x| x.iter().fold(0, |byte, x| byte << 1 | x))
            .collect()
    }
}

#[async_trait]
impl Command for ImageSteg {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self { file, mask, format } = *self;

        let outdir = PathBuf::from(&file).file_stem().map(PathBuf::from).unwrap();
        if !outdir.is_dir() {
            create_dir_all(&outdir).await?;
        }

        let mut image = ImageReader::open(&file)?
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        let (width, height) = image.dimensions();
        tracing::info!(file, width, height);

        match format {
            Format::Bin => {
                let bits = Self::extract_bits(&image, mask);
                let bytes = Self::bits2bytes(&bits);

                let file_path = outdir.join(format!("{:?}.bin", mask));
                write(file_path, bytes).await?;
            }
            Format::Utf8 => {
                let bits = Self::extract_bits(&image, mask);
                let bytes = Self::bits2bytes(&bits);
                let utf8 = String::from_utf8_lossy(&bytes);

                let file_path = outdir.join(format!("{:?}.txt", mask));
                write(file_path, utf8.into_owned()).await?;
            }
            Format::RGBA => {
                if mask[3] == 0 {
                    tracing::warn!("Output format rgba with alpha(0), maybe you want alpha 255.");
                }

                Self::filter_bits(&mut image, mask);

                let file_path = outdir.join(format!("{:?}.rgba.png", mask));
                image.save(file_path)?;
            }
            Format::Aspect => {
                let mask_in_u32 = u32::from_be_bytes(mask);
                let masks = (0..u32::BITS)
                    .map(|i| mask_in_u32 & 1 << i)
                    .filter(|&x| x != 0)
                    .map(u32::to_be_bytes)
                    .collect::<Vec<_>>();
                tracing::debug!(?masks);

                for mask in masks {
                    let bits = Self::extract_bits(&image, mask);
                    let bytes = Self::bits2bytes(&bits);
                    tracing::debug!(
                        num_bits = bits.len(),
                        num_bytes = bytes.len(),
                        dimensions = width * height,
                    );

                    let file_path = outdir.join(format!("{:?}.aspect.png", mask));
                    let writer = File::create(file_path)?;
                    let mut encoder = PngEncoder::new(writer, width, height);
                    encoder.set_depth(BitDepth::One);
                    encoder.set_color(ColorType::Grayscale);

                    encoder
                        .write_header()
                        .and_then(|mut x| x.write_image_data(&bytes))?;
                }
            }
        }

        Ok(())
    }
}
