use crate::models::percent_encode;
use url::Url;

#[derive(Debug, Clone, Default)]
pub struct UrlWrapper {
    pub scheme: Option<String>,
    pub host: Option<String>,
    pub path: Option<String>,
    pub query: Option<String>,
    pub fragment: Option<String>,
}

impl UrlWrapper {
    pub fn parse(raw: &str) -> Option<Self> {
        match Url::parse(raw) {
            Ok(u) => Some(Self {
                scheme: Some(u.scheme().to_string()),
                host: u.host_str().map(|s| s.to_string()),
                path: Some(u.path().to_string()),
                query: u.query().map(|s| s.to_string()),
                fragment: u.fragment().map(|s| s.to_string()),
            }),
            Err(_) => None,
        }
    }

    pub fn string(&self) -> String {
        let mut out = String::new();
        if let Some(scheme) = &self.scheme {
            out.push_str(scheme);
            out.push_str("://");
        }
        if let Some(host) = &self.host {
            out.push_str(host);
        }
        if let Some(path) = &self.path {
            if !path.is_empty() {
                out.push_str(path);
            } else {
                out.push('/');
            }
        }
        if let Some(query) = &self.query {
            out.push('?');
            out.push_str(query);
        }
        if let Some(fragment) = &self.fragment {
            out.push('#');
            out.push_str(fragment);
        }
        out
    }
}

#[derive(Debug, Clone, Default)]
pub struct Params {
    pub params: std::collections::HashMap<String, Vec<String>>,
}

impl Params {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.entry(key.into()).or_default().push(value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).and_then(|v| v.first()).map(|s| s.as_str())
    }

    pub fn encode(&self) -> String {
        let mut parts = Vec::new();
        for (key, values) in &self.params {
            for value in values {
                parts.push(format!(
                    "{}={}",
                    percent_encode(key),
                    percent_encode(value)
                ));
            }
        }
        parts.join("&")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Headers {
    pub headers: reqwest::header::HeaderMap,
    pub order: Vec<String>,
    pub pseudo_order: Vec<String>,
}

impl Headers {
    pub fn new() -> Self {
        Self {
            headers: reqwest::header::HeaderMap::new(),
            order: Vec::new(),
            pseudo_order: vec![
                ":method".to_string(),
                ":authority".to_string(),
                ":scheme".to_string(),
                ":path".to_string(),
            ],
        }
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value) {
                self.headers.insert(header_name, header_value);
                if !self.order.contains(&key) {
                    self.order.push(key);
                }
            }
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Values {
    pub values: std::collections::HashMap<String, Vec<String>>,
    pub index: Vec<String>,
}

impl Values {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        self.values.insert(key.clone(), vec![value]);
        if !self.index.contains(&key) {
            self.index.push(key);
        }
    }

    pub fn add(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        self.values.entry(key.clone()).or_default().push(value);
        if !self.index.contains(&key) {
            self.index.push(key);
        }
    }

    pub fn encode(&self) -> String {
        let mut parts = Vec::new();
        for key in &self.index {
            if let Some(values) = self.values.get(key) {
                for value in values {
                    parts.push(format!(
                        "{}={}",
                        percent_encode(key),
                        percent_encode(value)
                    ));
                }
            }
        }
        parts.join("&")
    }
}

#[derive(Debug, Clone, Default)]
pub struct Files {
    pub files: Vec<(String, FilePart)>,
}

#[derive(Debug, Clone)]
pub enum FilePart {
    Field { name: String, value: String },
    File { name: String, filename: String, content_type: String, path: std::path::PathBuf },
}

impl Files {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_field(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.files.push((
            name.into(),
            FilePart::Field {
                name: String::new(),
                value: value.into(),
            },
        ));
    }

    pub fn add_file(
        &mut self,
        name: impl Into<String>,
        filename: impl Into<String>,
        path: impl Into<std::path::PathBuf>,
        content_type: impl Into<String>,
    ) {
        self.files.push((
            name.into(),
            FilePart::File {
                name: String::new(),
                filename: filename.into(),
                content_type: content_type.into(),
                path: path.into(),
            },
        ));
    }
}
