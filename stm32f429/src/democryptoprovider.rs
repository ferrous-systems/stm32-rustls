use rustls::crypto::CryptoProvider;

use crate::RNG_MUTEX;
mod aead;
mod hash;
mod hmac;
mod kx;

static ALL_CIPHER_SUITES: &[rustls::SupportedCipherSuite] =
    &[TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256];

pub static TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256: rustls::SupportedCipherSuite =
    rustls::SupportedCipherSuite::Tls12(&rustls::Tls12CipherSuite {
        common: rustls::cipher_suite::CipherSuiteCommon {
            suite: rustls::CipherSuite::TLS_ECDHE_RSA_WITH_CHACHA20_POLY1305_SHA256,
            hash_provider: &hash::Sha256,
        },
        kx: rustls::crypto::KeyExchangeAlgorithm::ECDHE,
        sign: &[
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
        ],
        prf_provider: &rustls::crypto::tls12::PrfUsingHmac(&hmac::Sha256Hmac),
        aead_alg: &aead::Chacha20Poly1305,
    });
#[derive(Debug)]
pub struct DemoCryptoProvider;
impl CryptoProvider for DemoCryptoProvider {
    fn fill_random(&self, bytes: &mut [u8]) -> Result<(), rustls::crypto::GetRandomFailed> {
        // This is a non async task so I need embassy_futures
        embassy_futures::block_on(async {
            let mut binding = RNG_MUTEX.lock().await;
            let rng = binding.as_mut().unwrap();
            rng.async_fill_bytes(bytes)
                .await
                .map_err(|_| rustls::crypto::GetRandomFailed)
        })
    }

    fn default_cipher_suites(&self) -> &'static [rustls::SupportedCipherSuite] {
        ALL_CIPHER_SUITES
    }

    fn default_kx_groups(&self) -> &'static [&'static dyn rustls::crypto::SupportedKxGroup] {
        kx::ALL_KX_GROUPS
    }
}
