use std::{collections::BTreeMap, process::Output};

use anyhow::Result;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use tokio::process::Command as Process;

use crate::{error::Error, Command};

static KEY_MAP: Lazy<BTreeMap<(u8, u8), char>> = Lazy::new(KeyboardTrafficSteg::build_key_map);

#[derive(Debug)]
pub struct KeyboardTrafficSteg {
    file: String,
}

impl KeyboardTrafficSteg {
    pub fn new(file: String) -> Self {
        Self { file }
    }

    fn build_key_map() -> BTreeMap<(u8, u8), char> {
        [
            ((0x00, 0x04), 'a'),
            ((0x00, 0x05), 'b'),
            ((0x00, 0x06), 'c'),
            ((0x00, 0x07), 'd'),
            ((0x00, 0x08), 'e'),
            ((0x00, 0x09), 'f'),
            ((0x00, 0x0a), 'g'),
            ((0x00, 0x0b), 'h'),
            ((0x00, 0x0c), 'i'),
            ((0x00, 0x0d), 'j'),
            ((0x00, 0x0e), 'k'),
            ((0x00, 0x0f), 'l'),
            ((0x00, 0x10), 'm'),
            ((0x00, 0x11), 'n'),
            ((0x00, 0x12), 'o'),
            ((0x00, 0x13), 'p'),
            ((0x00, 0x14), 'q'),
            ((0x00, 0x15), 'r'),
            ((0x00, 0x16), 's'),
            ((0x00, 0x17), 't'),
            ((0x00, 0x18), 'u'),
            ((0x00, 0x19), 'v'),
            ((0x00, 0x1a), 'w'),
            ((0x00, 0x1b), 'x'),
            ((0x00, 0x1c), 'y'),
            ((0x00, 0x1d), 'z'),
            ((0x00, 0x1e), '1'),
            ((0x00, 0x1f), '2'),
            ((0x00, 0x20), '3'),
            ((0x00, 0x21), '4'),
            ((0x00, 0x22), '5'),
            ((0x00, 0x23), '6'),
            ((0x00, 0x24), '7'),
            ((0x00, 0x25), '8'),
            ((0x00, 0x26), '9'),
            ((0x00, 0x27), '0'),
            ((0x00, 0x28), '\r'),
            ((0x00, 0x29), '\x1b'), // ESC
            ((0x00, 0x2a), '\x7f'), // DEL
            ((0x00, 0x2b), '\t'),
            ((0x00, 0x2c), ' '),
            ((0x00, 0x2d), '-'),
            ((0x00, 0x2e), '='),
            ((0x00, 0x2f), '['),
            ((0x00, 0x30), ']'),
            ((0x00, 0x31), '\\'),
            ((0x00, 0x33), ';'),
            ((0x00, 0x34), '\''),
            ((0x00, 0x36), ','),
            ((0x00, 0x37), '.'),
            ((0x00, 0x38), '/'),
            ((0x20, 0x04), 'A'),
            ((0x20, 0x05), 'B'),
            ((0x20, 0x06), 'C'),
            ((0x20, 0x07), 'D'),
            ((0x20, 0x08), 'E'),
            ((0x20, 0x09), 'F'),
            ((0x20, 0x0a), 'G'),
            ((0x20, 0x0b), 'H'),
            ((0x20, 0x0c), 'I'),
            ((0x20, 0x0d), 'J'),
            ((0x20, 0x0e), 'K'),
            ((0x20, 0x0f), 'L'),
            ((0x20, 0x10), 'M'),
            ((0x20, 0x11), 'N'),
            ((0x20, 0x12), 'O'),
            ((0x20, 0x13), 'P'),
            ((0x20, 0x14), 'Q'),
            ((0x20, 0x15), 'R'),
            ((0x20, 0x16), 'S'),
            ((0x20, 0x17), 'T'),
            ((0x20, 0x18), 'U'),
            ((0x20, 0x19), 'V'),
            ((0x20, 0x1a), 'W'),
            ((0x20, 0x1b), 'X'),
            ((0x20, 0x1c), 'Y'),
            ((0x20, 0x1d), 'Z'),
            ((0x20, 0x1e), '!'),
            ((0x20, 0x1f), '@'),
            ((0x20, 0x20), '#'),
            ((0x20, 0x21), '$'),
            ((0x20, 0x22), '%'),
            ((0x20, 0x23), '^'),
            ((0x20, 0x24), '&'),
            ((0x20, 0x25), '*'),
            ((0x20, 0x26), '('),
            ((0x20, 0x27), ')'),
            ((0x20, 0x28), '\r'),
            ((0x20, 0x29), '\x1b'), // ESC
            ((0x20, 0x2a), '\x7f'), // DEL
            ((0x20, 0x2b), '\t'),
            ((0x20, 0x2c), ' '),
            ((0x20, 0x2d), '_'),
            ((0x20, 0x2e), '+'),
            ((0x20, 0x2f), '{'),
            ((0x20, 0x30), '}'),
            ((0x20, 0x31), '|'),
            ((0x20, 0x33), ':'),
            ((0x20, 0x34), '"'),
            ((0x20, 0x36), '<'),
            ((0x20, 0x37), '>'),
            ((0x20, 0x38), '?'),
        ]
        .into_iter()
        .collect()
    }

