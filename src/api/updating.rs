//! Types for updating functionality.

use std::ops::Deref;
use std::str::FromStr;

use anyhow::{Context as _, Error};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

/// A SHA-256 sum in hexadecimal representation is 64 characters long.
const SHA256_SUM_HEX_LENGTH: usize = 64;

#[derive(Debug)]
pub struct Sha256Sum([u8; 32]);

#[derive(Debug, Deserialize)]
pub struct ReleaseRegistryFile {
    pub url: String,
    #[serde(rename = "checksums")]
    #[serde(deserialize_with = "deserialize_checksums")]
    pub checksum: Sha256Sum,
}

fn deserialize_checksums<'de, D>(deserializer: D) -> Result<Sha256Sum, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(rename_all = "kebab-case")]
    struct RawChecksumsMapping {
        sha256_hex: String,
    }

    let RawChecksumsMapping { sha256_hex } = RawChecksumsMapping::deserialize(deserializer)?;
    sha256_hex.parse().map_err(D::Error::custom)
}

impl FromStr for Sha256Sum {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != SHA256_SUM_HEX_LENGTH {
            anyhow::bail!(
                "cannot parse SHA-256: expected a {SHA256_SUM_HEX_LENGTH}-character long string"
            );
        }

        let mut bytes = [0u8; 32];

        bytes
            .iter_mut()
            .zip(s.as_bytes().chunks(2))
            .map(|(byte, hex_byte)| {
                let hex_str = str::from_utf8(hex_byte)?;
                *byte = u8::from_str_radix(hex_str, 16)?;
                Ok::<_, Self::Err>(())
            })
            .map(|result| result.context("cannot parse SHA-256: not a valid hex string"))
            .collect::<Result<Vec<()>, _>>()?;

        Ok(Sha256Sum(bytes))
    }
}

impl Deref for Sha256Sum {
    type Target = [u8; 32];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Rhs> PartialEq<Rhs> for Sha256Sum
where
    Rhs: Deref<Target = [u8]>,
{
    fn eq(&self, other: &Rhs) -> bool {
        self.0 == **other
    }
}
