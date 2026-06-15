//! 签名版本索引 `versions.json` 解析。

use anyhow::{bail, Context, Result};
use semver::Version;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct VersionsIndex {
    pub schema_version: u32,
    pub latest: String,
    pub releases: BTreeMap<String, ReleaseEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseEntry {
    pub tag: String,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub deprecation_reason: Option<String>,
    pub assets: BTreeMap<String, AssetEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetEntry {
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub sig_url: Option<String>,
}

impl VersionsIndex {
    pub fn parse(json: &[u8]) -> Result<Self> {
        let index: Self = serde_json::from_slice(json).context("解析 versions.json 失败")?;
        if index.schema_version != 1 {
            bail!(
                "不支持的 versions.json schema_version: {}（仅支持 1）",
                index.schema_version
            );
        }
        if index.releases.is_empty() {
            bail!("versions.json 未包含任何 release");
        }
        Ok(index)
    }

    pub fn resolve_version(&self, requested: Option<&str>) -> Result<(String, &ReleaseEntry)> {
        match requested {
            Some(v) => {
                let key = normalize_version_key(v)?;
                let entry = self
                    .releases
                    .get(&key)
                    .with_context(|| format!("versions.json 中不存在版本: {key}"))?;
                Ok((key, entry))
            }
            None => {
                let entry = self
                    .releases
                    .get(&self.latest)
                    .with_context(|| format!("versions.json 中不存在 latest 版本: {}", self.latest))?;
                Ok((self.latest.clone(), entry))
            }
        }
    }

    pub fn asset_for_platform<'a>(
        entry: &'a ReleaseEntry,
        platform: &str,
    ) -> Result<&'a AssetEntry> {
        entry
            .assets
            .get(platform)
            .with_context(|| format!("该版本未提供平台 {platform} 的下载包"))
    }
}

pub fn normalize_version_key(v: &str) -> Result<String> {
    let trimmed = v.trim().trim_start_matches('v');
    Version::parse(trimmed).with_context(|| format!("无效版本号: {v}"))?;
    Ok(trimmed.to_string())
}

pub fn asset_sig_url(asset: &AssetEntry) -> String {
    asset
        .sig_url
        .clone()
        .unwrap_or_else(|| format!("{}.sig", asset.url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_index() {
        let json = r#"{
            "schema_version": 1,
            "latest": "0.1.0",
            "releases": {
                "0.1.0": {
                    "tag": "v0.1.0",
                    "deprecated": false,
                    "assets": {
                        "x86_64-apple-darwin": {
                            "url": "https://github.com/example/onemini-x86_64-apple-darwin.tar.gz",
                            "sha256": "abc"
                        }
                    }
                }
            }
        }"#;
        let index = VersionsIndex::parse(json.as_bytes()).unwrap();
        assert_eq!(index.latest, "0.1.0");
        let (_, entry) = index.resolve_version(None).unwrap();
        assert_eq!(entry.tag, "v0.1.0");
    }
}
