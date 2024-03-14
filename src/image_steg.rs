use std::{fs::File, path::PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use image::{io::Reader as ImageReader, Rgba, RgbaImage};
use png::{BitDepth, ColorType, Encoder as PngEncoder};
use tokio::fs::{create_dir_all, write};
use tracing::instrument;

use crate::{cli::ImageStegFormat, Command};

#[derive(Debug)]
pub struct ImageSteg {
    file: String,
    mask: [u8; 4],
    y_then_x: bool,
    x_reverse: bool,
    y_reverse: bool,
    order: [u8; 4],
    format: ImageStegFormat,
}

impl ImageSteg {
    pub fn new(
        file: String,
        mask: [u8; 4],
        y_then_x: bool,
        x_reverse: bool,
        y_reverse: bool,
        order: [u8; 4],
        format: ImageStegFormat,
    ) -> Self {
        Self {
            file,
            mask,
            y_then_x,
            x_reverse,
            y_reverse,
            order,
            format,
        }
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

    fn extract_bits_x_first<I1, I2, F, O>(x_iter: I1, y_iter: I2, f: F) -> Vec<u8>
    where
        I1: DoubleEndedIterator<Item = u32> + Clone,
        I2: DoubleEndedIterator<Item = u32> + Clone,
        O: Iterator<Item = u8>,
        F: Fn((u32, u32)) -> O,
    {
        y_iter
            .flat_map(|y| x_iter.clone().map(move |x| (x, y)).flat_map(&f))
            .collect()
    }

    fn extract_bits_y_first<I1, I2, F, O>(x_iter: I1, y_iter: I2, f: F) -> Vec<u8>
    where
        I1: DoubleEndedIterator<Item = u32> + Clone,
        I2: DoubleEndedIterator<Item = u32> + Clone,
        O: Iterator<Item = u8>,
        F: Fn((u32, u32)) -> O,
    {
        x_iter
            .flat_map(|x| y_iter.clone().map(move |y| (x, y)).flat_map(&f))
            .collect()
    }

    fn extract_bits(
        image: &RgbaImage,
        mask: [u8; 4],
        y_then_x: bool,
        x_reverse: bool,
        y_reverse: bool,
        order: [u8; 4],
    ) -> Vec<u8> {
        let f = |(x, y)| {
            let Rgba(rgba) = image.get_pixel(x, y);
            order
                .into_iter()
                .map(move |x| (rgba[x as usize], mask[x as usize]))
                .filter(|&(_, m)| m != 0)
                .flat_map(|(x, m)| {
                    (0..u8::BITS)
                        .rev()
                        .filter(move |i| m >> i & 1 == 1)
                        .map(move |i| x >> i & 1)
                })
        };

        let (width, height) = image.dimensions();
        match (y_then_x, x_reverse, y_reverse) {
            (true, true, true) => {
                Self::extract_bits_y_first((0..width).rev(), (0..height).rev(), f)
            }
            (true, true, false) => Self::extract_bits_y_first((0..width).rev(), 0..height, f),
            (true, false, true) => Self::extract_bits_y_first(0..width, (0..height).rev(), f),
            (true, false, false) => Self::extract_bits_y_first(0..width, 0..height, f),
            (false, true, true) => {
                Self::extract_bits_x_first((0..width).rev(), (0..height).rev(), f)
            }
            (false, true, false) => Self::extract_bits_x_first((0..width).rev(), 0..height, f),
            (false, false, true) => Self::extract_bits_x_first(0..width, (0..height).rev(), f),
            (false, false, false) => Self::extract_bits_x_first(0..width, 0..height, f),
        }
    }

    fn bits2bytes(bits: &[u8]) -> Vec<u8> {
        bits.chunks(u8::BITS as usize)
            .map(|x| x.iter().fold(0, |byte, x| byte << 1 | x))
            .collect()
    }

    #[instrument]
    fn aspect_masks(mask: [u8; 4]) -> Vec<[u8; 4]> {
        let mask = u32::from_be_bytes(mask);
        (0..u32::BITS)
            .map(|i| mask & 1 << i)
            .filter(|&x| x != 0)
            .map(u32::to_be_bytes)
            .inspect(|x| tracing::debug!(?x))
            .collect()
    }
}

#[async_trait]
impl Command for ImageSteg {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self {
            file,
            mask,
            y_then_x,
            x_reverse,
            y_reverse,
            order,
            format,
        } = *self;

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
            ImageStegFormat::Bin => {
                let bits = Self::extract_bits(&image, mask, y_then_x, x_reverse, y_reverse, order);
                let bytes = Self::bits2bytes(&bits);

                let file_path = outdir.join(format!("{:?}.bin", mask));
                write(&file_path, bytes).await?;

                tracing::info!(?file_path);
            }
            ImageStegFormat::Utf8 => {
                let bits = Self::extract_bits(&image, mask, y_then_x, x_reverse, y_reverse, order);
                let bytes = Self::bits2bytes(&bits);
                let utf8 = String::from_utf8_lossy(&bytes);

                let file_path = outdir.join(format!("{:?}.txt", mask));
                write(file_path, utf8.into_owned()).await?;
            }
            ImageStegFormat::RGBA => {
                if mask[3] == 0 {
                    tracing::warn!("Output format rgba with alpha(0), maybe you want alpha 255.");
                }

                Self::filter_bits(&mut image, mask);

                let file_path = outdir.join(format!("{:?}.rgba.png", mask));
                image.save(&file_path)?;

                tracing::info!(?file_path);
            }
            ImageStegFormat::Aspect => {
                let masks = Self::aspect_masks(mask);
                for mask in masks {
                    let bits =
                        Self::extract_bits(&image, mask, y_then_x, x_reverse, y_reverse, order);
                    let bytes = Self::bits2bytes(&bits);
                    tracing::debug!(
                        ?mask,
                        num_bits = bits.len(),
                        num_bytes = bytes.len(),
                        dimensions = width * height,
                    );

                    let file_path = outdir.join(format!("{:?}.aspect.png", mask));
                    let writer = File::create(&file_path)?;
                    let mut encoder = PngEncoder::new(writer, width, height);
                    encoder.set_depth(BitDepth::One);
                    encoder.set_color(ColorType::Grayscale);

                    encoder
                        .write_header()
                        .and_then(|mut x| x.write_image_data(&bytes))?;

                    tracing::info!(?file_path);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use anyhow::Result;
    use image::io::Reader as ImageReader;
    use once_cell::sync::Lazy;

    use super::ImageSteg;

    static IMAGE_DATA: Lazy<Vec<u8>> = Lazy::new(|| {
        vec![
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x02, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x9d, 0x74, 0x66, 0x1a, 0x00, 0x00, 0x00, 0x06, 0x62, 0x4b, 0x47, 0x44, 0x00,
            0xff, 0x00, 0xff, 0x00, 0xff, 0xa0, 0xbd, 0xa7, 0x93, 0x00, 0x00, 0x00, 0x09, 0x70,
            0x48, 0x59, 0x73, 0x00, 0x00, 0x2e, 0x23, 0x00, 0x00, 0x2e, 0x23, 0x01, 0x78, 0xa5,
            0x3f, 0x76, 0x00, 0x00, 0x00, 0x25, 0x49, 0x44, 0x41, 0x54, 0x08, 0x1d, 0x01, 0x1a,
            0x00, 0xe5, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00,
            0xff, 0xff, 0x02, 0x00, 0xff, 0xff, 0x00, 0x80, 0x81, 0x80, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x91, 0x36, 0x09, 0x7d, 0x7b, 0x7e, 0xfa, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x49,
            0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ]
    });

    #[test]
    fn test_filter_bits_with_zero_mask() -> Result<()> {
        let mask = [0, 0, 0, 0];
        let mut image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        ImageSteg::filter_bits(&mut image, mask);
        assert!(image.into_vec().into_iter().all(|x| x == 0));

        Ok(())
    }

    #[test]
    fn test_filter_bits_with_red_mask() -> Result<()> {
        let mask = [0xff, 0, 0, 0];
        let mut image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        ImageSteg::filter_bits(&mut image, mask);
        assert_eq!(
            image.into_vec(),
            vec![255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0],
        );

        Ok(())
    }

    #[test]
    fn test_filter_bits_with_green_mask() -> Result<()> {
        let mask = [0, 0xff, 0, 0];
        let mut image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        ImageSteg::filter_bits(&mut image, mask);
        assert_eq!(
            image.into_vec(),
            vec![0, 0, 0, 0, 0, 255, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0],
        );

        Ok(())
    }

    #[test]
    fn test_filter_bits_with_blue_mask() -> Result<()> {
        let mask = [0, 0, 0xff, 0];
        let mut image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        ImageSteg::filter_bits(&mut image, mask);
        assert_eq!(
            image.into_vec(),
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 128, 0, 0, 0, 0, 0],
        );

        Ok(())
    }

    #[test]
    fn test_filter_bits_with_alpha_mask() -> Result<()> {
        let mask = [0, 0, 0, 0xff];
        let mut image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        ImageSteg::filter_bits(&mut image, mask);
        assert_eq!(
            image.into_vec(),
            vec![
                0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 255,
            ],
        );

        Ok(())
    }

    #[test]
    fn test_filter_bits_with_mixed_mask() -> Result<()> {
        let mask = [0x0f, 0x0f, 0x0f, 0x0f];
        let mut image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        ImageSteg::filter_bits(&mut image, mask);
        assert_eq!(
            image.into_vec(),
            vec![
                15, 0, 0, 15, 0, 15, 0, 15, 0, 0, 15, 15, 15, 15, 15, 15, 0, 0, 0, 15, 0, 0, 0, 15,
            ],
        );

        Ok(())
    }

    #[test]
    fn test_extract_bits() -> Result<()> {
        let mask = [0x0f, 0x0f, 0x0f, 0x0f];
        let (y_then_x, x_reverse, y_reverse) = (false, false, false);
        let order = [0, 1, 2, 3];
        let image = ImageReader::new(Cursor::new(IMAGE_DATA.clone()))
            .with_guessed_format()?
            .decode()?
            .into_rgba8();

        let bits = ImageSteg::extract_bits(&image, mask, y_then_x, x_reverse, y_reverse, order);
        let bytes = ImageSteg::bits2bytes(&bits);
        assert_eq!(bytes, vec![240, 15, 15, 15, 0, 255, 255, 255, 0, 15, 0, 15]);

        Ok(())
    }

    #[test]
    fn test_bits2bytes() {
        let bits = vec![0, 0, 1, 0, 0, 0, 0, 1];
        let bytes = ImageSteg::bits2bytes(&bits);
        assert_eq!(bytes, vec![0b00100001]);
    }

    #[test]
    fn test_aspect_masks() {
        let mask = [0xff, 0xff, 0xff, 0xff];
        let masks = ImageSteg::aspect_masks(mask);
        assert_eq!(
            masks,
            vec![
                [0, 0, 0, 1],
                [0, 0, 0, 2],
                [0, 0, 0, 4],
                [0, 0, 0, 8],
                [0, 0, 0, 16],
                [0, 0, 0, 32],
                [0, 0, 0, 64],
                [0, 0, 0, 128],
                [0, 0, 1, 0],
                [0, 0, 2, 0],
                [0, 0, 4, 0],
                [0, 0, 8, 0],
                [0, 0, 16, 0],
                [0, 0, 32, 0],
                [0, 0, 64, 0],
                [0, 0, 128, 0],
                [0, 1, 0, 0],
                [0, 2, 0, 0],
                [0, 4, 0, 0],
                [0, 8, 0, 0],
                [0, 16, 0, 0],
                [0, 32, 0, 0],
                [0, 64, 0, 0],
                [0, 128, 0, 0],
                [1, 0, 0, 0],
                [2, 0, 0, 0],
                [4, 0, 0, 0],
                [8, 0, 0, 0],
                [16, 0, 0, 0],
                [32, 0, 0, 0],
                [64, 0, 0, 0],
                [128, 0, 0, 0],
            ],
        );
    }
}
