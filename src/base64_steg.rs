use anyhow::Result;
use async_trait::async_trait;
use tokio::fs;
use tracing::instrument;

use crate::Command;

const BASE64MATRIX: [u8; 128] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x3e, 0xff, 0xff, 0xff, 0x3f,
    0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
    0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0xff, 0xff, 0xff, 0xff, 0xff,
    0xff, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28,
    0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e, 0x2f, 0x30, 0x31, 0x32, 0x33, 0xff, 0xff, 0xff, 0xff, 0xff,
];

#[derive(Debug)]
pub struct Base64Steg {
    file: String,
}

impl Base64Steg {
    pub fn new(file: String) -> Self {
        Self { file }
    }

    #[instrument]
    fn steg_from_base64(base64: &str) -> Option<Vec<u8>> {
        let (i, last) = base64.bytes().rev().enumerate().find(|&(_, x)| x != b'=')?;
        let preimage =
            BASE64MATRIX
                .get(usize::from(last))
                .and_then(|&x| if x != 0xff { Some(x) } else { None })?;
        tracing::trace!(i, ?last, preimage);

        let bits = (0..i)
            .map(|i| i * 2)
            .rev()
            .map(|i| (preimage & 0b11 << i) >> i)
            .collect();
        Some(bits)
    }

    fn bits2string(bits: Vec<u8>) -> String {
        let bytes = bits
            .chunks(4)
            .map(|x| x.iter().fold(0_u8, |x, &b| x << 2 | b))
            .filter(|&x| x != 0)
            .collect::<Vec<u8>>();
        String::from_utf8_lossy(&bytes).to_string()
    }
}

#[async_trait]
impl Command for Base64Steg {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self { file } = *self;

        let data = fs::read_to_string(file).await?;
        let bits = data
            .split_whitespace()
            .flat_map(Self::steg_from_base64)
            .flatten()
            .collect();
        tracing::debug!(?bits);

        let steg = Self::bits2string(bits);
        tracing::info!(steg);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Base64Steg;

    #[test]
    fn test_steg_from_base64_with_empty() {
        let base64 = "";
        let bits = Base64Steg::steg_from_base64(base64);
        assert_eq!(bits, None);
    }

    #[test]
    fn test_steg_from_base64_with_zero_bit() {
        let base64 = "SUNBZ0lHVnNjMlVnYVdZb1lTNXphWHBsS0NrZ1BDQmlMbk5wZW1Vb0tTa0sN";
        let bits = Base64Steg::steg_from_base64(base64);
        assert_eq!(bits, Some(Vec::new()));
    }

    #[test]
    fn test_steg_from_base64_with_one_bit() {
        let base64 = "STJsdVkyeDFaR1U4YVc5emRISmxZVzArQ2c9PQ1=";
        let bits = Base64Steg::steg_from_base64(base64);
        assert_eq!(bits, Some(vec![0b01]));
    }

    #[test]
    fn test_steg_from_base64_with_two_bit() {
        let base64 = "STJsdVkyeDFaR1U4WTNOMGNtbHVaejRLDV==";
        let bits = Base64Steg::steg_from_base64(base64);
        assert_eq!(bits, Some(vec![0b01, 0b01]));
    }

    #[test]
    fn test_bits2string() {
        let bits = vec![
            1, 0, 0, 1, 1, 0, 0, 3, 1, 1, 1, 0, 1, 0, 1, 2, 1, 3, 2, 3, 0, 3, 1, 2, 1, 2, 0, 1, 1,
            3, 0, 3, 1, 2, 1, 1, 1, 2, 0, 2, 0, 3, 1, 0, 1, 1, 3, 3, 1, 2, 1, 2, 0, 3, 0, 3, 0, 3,
            0, 3, 0, 2, 0, 1, 1, 3, 3, 1,
        ];
        let steg = Base64Steg::bits2string(bits);
        assert_eq!(steg, "ACTF{6aseb4_f33!}");
    }
}
