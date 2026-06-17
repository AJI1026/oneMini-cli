use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// 写入敏感文件并设置私有权限（Unix 0o600）。
pub fn write_private(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents.as_ref())
        .with_context(|| format!("写入文件失败: {}", path.display()))?;
    set_private_permissions(path)?;
    Ok(())
}

pub fn set_private_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// 校验 API base URL 必须为 HTTPS。
pub fn ensure_https_url(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") {
        anyhow::bail!("API 接口地址必须使用 HTTPS: {trimmed}");
    }
    Ok(())
}
