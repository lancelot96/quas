use std::{
    collections::{BTreeSet, HashSet},
    fmt,
    path::PathBuf,
    process::Output,
};

use aes::{
    cipher::{BlockDecrypt, KeyInit},
    Aes128Dec, Block,
};
use anyhow::Result;
use async_trait::async_trait;
use base64::{
    alphabet::STANDARD,
    engine::{
        general_purpose::{GeneralPurpose, GeneralPurposeConfig},
        DecodePaddingMode,
    },
    Engine,
};
use infer::{Infer, MatcherType, Type};
use regex::Regex;
use serde_json::Value;
use tokio::{fs, process::Command as Process};

use crate::{error::Error, Command};

#[derive(Debug)]
pub struct BehinderTrafficAnalyse {
    file: String,
    outdir: PathBuf,
    key: Option<String>,
}

impl BehinderTrafficAnalyse {
    pub fn new(file: String, outdir: PathBuf, key: Option<String>) -> Self {
        Self { file, outdir, key }
    }

    async fn get_packets(file: &str) -> Result<Vec<(String, Vec<u8>)>> {
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
                "http",
                "-T",
                "fields",
                "-e",
                "frame.number",
                "-e",
                "http.file_data",
            ])
            .output()
            .await?;
        if !status.success() {
            let stderr = String::from_utf8(stderr)?;
            return Err(Error::Process(stderr).into());
        }

        let responses = String::from_utf8(stdout)?
            .lines()
            .flat_map(|x| x.split_once('\t'))
            .flat_map(|(i, x)| hex::decode(x).map(|x| (i.to_owned(), x)))
            .collect();
        Ok(responses)
    }

    fn key_from_packets(packets: &[(String, Vec<u8>)]) -> Option<String> {
        let pattern = Regex::new(r#""(\w{16})""#).unwrap();

        let keys = packets
            .iter()
            .map(|(_, x)| x)
            .cloned()
            .flat_map(String::from_utf8)
            .flat_map(|x| {
                pattern
                    .captures_iter(&x)
                    .map(|x| x[1].to_owned())
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<_>>();
        tracing::info!(?keys);

        keys.into_iter().next()
    }
}

#[async_trait]
impl Command for BehinderTrafficAnalyse {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self { file, outdir, key } = *self;
        if !outdir.is_dir() {
            fs::create_dir_all(&outdir).await?;
        }

        let packets = Self::get_packets(&file).await?;
        let Some(key) = key.or_else(|| Self::key_from_packets(&packets)) else {
            return Err(anyhow::anyhow!("No key found."));
        };

        let cipher = Aes128Dec::new_from_slice(key.as_bytes())?;
        Extractor::new(outdir, cipher)
            .steg_from_packets(packets)
            .await
    }
}

struct Extractor {
    outdir: PathBuf,
    cipher: Aes128Dec,
    info: Infer,
    alphabet: BTreeSet<u8>,
    base64: GeneralPurpose,
}

impl Extractor {
    fn new(outdir: PathBuf, cipher: Aes128Dec) -> Self {
        let base64_config = GeneralPurposeConfig::new()
            .with_decode_allow_trailing_bits(true)
            .with_decode_padding_mode(DecodePaddingMode::Indifferent);

        Self {
            outdir,
            cipher,
            info: Infer::new(),
            alphabet: STANDARD.as_str().bytes().collect(),
            base64: GeneralPurpose::new(&STANDARD, base64_config),
        }
    }

    fn dejson_base64_nested(&self, json: &mut Value) {
        match json {
            Value::String(s) => {
                let Ok(base64) = self.base64.decode(s) else {
                    return;
                };

                if let Ok(v) = serde_json::from_slice::<Value>(&base64) {
                    *json = v;
                    return self.dejson_base64_nested(json);
                }
                if let Ok(v) = String::from_utf8(base64) {
                    *json = Value::String(v);
                }
            }
            Value::Array(a) => a.iter_mut().for_each(|x| self.dejson_base64_nested(x)),
            Value::Object(o) => o.values_mut().for_each(|x| self.dejson_base64_nested(x)),
            _ => (),
        }
    }

    async fn steg_from_packets(&self, packets: Vec<(String, Vec<u8>)>) -> Result<()> {
        for (frame_id, packet) in packets {
            self.steg_from_packet(frame_id, packet).await?;
        }

        Ok(())
    }

    async fn steg_from_packet(&self, frame_id: String, packet: Vec<u8>) -> Result<()> {
        let kind = self
            .info
            .get(&packet)
            .unwrap_or_else(|| Type::new(MatcherType::Custom, "unknown", "unknown", |_| true));
        tracing::debug!(?kind);

        let path = self.outdir.join(frame_id);
        let (file, data) = match kind.extension() {
            "html" => return Ok(()),
            "unknown" => {
                let mut packet_len = packet
                    .iter()
                    .take_while(|x| self.alphabet.contains(x))
                    .count();
                if packet_len & 0b11 == 1 {
                    packet_len &= !0 << 1;
                }

                let Some(data) = self
                    .base64
                    .decode(&packet[..packet_len])
                    .ok()
                    .filter(|x| !x.is_empty())
                    .map(|x| self.decrypt_packet(&x))
                else {
                    return Ok(());
                };

                let json_len = data.iter().take_while(|x| x.is_ascii_graphic()).count();
                match serde_json::from_slice::<Value>(&data[..json_len]) {
                    Ok(mut json) => {
                        self.dejson_base64_nested(&mut json);
                        let file = path.with_extension("json");
                        let json_str = serde_json::to_string(&json)?.into_bytes();
                        (file, json_str)
                    }
                    Err(_) => (path, data),
                }
            }
            _ => {
                let file = path.with_extension(kind.extension());
                (file, packet)
            }
        };

        tracing::info!(?file);
        fs::write(file, data).await.map_err(Into::into)
    }

    fn decrypt_packet(&self, packet: &[u8]) -> Vec<u8> {
        let mut packet = packet.to_owned();
        packet
            .chunks_exact_mut(16)
            .map(Block::from_mut_slice)
            .for_each(|x| self.cipher.decrypt_block(x));

        packet
    }
}

impl fmt::Debug for Extractor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Extractor")
            .field("cipher", &self.cipher)
            .field("alphabet", &self.alphabet)
            .finish()
    }
}
