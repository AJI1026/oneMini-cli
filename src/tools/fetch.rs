use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use super::{truncate_output, Tool};
use crate::fs_util::secure_http_client;

const TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_CHARS: usize = 50_000;
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

pub struct FetchTool;

impl FetchTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, serde::Serialize)]
struct FetchResult {
    url: String,
    final_url: String,
    status: u16,
    content_type: Option<String>,
    body: String,
    truncated: bool,
}

#[async_trait]
impl Tool for FetchTool {
    fn name(&self) -> &str {
        "fetch"
    }

    fn description(&self) -> &str {
        "通过 HTTPS 获取网页或 API 内容（只读）。仅支持 https:// URL，禁止访问内网/本地地址。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "HTTPS URL，例如 https://www.google.com" },
                "max_chars": { "type": "integer", "description": "返回正文最大字符数（默认 50000）" }
            },
            "required": ["url"]
        })
    }

    fn requires_approval(&self, _args: &Value) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let url = args["url"]
            .as_str()
            .context("缺少 url 参数")?
            .trim();
        let max_chars = args["max_chars"]
            .as_u64()
            .map(|n| n as usize)
            .unwrap_or(DEFAULT_MAX_CHARS)
            .clamp(1_000, DEFAULT_MAX_CHARS);

        validate_public_https_url(url)?;

        let client = secure_http_client("onemini-cli-fetch", TIMEOUT_SECS)?;
        let resp = client
            .get(url)
            .send()
            .await
            .with_context(|| format!("请求失败: {url}"))?;

        let status = resp.status().as_u16();
        let final_url = resp.url().to_string();
        validate_public_https_url(&final_url)?;

        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = resp
            .bytes()
            .await
            .with_context(|| format!("读取响应体失败: {url}"))?;
        if bytes.len() > MAX_BODY_BYTES {
            bail!(
                "响应体过大（{} 字节，上限 {} 字节）",
                bytes.len(),
                MAX_BODY_BYTES
            );
        }

        let body = String::from_utf8_lossy(&bytes).into_owned();
        let truncated = body.chars().count() > max_chars;
        let body = truncate_output(&body, max_chars);

        let result = FetchResult {
            url: url.to_string(),
            final_url,
            status,
            content_type,
            body,
            truncated,
        };
        Ok(serde_json::to_string_pretty(&result)?)
    }
}

fn validate_public_https_url(url: &str) -> Result<()> {
    crate::fs_util::ensure_https_url(url)?;
    let parsed = url
        .parse::<reqwest::Url>()
        .with_context(|| format!("无效 URL: {url}"))?;

    if parsed.username() != "" || parsed.password().is_some() {
        bail!("URL 不允许包含用户名或密码");
    }

    let host = parsed
        .host_str()
        .with_context(|| format!("URL 缺少主机名: {url}"))?;

    if is_blocked_host(host) {
        bail!("禁止访问内网或本地地址: {host}");
    }

    Ok(())
}

fn is_blocked_host(host: &str) -> bool {
    let lower = host.to_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") || lower.ends_with(".local") {
        return true;
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        return is_private_ip(ip);
    }

    if let Ok(ip) = lower.parse::<IpAddr>() {
        return is_private_ip(ip);
    }

    false
}

fn is_private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_private_ipv4(v4),
        IpAddr::V6(v6) => is_private_ipv6(v6),
    }
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_unspecified()
        || ip.is_broadcast()
        || ip.octets()[0] == 169 && ip.octets()[1] == 254
}

fn is_private_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_http_and_private_hosts() {
        assert!(validate_public_https_url("http://example.com").is_err());
        assert!(validate_public_https_url("https://localhost/").is_err());
        assert!(validate_public_https_url("https://127.0.0.1/").is_err());
        assert!(validate_public_https_url("https://10.0.0.1/").is_err());
        assert!(validate_public_https_url("https://192.168.1.1/").is_err());
        assert!(validate_public_https_url("https://www.google.com").is_ok());
    }
}
