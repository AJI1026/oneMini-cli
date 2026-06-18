use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const BINARY: &str = "onemini";

pub struct InstallOptions {
    pub install_dir: Option<PathBuf>,
    pub skip_path: bool,
}

pub fn run(opts: InstallOptions) -> Result<()> {
    let source = std::env::current_exe().context("无法定位当前可执行文件")?;
    let install_dir = opts
        .install_dir
        .or_else(default_install_dir)
        .context("无法确定安装目录（可设置 ONEMINI_INSTALL_DIR）")?;

    std::fs::create_dir_all(&install_dir)
        .with_context(|| format!("无法创建目录 {}", install_dir.display()))?;

    let dest = binary_path(&install_dir);

    if same_binary_location(&source, &dest)? {
        println!(
            "{}",
            crate::ui::success(&format!(
                "已安装在 {}",
                dest.display()
            ))
        );
        if !opts.skip_path && std::env::var("ONEMINI_SKIP_PATH").ok().as_deref() != Some("1") {
            ensure_path(&install_dir)?;
        }
        return Ok(());
    }

    copy_binary(&source, &dest)?;
    println!(
        "{}",
        crate::ui::success(&format!("已安装到 {}", dest.display()))
    );

    if opts.skip_path || std::env::var("ONEMINI_SKIP_PATH").ok().as_deref() == Some("1") {
        println!(
            "{}",
            crate::ui::warn("已跳过 PATH 配置（--skip-path 或 ONEMINI_SKIP_PATH=1）")
        );
    } else {
        ensure_path(&install_dir)?;
    }

    println!(
        "{}",
        crate::ui::dim("请重新打开终端，然后运行 onemini --help")
    );
    Ok(())
}

fn default_install_dir() -> Option<PathBuf> {
    std::env::var_os("ONEMINI_INSTALL_DIR")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".local").join("bin")))
}

fn binary_path(install_dir: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        install_dir.join(format!("{BINARY}.exe"))
    }
    #[cfg(not(windows))]
    {
        install_dir.join(BINARY)
    }
}

fn same_binary_location(source: &Path, dest: &Path) -> Result<bool> {
    let source_canon = source.canonicalize().unwrap_or_else(|_| source.to_path_buf());
    if let Ok(dest_canon) = dest.canonicalize() {
        return Ok(source_canon == dest_canon);
    }
    Ok(false)
}

fn copy_binary(source: &Path, dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    #[cfg(windows)]
    {
        let staged = dest.with_extension("exe.new");
        std::fs::copy(source, &staged)
            .with_context(|| format!("无法写入 {}", staged.display()))?;
        if dest.exists() {
            std::fs::remove_file(dest)
                .with_context(|| format!("无法替换 {}", dest.display()))?;
        }
        std::fs::rename(&staged, dest)
            .with_context(|| format!("无法安装到 {}", dest.display()))?;
    }

    #[cfg(not(windows))]
    {
        std::fs::copy(source, dest)
            .with_context(|| format!("无法复制到 {}", dest.display()))?;
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(dest, std::fs::Permissions::from_mode(0o755))?;
    }

    Ok(())
}

fn ensure_path(install_dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        ensure_windows_user_path(install_dir)
    }
    #[cfg(not(windows))]
    {
        println!(
            "{}",
            crate::ui::warn(&format!(
                "请确保 {} 在 PATH 中（可运行在线 install.sh 自动配置）",
                install_dir.display()
            ))
        );
        Ok(())
    }
}

#[cfg(windows)]
fn ensure_windows_user_path(install_dir: &Path) -> Result<()> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let normalized = install_dir
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
        .context("无法打开用户环境变量（HKCU\\Environment）")?;

    let user_path: String = env.get_value("Path").unwrap_or_default();
    let already_present = user_path
        .split(';')
        .filter(|entry| !entry.is_empty())
        .any(|entry| entry.trim_end_matches(['\\', '/']) == normalized);

    if already_present {
        println!(
            "{}",
            crate::ui::dim(&format!("{normalized} 已在用户 PATH 中"))
        );
        return Ok(());
    }

    let new_path = if user_path.trim().is_empty() {
        normalized.clone()
    } else {
        format!("{normalized};{user_path}")
    };

    env.set_value("Path", &new_path)
        .context("无法写入用户 PATH")?;
    println!(
        "{}",
        crate::ui::success(&format!("已将 {normalized} 加入用户 PATH"))
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_install_dir_under_home() {
        let dir = default_install_dir().expect("home dir");
        assert!(dir.ends_with(".local/bin") || dir.ends_with(".local\\bin"));
    }
}