    async fn packets_from_file(file: &str) -> Result<String> {
        let Output {
            status,
            stdout,
            stderr,
        } = Process::new("tshark")
            .args([
                "-r",
                file,
                "-2",
                "-R",
                "usb",
                "-T",
                "fields",
                "-e",
                "usb.capdata",
            ])
            .output()
            .await?;
        if !status.success() {
            let stderr = String::from_utf8(stderr)?;
            return Err(Error::Process(stderr).into());
        }

        String::from_utf8(stdout).map_err(Into::into)
    }

    fn traffic_from_packets(packets: &str) -> Vec<(u8, u8)> {
        packets
            .lines()
            .filter(|x| x.len() == 16)
            .flat_map(|x| {
                u8::from_str_radix(&x[..2], 16)
                    .ok()
                    .zip(u8::from_str_radix(&x[4..6], 16).ok())
            })
            .collect()
    }

    fn steg_from_traffic(traffic: Vec<(u8, u8)>) -> String {
        traffic.into_iter().flat_map(|x| KEY_MAP.get(&x)).collect()
    }
}

#[async_trait]
impl Command for KeyboardTrafficSteg {
    async fn execute(self: Box<Self>) -> Result<()> {
        let packets = Self::packets_from_file(&self.file).await?;
        let traffic = Self::traffic_from_packets(&packets);
        tracing::debug!(?traffic);

        let steg = Self::steg_from_traffic(traffic);
        tracing::info!(steg);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::KeyboardTrafficSteg;

    #[test]
    fn test_traffic_from_packets() {
        let packets = "0000090000000000\n0000000000000000\n00000f0000000000\n0000000000000000\n0000040000000000\n0000000000000000\n00000a0000000000\n0000000000000000\n2000000000000000\n20002f0000000000";
        let traffic = KeyboardTrafficSteg::traffic_from_packets(packets);
        assert_eq!(
            traffic,
            vec![
                (0, 9),
                (0, 0),
                (0, 15),
                (0, 0),
                (0, 4),
                (0, 0),
                (0, 10),
                (0, 0),
                (32, 0),
                (32, 47)
            ]
        );
    }

    #[test]
    fn test_steg_from_traffic() {
        let traffic = vec![
            (0, 9),
            (0, 0),
            (0, 15),
            (0, 0),
            (0, 4),
            (0, 0),
            (0, 10),
            (0, 0),
            (32, 0),
            (32, 47),
            (32, 0),
            (0, 0),
            (0, 19),
            (0, 0),
            (0, 21),
            (0, 0),
            (0, 32),
            (0, 0),
            (0, 34),
            (0, 0),
            (0, 34),
            (0, 0),
            (32, 0),
            (32, 45),
            (32, 0),
            (0, 0),
            (0, 39),
            (0, 0),
            (0, 17),
            (0, 0),
            (0, 26),
            (0, 0),
            (0, 4),
            (0, 0),
            (0, 21),
            (0, 0),
            (0, 7),
            (0, 0),
            (0, 22),
            (0, 0),
            (32, 0),
            (32, 45),
            (32, 0),
            (0, 0),
            (0, 4),
            (0, 0),
            (0, 31),
            (0, 0),
            (0, 9),
            (0, 0),
            (0, 8),
            (0, 0),
            (0, 8),
            (0, 0),
            (0, 35),
            (0, 0),
            (0, 8),
            (0, 0),
            (0, 39),
            (0, 0),
            (32, 0),
            (32, 48),
            (32, 0),
            (0, 0),
            (1, 0),
            (1, 6),
        ];
        let steg = KeyboardTrafficSteg::steg_from_traffic(traffic);
        assert_eq!(steg, "flag{pr355_0nwards_a2fee6e0}");
    }
}
