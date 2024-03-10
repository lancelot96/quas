use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::Result;
use async_trait::async_trait;
use crc32fast::Hasher;
use tokio::{fs, spawn};
use tracing::instrument;

use crate::Command;

#[derive(Debug)]
pub struct PngCrc {
    file: String,
}

#[derive(Copy, PartialEq, Eq, Clone, Debug)]
enum WoH {
    Width,
    Height,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, PartialEq, Eq, Debug)]
struct IHDR {
    header: [u8; 4],
    width: [u8; 4],
    height: [u8; 4],
    others: [u8; 5],
}

impl IHDR {
    fn from(data: &[u8]) -> (Self, u32) {
        let ihdr = Self {
            header: data[12..16].try_into().unwrap(),
            width: data[16..20].try_into().unwrap(),
            height: data[20..24].try_into().unwrap(),
            others: data[24..29].try_into().unwrap(),
        };
        let crc = Self::parse_crc(data);

        (ihdr, crc)
    }

    fn parse_crc(data: &[u8]) -> u32 {
        let bytes = &data[29..33];
        u32::from_be_bytes(bytes.try_into().unwrap())
    }

    #[instrument(skip(self, finished))]
    async fn brute(&self, woh: WoH, expected: u32, finished: Arc<AtomicBool>) -> (WoH, u32) {
        for i in 0_u32.. {
            let bytes = i.to_be_bytes();
            let computed = self.crc(Some((woh, bytes)));
            if finished.load(Ordering::SeqCst) || computed == expected {
                tracing::trace!(?woh, i, computed);

                finished.store(true, Ordering::SeqCst);
                return (woh, i);
            }
        }

        unreachable!()
    }

    fn crc(&self, woh: Option<(WoH, [u8; 4])>) -> u32 {
        let mut hasher = Hasher::new();
        hasher.update(&self.header);
        match woh {
            Some((WoH::Width, width)) => {
                hasher.update(&width);
                hasher.update(&self.height);
            }
            Some((WoH::Height, height)) => {
                hasher.update(&self.width);
                hasher.update(&height);
            }
            None => {
                hasher.update(&self.width);
                hasher.update(&self.height);
            }
        }
        hasher.update(&self.others);
        hasher.finalize()
    }

    fn width(&self) -> u32 {
        u32::from_be_bytes(self.width)
    }

    fn height(&self) -> u32 {
        u32::from_be_bytes(self.height)
    }
}

#[async_trait]
impl Command for PngCrc {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self { file } = *self;

        let mut data = fs::read(&file).await?;
        let (ihdr, expected) = IHDR::from(&data);
        tracing::info!(
            "Read png with width({:#x}), height({:#x}) and CRC({:#x}).",
            ihdr.width(),
            ihdr.height(),
            expected,
        );

        let computed = ihdr.crc(None);
        tracing::info!("Computed CRC is {:#x}.", computed);
        if computed == expected {
            return Ok(());
        }

        let ihdr = Arc::new(ihdr);
        let finished = Arc::new(AtomicBool::new(false));
        let (woh, v) = tokio::select! {
            width = {
                let ihdr = ihdr.clone();
                let finished = finished.clone();
                spawn(async move {ihdr.brute(WoH::Width, expected, finished).await})
            } => width,
            height = {
                spawn(async move { ihdr.brute(WoH::Height, expected, finished).await})
            } => height,
        }?;
        tracing::info!("Found correct {:?}({:#x}).", woh, v);

        let bytes = v.to_be_bytes();
        match woh {
            WoH::Width => data[16..20].copy_from_slice(&bytes),
            WoH::Height => data[20..24].copy_from_slice(&bytes),
        }

        let png_path = PathBuf::from(file)
            .file_stem()
            .and_then(|x| x.to_str())
            .map(|x| format!("{}-fixed.png", x))
            .unwrap();
        fs::write(&png_path, data).await?;
        tracing::info!("Fixed png saved as ({:?}).", png_path);

        Ok(())
    }
}

impl PngCrc {
    pub fn new(file: String) -> Self {
        Self { file }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use super::{WoH, IHDR};

    #[test]
    fn test_ihdr_from() {
        let data = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x01, 0x35, 0x00, 0x00, 0x04, 0x24, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x93, 0xcf, 0x1e, 0xca,
        ];
        let (ihdr, crc) = IHDR::from(&data);
        assert_eq!(
            ihdr,
            IHDR {
                header: *b"IHDR",
                width: [0x00, 0x00, 0x01, 0x35],
                height: [0x00, 0x00, 0x04, 0x24],
                others: [0x08, 0x02, 0x00, 0x00, 0x00],
            }
        );
        assert_eq!(crc, 0x93cf1eca);
    }

    #[test]
    fn test_ihdr_crc() {
        let data = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x01, 0x35, 0x00, 0x00, 0x04, 0x24, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x93, 0xcf, 0x1e, 0xca,
        ];
        let (ihdr, crc) = IHDR::from(&data);
        assert_eq!(ihdr.crc(None), crc);
    }

    #[tokio::test]
    async fn test_crc_brute_height() {
        let data = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x01, 0x35, 0x00, 0x00, 0x00, 0xe8, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x93, 0xcf, 0x1e, 0xca,
        ];
        let (ihdr, expected) = IHDR::from(&data);
        let computed = ihdr.crc(None);
        assert_ne!(computed, expected);

        let woh = WoH::Height;
        let finished = Arc::new(AtomicBool::new(false));
        let (_woh, height) = ihdr.brute(woh, expected, finished.clone()).await;
        let crc = ihdr.crc(Some((woh, height.to_be_bytes())));
        assert_eq!(woh, _woh);
        assert_eq!(crc, expected);
        assert!(finished.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_crc_brute_width() {
        let data = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x35, 0x00, 0x00, 0x04, 0x24, 0x08, 0x02, 0x00, 0x00,
            0x00, 0x93, 0xcf, 0x1e, 0xca,
        ];
        let (ihdr, expected) = IHDR::from(&data);
        let computed = ihdr.crc(None);
        assert_ne!(computed, expected);

        let woh = WoH::Width;
        let finished = Arc::new(AtomicBool::new(false));
        let (_woh, height) = ihdr.brute(woh, expected, finished.clone()).await;
        let crc = ihdr.crc(Some((woh, height.to_be_bytes())));
        assert_eq!(woh, _woh);
        assert_eq!(crc, expected);
        assert!(finished.load(Ordering::SeqCst));
    }
}
