use std::sync::Arc;

use crate::transport::{JA3Fingerprint, JA3Parsed, TLSExtensions};
use crate::error::Result;
use rustls::{
    pki_types::{CertificateDer, ServerName},
};

/// Parse ALPN from JA3 extension list.
fn alpn_from_extensions(extensions: &[u16]) -> Vec<Vec<u8>> {
    let mut alpn = Vec::new();
    if extensions.contains(&0x0010) {
        alpn.push(b"h2".to_vec());
    }
    if extensions.iter().any(|&e| e == 0x0000 || e == 0x10) {
        alpn.push(b"http/1.1".to_vec());
    }
    if alpn.is_empty() {
        alpn.push(b"http/1.1".to_vec());
    }
    alpn
}

/// Build a rustls `ClientConfig` from session TLS settings.
pub fn build_rustls_client_config(
    verify: bool,
    ja3: &Option<JA3Fingerprint>,
    tls_extensions: &Option<TLSExtensions>,
    _tls_min_version: &Option<String>,
    _tls_max_version: &Option<String>,
    tls_alpn: &Option<Vec<String>>,
) -> Result<Arc<rustls::ClientConfig>> {
    let parsed_ja3 = ja3.as_ref().and_then(|j| {
        if j.random_ja3 {
            Some(generate_random_ja3())
        } else {
            j.parse_ja3()
        }
    });

    let provider = Arc::new(rustls::crypto::ring::default_provider());

    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| crate::RequestsError::Tls(e.to_string()))?;

    let mut config = builder
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();

    if !verify {
        config
            .dangerous()
            .set_certificate_verifier(Arc::new(AllowAllVerifier));
    }

    // ALPN from JA3 extensions or explicit config
    if let Some(ref alpn) = tls_alpn {
        if !alpn.is_empty() {
            config.alpn_protocols = alpn.iter().map(|s| s.clone().into_bytes()).collect();
        }
    } else if let Some(ref parsed) = parsed_ja3 {
        let alpn = alpn_from_extensions(&parsed.extensions);
        config.alpn_protocols = alpn;
    }

    // Apply TLS extension customizations if provided.
    if let Some(ext) = tls_extensions {
        apply_tls_extensions(&mut config, ext);
    }

    Ok(Arc::new(config))
}

/// Generate a randomized JA3-like fingerprint for anti-fingerprinting.
///
/// This does not yet change the wire JA3 bytes because rustls 0.23 does
/// not expose that control. It is preserved as a hook for future
/// lower-level TLS integration.
pub fn generate_random_ja3() -> JA3Parsed {
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    let mut rng = thread_rng();

    let all_ciphers = vec![
        0x009C, 0x009D, 0xC02F, 0xC030, 0xC02B, 0xC02C,
        0xCCA9, 0xCCA8, 0x002F, 0x003C, 0x000A, 0x0035,
    ];
    let mut ciphers = all_ciphers.clone();
    ciphers.shuffle(&mut rng);
    let ciphers: Vec<u16> = ciphers.into_iter().take(6).collect();

    let all_extensions = vec![
        0x0000, 0x0005, 0x000A, 0x000B, 0x000D, 0x0010,
        0x0011, 0x0012, 0x0013, 0x0014, 0x0017, 0x001B,
        0x001C, 0x0023, 0x002B, 0x002D, 0x3374, 0x4469,
        0x7550, 0xFE0D, 0xFE0E, 0xFE1D, 0x756B, 0x756D,
    ];
    let mut extensions = all_extensions.clone();
    extensions.shuffle(&mut rng);
    let extensions: Vec<u16> = extensions.into_iter().take(12).collect();

    let all_curves = vec![0x001E, 0x0017, 0x0016, 0x0010, 0x0019, 0x0018];
    let mut curves = all_curves.clone();
    curves.shuffle(&mut rng);
    let curves: Vec<u16> = curves.into_iter().take(4).collect();

    let ec_formats = vec![0, 1, 2];

    let all_sig_algs = vec![
        0x0403, 0x0503, 0x0603, 0x0804, 0x0813, 0x0823,
        0x0833, 0x0401, 0x0501, 0x0601, 0x0301, 0x0302,
        0x0303, 0x0201, 0x0203,
    ];
    let mut sig_algs = all_sig_algs.clone();
    sig_algs.shuffle(&mut rng);
    let sig_algs: Vec<u16> = sig_algs.into_iter().take(8).collect();

    JA3Parsed {
        cipher_suites: ciphers,
        extensions,
        curves,
        ec_point_formats: ec_formats,
        sig_algs,
    }
}

fn apply_tls_extensions(_config: &mut rustls::ClientConfig, _ext: &TLSExtensions) {
    let _ = _ext;
}

#[derive(Debug)]
struct AllowAllVerifier;

impl rustls::client::danger::ServerCertVerifier for AllowAllVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
        ]
    }
}
