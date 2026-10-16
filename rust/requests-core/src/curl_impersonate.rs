use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::path::PathBuf;
use std::process::Command;

use crate::error::{RequestsError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserProfile {
    Chrome110,
    Chrome120,
    Firefox,
    Safari,
    Edge,
}

impl BrowserProfile {
    pub fn binary_name(&self) -> &'static str {
        match self {
            BrowserProfile::Chrome110 | BrowserProfile::Chrome120 => "curl-impersonate-chrome",
            BrowserProfile::Firefox => "curl-impersonate-firefox",
            BrowserProfile::Safari => "curl-impersonate-safari",
            BrowserProfile::Edge => "curl-impersonate-edge",
        }
    }

    pub fn profile_name(&self) -> &'static str {
        match self {
            BrowserProfile::Chrome110 => "chrome110",
            BrowserProfile::Chrome120 => "chrome120",
            BrowserProfile::Firefox => "firefox147",
            BrowserProfile::Safari => "safari155",
            BrowserProfile::Edge => "edge101",
        }
    }

    pub fn extra_args(&self) -> Vec<&'static str> {
        match self {
            BrowserProfile::Chrome110 | BrowserProfile::Chrome120 => vec!["--curl-impersonate", "chrome110"],
            BrowserProfile::Firefox => vec!["--curl-impersonate", "firefox147"],
            BrowserProfile::Safari => vec!["--curl-impersonate", "safari155"],
            BrowserProfile::Edge => vec!["--curl-impersonate", "edge101"],
        }
    }
}

#[derive(Debug, Clone)]
pub struct CurlImpersonateConfig {
    pub binary_path: PathBuf,
    pub profile: BrowserProfile,
    pub timeout_secs: u64,
}

impl Default for CurlImpersonateConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("curl-impersonate"),
            profile: BrowserProfile::Chrome110,
            timeout_secs: 30,
        }
    }
}

impl CurlImpersonateConfig {
    pub fn new(binary_path: impl Into<PathBuf>, profile: BrowserProfile) -> Self {
        Self {
            binary_path: binary_path.into(),
            profile,
            ..Default::default()
        }
    }

    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub fn execute(&self, method: &str, url: &str, headers: &[String], body: Option<&[u8]>) -> Result<(u16, HeaderMap, Vec<u8>)> {
        let mut cmd = Command::new(&self.binary_path);
        let profile_name = self.profile.profile_name();
        cmd.env("CURL_IMPERSONATE", profile_name)
            .arg("-X")
            .arg(method)
            .arg(url)
            .arg("--max-time")
            .arg(self.timeout_secs.to_string())
            .arg("-sS")
            .arg("-w")
            .arg("\n%{http_code}")
            .arg("-D")
            .arg("-");

        for h in headers {
            cmd.arg("-H").arg(h);
        }

        if let Some(b) = body {
            cmd.arg("-d").arg(std::str::from_utf8(b).unwrap_or(""));
        }

        let output = cmd.output().map_err(|e| RequestsError::Tls(format!("failed to spawn curl-impersonate: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(RequestsError::Tls(format!("curl-impersonate failed: {stderr}")));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let parts: Vec<&str> = stdout.rsplitn(2, "\r\n\r\n").collect();

        let (header_part, body_part) = if parts.len() >= 2 {
            (parts[1], parts[0])
        } else {
            ("HTTP/1.1 200 OK", parts[0])
        };

        let mut header_map = HeaderMap::new();
        for line in header_part.lines() {
            if let Some(header) = line.strip_suffix("\r") {
                if let Some((name, value)) = header.split_once(": ") {
                    if let (Ok(name), Ok(value)) = (
                        HeaderName::from_bytes(name.as_bytes()),
                        HeaderValue::from_str(value),
                    ) {
                        header_map.insert(name, value);
                    }
                }
            }
        }

        let status_code = header_part
            .lines()
            .last()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(200);

        Ok((status_code, header_map, body_part.as_bytes().to_vec()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_profile_binary_name() {
        assert_eq!(BrowserProfile::Chrome110.binary_name(), "curl-impersonate-chrome");
        assert_eq!(BrowserProfile::Firefox.binary_name(), "curl-impersonate-firefox");
    }

    #[test]
    fn test_browser_profile_extra_args() {
        let args = BrowserProfile::Chrome110.extra_args();
        assert!(args.contains(&"--curl-impersonate"));
    }
}
