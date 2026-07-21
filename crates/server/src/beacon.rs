//! Master-seed entropy from the public drand beacon.

use std::future::Future;

use serde::Deserialize;

const PRIMARY_URL: &str = "https://drand.cloudflare.com/public/latest";
const FALLBACK_URL: &str = "https://api.drand.sh/public/latest";
const RETRIES: usize = 3;
const MASTER_SEED_ENV: &str = "MTGFR_MASTER_SEED";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MasterEntropy {
    pub master_seed: [u8; 32],
    pub beacon_round: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BeaconError {
    InvalidRandomness,
    Unavailable,
}

impl std::fmt::Display for BeaconError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BeaconError::InvalidRandomness => write!(f, "beacon randomness was not 32-byte hex"),
            BeaconError::Unavailable => write!(f, "beacon entropy unavailable"),
        }
    }
}

impl std::error::Error for BeaconError {}

pub trait EntropySource {
    fn latest(&self) -> impl Future<Output = Result<MasterEntropy, BeaconError>> + Send;
}

pub struct HttpEntropySource;

impl EntropySource for HttpEntropySource {
    async fn latest(&self) -> Result<MasterEntropy, BeaconError> {
        let client = reqwest::Client::new();
        for _ in 0..RETRIES {
            for url in [PRIMARY_URL, FALLBACK_URL] {
                if let Ok(entropy) = fetch_from_url(&client, url).await {
                    return Ok(entropy);
                }
            }
        }
        Err(BeaconError::Unavailable)
    }
}

pub async fn fetch_master_entropy() -> Result<MasterEntropy, BeaconError> {
    resolve_entropy_with_env(&HttpEntropySource, None).await
}

pub(crate) async fn resolve_entropy_with_env(
    source: &impl EntropySource,
    master_seed_override: Option<&str>,
) -> Result<MasterEntropy, BeaconError> {
    if let Some(seed) = master_seed_override {
        return Ok(MasterEntropy {
            master_seed: hex_to_32(seed)?,
            beacon_round: 0,
        });
    }

    let env_seed = std::env::var(MASTER_SEED_ENV).ok();
    if let Some(seed) = env_seed.as_deref() {
        return Ok(MasterEntropy {
            master_seed: hex_to_32(seed)?,
            beacon_round: 0,
        });
    }

    source.latest().await
}

#[derive(Deserialize)]
struct DrandLatest {
    round: u64,
    randomness: String,
}

async fn fetch_from_url(client: &reqwest::Client, url: &str) -> Result<MasterEntropy, BeaconError> {
    let beacon = client
        .get(url)
        .send()
        .await
        .map_err(|_| BeaconError::Unavailable)?
        .error_for_status()
        .map_err(|_| BeaconError::Unavailable)?
        .json::<DrandLatest>()
        .await
        .map_err(|_| BeaconError::Unavailable)?;

    // ponytail: HTTPS + Cloudflare/drand relay is enough for v1; verify the BLS signature if
    // seed provenance ever needs to survive a compromised transport/provider.
    Ok(MasterEntropy {
        master_seed: hex_to_32(&beacon.randomness)?,
        beacon_round: beacon.round,
    })
}

fn hex_to_32(hex: &str) -> Result<[u8; 32], BeaconError> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return Err(BeaconError::InvalidRandomness);
    }

    let mut out = [0; 32];
    for i in 0..32 {
        let hi = hex_nibble(bytes[i * 2]).ok_or(BeaconError::InvalidRandomness)?;
        let lo = hex_nibble(bytes[i * 2 + 1]).ok_or(BeaconError::InvalidRandomness)?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSource {
        calls: Arc<AtomicUsize>,
    }

    impl EntropySource for CountingSource {
        async fn latest(&self) -> Result<MasterEntropy, BeaconError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Err(BeaconError::Unavailable)
        }
    }

    #[tokio::test]
    async fn env_master_seed_skips_network() {
        let calls = Arc::new(AtomicUsize::new(0));
        let source = CountingSource {
            calls: Arc::clone(&calls),
        };
        let env_seed = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

        let entropy = resolve_entropy_with_env(&source, Some(env_seed))
            .await
            .expect("env seed resolves");

        assert_eq!(entropy.beacon_round, 0);
        assert_eq!(
            entropy.master_seed,
            [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25, 26, 27, 28, 29, 30, 31,
            ]
        );
        assert_eq!(
            calls.load(Ordering::Relaxed),
            0,
            "network source is skipped"
        );
    }
}
