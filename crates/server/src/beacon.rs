//! Master-seed entropy from the public drand beacon.

use std::future::Future;
use std::time::Duration;

use serde::Deserialize;

const PRIMARY_URL: &str = "https://drand.cloudflare.com/public/latest";
const FALLBACK_URL: &str = "https://api.drand.sh/public/latest";
const FETCH_URLS: [&str; 2] = [PRIMARY_URL, FALLBACK_URL];
const MAX_FETCH_ATTEMPTS: usize = 3;
const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
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
        let client = http_client()?;
        for attempt in 0..MAX_FETCH_ATTEMPTS {
            let url = FETCH_URLS[attempt % FETCH_URLS.len()];
            if let Ok(entropy) = fetch_from_url(&client, url).await {
                return Ok(entropy);
            }
        }
        Err(BeaconError::Unavailable)
    }
}

fn http_client() -> Result<reqwest::Client, BeaconError> {
    reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .connect_timeout(FETCH_TIMEOUT)
        .build()
        .map_err(|_| BeaconError::Unavailable)
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
            master_seed: parse_master_seed_hex(seed)?,
            beacon_round: 0,
        });
    }

    if let Some(seed) = env_master_seed()? {
        return Ok(MasterEntropy {
            master_seed: seed,
            beacon_round: 0,
        });
    }

    source.latest().await
}

fn env_master_seed() -> Result<Option<[u8; 32]>, BeaconError> {
    match std::env::var(MASTER_SEED_ENV) {
        Ok(seed) => Ok(Some(parse_master_seed_hex(&seed)?)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(BeaconError::InvalidRandomness),
    }
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
        master_seed: parse_master_seed_hex(&beacon.randomness)?,
        beacon_round: beacon.round,
    })
}

fn parse_master_seed_hex(hex: &str) -> Result<[u8; 32], BeaconError> {
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
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const TEST_SEED_HEX: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
    const TEST_SEED_BYTES: [u8; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];

    static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

    struct CountingSource {
        calls: Arc<AtomicUsize>,
    }

    impl EntropySource for CountingSource {
        async fn latest(&self) -> Result<MasterEntropy, BeaconError> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            Err(BeaconError::Unavailable)
        }
    }

    fn with_env_var<T>(name: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_TEST_LOCK.lock().expect("env test lock");
        let previous = std::env::var(name).ok();
        // SAFETY: `ENV_TEST_LOCK` serializes env mutation across parallel tests.
        unsafe {
            match value {
                Some(seed) => std::env::set_var(name, seed),
                None => std::env::remove_var(name),
            }
        }

        let result = f();

        // SAFETY: same lock; restore the prior value for other tests/processes.
        unsafe {
            match previous {
                Some(seed) => std::env::set_var(name, seed),
                None => std::env::remove_var(name),
            }
        }

        result
    }

    #[test]
    fn parse_master_seed_hex_accepts_64_char_hex() {
        assert_eq!(
            parse_master_seed_hex(TEST_SEED_HEX).expect("valid hex"),
            TEST_SEED_BYTES
        );
    }

    #[test]
    fn parse_master_seed_hex_rejects_bad_length() {
        assert_eq!(
            parse_master_seed_hex("deadbeef"),
            Err(BeaconError::InvalidRandomness)
        );
    }

    #[test]
    fn env_master_seed_reads_mtgfr_master_seed() {
        with_env_var(MASTER_SEED_ENV, Some(TEST_SEED_HEX), || {
            assert_eq!(
                env_master_seed().expect("env read succeeds"),
                Some(TEST_SEED_BYTES)
            );
        });
    }

    #[test]
    fn env_master_seed_absent_returns_none() {
        with_env_var(MASTER_SEED_ENV, None, || {
            assert_eq!(env_master_seed().expect("env read succeeds"), None);
        });
    }

    #[tokio::test]
    async fn env_master_seed_skips_network() {
        let calls = Arc::new(AtomicUsize::new(0));
        let source = CountingSource {
            calls: Arc::clone(&calls),
        };

        let entropy = resolve_entropy_with_env(&source, Some(TEST_SEED_HEX))
            .await
            .expect("override seed resolves");

        assert_eq!(entropy.beacon_round, 0);
        assert_eq!(entropy.master_seed, TEST_SEED_BYTES);
        assert_eq!(
            calls.load(Ordering::Relaxed),
            0,
            "network source is skipped"
        );
    }

    #[test]
    fn mtgfr_master_seed_env_var_skips_network() {
        with_env_var(MASTER_SEED_ENV, Some(TEST_SEED_HEX), || {
            let calls = Arc::new(AtomicUsize::new(0));
            let source = CountingSource {
                calls: Arc::clone(&calls),
            };
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("test runtime");
            let entropy = runtime
                .block_on(resolve_entropy_with_env(&source, None))
                .expect("MTGFR_MASTER_SEED resolves");

            assert_eq!(entropy.beacon_round, 0);
            assert_eq!(entropy.master_seed, TEST_SEED_BYTES);
            assert_eq!(
                calls.load(Ordering::Relaxed),
                0,
                "network source is skipped when MTGFR_MASTER_SEED is set"
            );
        });
    }
}
