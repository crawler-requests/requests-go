#[derive(Debug, Clone, Default)]
pub struct TransportConfig {
    pub proxies: Option<String>,
    pub verify: bool,
    pub cert: Vec<String>,
    pub ja3: Option<String>,
    pub random_ja3: bool,
    pub force_http1: bool,
    pub tls_extensions: Option<String>,
    pub http2_settings: Option<String>,
}

impl TransportConfig {
    pub fn apply_client(&self, builder: reqwest::ClientBuilder) -> reqwest::ClientBuilder {
        let mut builder = builder;

        if let Some(proxy_url) = &self.proxies {
            if let Ok(proxy) = reqwest::Proxy::all(proxy_url) {
                builder = builder.proxy(proxy);
            }
        }

        if !self.verify {
            builder = builder.danger_accept_invalid_certs(true);
        }

        if self.force_http1 {
            builder = builder.http1_only();
        }

        builder
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct H2Settings {
    pub settings: std::collections::HashMap<String, u32>,
    pub settings_ack: bool,
    pub settings_order: Vec<String>,
    pub connection_flow: i64,
    pub headers_id: u32,
    pub header_priority: Option<H2PriorityParam>,
    pub priority_frames: Vec<H2PriorityFrame>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct H2PriorityParam {
    pub stream_dep: u32,
    pub exclusive: bool,
    pub weight: Option<u8>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct H2PriorityFrame {
    pub stream_id: u32,
    pub priority_param: H2PriorityParam,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct TLSExtensions {
    pub supported_signature_algorithms: Vec<String>,
    pub cert_compression_algo: Vec<String>,
    pub record_size_limit: Option<u16>,
    pub delegated_credentials: Vec<String>,
    pub supported_versions: Vec<String>,
    pub psk_key_exchange_modes: Vec<String>,
    pub signature_algorithms_cert: Vec<String>,
    pub key_share_curves: Vec<String>,
    pub not_used_grease: bool,
    pub client_hello_hex_stream: Option<String>,
}

impl TLSExtensions {
    pub fn from_json(json: Option<&str>) -> Option<Self> {
        if json.is_none() {
            return None;
        }
        let json = json.unwrap();
        if json.is_empty() {
            return None;
        }
        match serde_json::from_str::<Self>(json) {
            Ok(ext) => Some(ext),
            Err(_) => None,
        }
    }
}

impl TLSExtensions {
    pub fn to_tls_extensions(&self) -> Option<Self> {
        Some(self.clone())
    }
}

impl H2Settings {
    pub fn from_json(json: Option<&str>) -> Option<Self> {
        if json.is_none() {
            return None;
        }
        let json = json.unwrap();
        if json.is_empty() {
            return None;
        }
        match serde_json::from_str::<Self>(json) {
            Ok(settings) => Some(settings),
            Err(_) => None,
        }
    }
}

impl H2Settings {
    pub fn to_http2_settings(&self) -> Option<Self> {
        Some(self.clone())
    }
}

#[derive(Debug, Clone, Default)]
pub struct JA3Fingerprint {
    pub ja3: Option<String>,
    pub random_ja3: bool,
    pub tls_extensions: Option<String>,
}

impl JA3Fingerprint {
    pub fn new(ja3: Option<String>, random_ja3: bool, tls_extensions: Option<String>) -> Self {
        Self {
            ja3,
            random_ja3,
            tls_extensions,
        }
    }

    pub fn parse_ja3(&self) -> Option<JA3Parsed> {
        let ja3_str = self.ja3.as_ref()?;
        let parts: Vec<&str> = ja3_str.split(',').collect();
        if parts.len() < 3 {
            return None;
        }

        let parse_list = |s: &str| -> Vec<u16> {
            s.split('-')
                .filter_map(|x| {
                    let trimmed = x.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    u16::from_str_radix(trimmed, 10).ok()
                })
                .collect()
        };

        let cipher_suites = parse_list(parts[0]);
        let extensions = parse_list(parts[1]);
        let curves = parse_list(parts[2]);

        let ec_point_formats = parts.get(3).map(|s| {
            s.split('-')
                .filter_map(|x| x.trim().parse::<u8>().ok())
                .collect()
        }).unwrap_or_default();

        let sig_algs = parts.get(4).map(|s| {
            s.split('-')
                .filter_map(|x| u16::from_str_radix(x.trim(), 10).ok())
                .collect()
        }).unwrap_or_default();

        Some(JA3Parsed {
            cipher_suites,
            extensions,
            curves,
            ec_point_formats,
            sig_algs,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct JA3Parsed {
    pub cipher_suites: Vec<u16>,
    pub extensions: Vec<u16>,
    pub curves: Vec<u16>,
    pub ec_point_formats: Vec<u8>,
    pub sig_algs: Vec<u16>,
}
