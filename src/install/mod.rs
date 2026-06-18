use anyhow::{Context, Result};
use std::fs;
use std::io::{stdin, IsTerminal, Write};
use std::path::{Path, PathBuf};

const BINARY: &str = "onemini";
const PATH_MARKER_BEGIN: &str = "# >>> onemini >>>";
const PATH_MARKER_END: &str = "# <<< onemini <<<";

pub struct InstallOptions {
    pub install_dir: Option<PathBuf>,
    pub skip_path: bool,
}

/// 从 Release 包双击 / 首次打开 .app 时自动进入安装流程。
pub fn should_auto_install() -> bool {
    if is_installed_at_destination() {
        return false;
    }
    if !is_release_bundle_location() {
        return false;
    }
    if std::env::args_os().len() != 1 {
        return false;
    }

    #[cfg(windows)]
    {
        return launched_from_gui();
    }

    #[cfg(target_os = "macos")]
    {
        let exe = std::env::current_exe().unwrap_or_default();
        return macos_app_resources_dir(&exe).is_some() || !stdin().is_terminal();
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        false
    }
}

#[cfg(windows)]
fn launched_from_gui() -> bool {
    use std::process::Command;

    let script = r#"
$p = (Get-CimInstance Win32_Process -Filter "ProcessId=$PID").ParentProcessId
if (-not $p) { exit 1 }
$parent = (Get-CimInstance Win32_Process -Filter "ProcessId=$p").Name
if ($parent -in @('explorer.exe', 'OpenWith.exe')) { exit 0 } else { exit 1 }
"#;
    Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map(|o| o.status.success())
        .unwrap_or_else(|_| !stdin().is_terminal())
}

pub fn run(opts: InstallOptions) -> Result<()> {
    let source = std::env::current_exe().context("无法定位当前可执行文件")?;

    if let Some(sig_path) = bundle_signature_path(&source) {
        verify_bundle_signature(&source, &sig_path)?;
    } else {
        println!(
            "{}",
            crate::ui::warn("未找到包内签名校验文件，跳过离线验签（开发构建或手动拷贝）")
        );
    }

    copy_bundled_skills(&source)?;

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
            crate::ui::success(&format!("已安装在 {}", dest.display()))
        );
    } else {
        copy_binary(&source, &dest)?;
        println!(
            "{}",
            crate::ui::success(&format!("已安装到 {}", dest.display()))
        );
    }

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
        crate::ui::dim("下一步：新开终端后运行 onemini config setup")
    );

    pause_or_notify_if_gui_launch(&install_dir);
    Ok(())
}

fn is_installed_at_destination() -> bool {
    let Ok(source) = std::env::current_exe() else {
        return false;
    };
    let Some(install_dir) = default_install_dir() else {
        return false;
    };
    let dest = binary_path(&install_dir);
    same_binary_location(&source, &dest).unwrap_or(false)
}

fn is_release_bundle_location() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    if macos_app_resources_dir(&exe).is_some() {
        return true;
    }
    if bundle_signature_path(&exe).is_some() {
        return true;
    }
    if let Some(parent) = exe.parent() {
        return parent.join("skills").is_dir();
    }
    false
}

fn bundle_signature_path(exe: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let sibling = exe.with_extension("exe.sig");
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    #[cfg(not(windows))]
    {
        let sibling = exe.with_extension("sig");
        if sibling.is_file() {
            return Some(sibling);
        }
    }
    if let Some(resources) = macos_app_resources_dir(exe) {
        let resource_sig = resources.join(format!("{BINARY}.sig"));
        if resource_sig.is_file() {
            return Some(resource_sig);
        }
    }
    None
}

fn verify_bundle_signature(binary: &Path, sig_path: &Path) -> Result<()> {
    let data = fs::read(binary)
        .with_context(|| format!("读取可执行文件失败: {}", binary.display()))?;
    let sig_text = fs::read_to_string(sig_path)
        .with_context(|| format!("读取签名失败: {}", sig_path.display()))?;
    crate::update::verify_signature(&data, &sig_text)?;
    println!("{}", crate::ui::success("包内签名校验通过"));
    Ok(())
}

fn bundled_skills_source(exe: &Path) -> Option<PathBuf> {
    if let Some(resources) = macos_app_resources_dir(exe) {
        let skills = resources.join("skills");
        if skills.is_dir() {
            return Some(skills);
        }
    }
    exe.parent().map(|p| p.join("skills")).filter(|p| p.is_dir())
}

fn copy_bundled_skills(exe: &Path) -> Result<()> {
    let Some(src) = bundled_skills_source(exe) else {
        return Ok(());
    };
    let dest = crate::skills::bootstrap::document_skills_data_dir();
    if crate::skills::bootstrap::document_skills_ready(&dest) {
        return Ok(());
    }
    fs::create_dir_all(&dest)?;
    copy_dir_recursive(&src, &dest)?;
    println!(
        "{}",
        crate::ui::success(&format!("已安装技能到 {}", dest.display()))
    );
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(src) {
        let entry = entry?;
        let rel = entry
            .path()
            .strip_prefix(src)
            .context("技能目录路径异常")?;
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &target)
                .with_context(|| format!("复制技能文件失败: {}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn macos_app_resources_dir(exe: &Path) -> Option<PathBuf> {
    let components: Vec<_> = exe.components().collect();
    for i in 0..components.len().saturating_sub(2) {
        if matches!(components.get(i), Some(c) if c.as_os_str() == ".app")
            && matches!(components.get(i + 1), Some(c) if c.as_os_str() == "Contents")
            && matches!(components.get(i + 2), Some(c) if c.as_os_str() == "MacOS")
        {
            let mut root = PathBuf::new();
            for c in &components[..=i + 1] {
                root.push(c);
            }
            return Some(root.join("Resources"));
        }
    }
    None
}

#[cfg(not(target_os = "macos"))]
pub fn macos_app_resources_dir(_exe: &Path) -> Option<PathBuf> {
    None
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
        ensure_unix_shell_path(install_dir)
    }
}

#[cfg(not(windows))]
fn ensure_unix_shell_path(install_dir: &Path) -> Result<()> {
    let install_dir = install_dir
        .to_string_lossy()
        .trim_end_matches('/')
        .to_string();

    if std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .any(|entry| entry == install_dir)
    {
        println!(
            "{}",
            crate::ui::dim(&format!("{install_dir} 已在 PATH 中"))
        );
        return Ok(());
    }

    let profile = detect_shell_profile()?;
    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent)?;
    }

    if profile.is_file() {
        let existing = fs::read_to_string(&profile).unwrap_or_default();
        if existing.contains(PATH_MARKER_BEGIN) {
            println!(
                "{}",
                crate::ui::dim(&format!(
                    "onemini PATH 配置块已存在于 {}",
                    profile.display()
                ))
            );
            return Ok(());
        }
    }

    let block = path_block_for_shell(&install_dir);
    if profile.is_file() {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&profile)
            .with_context(|| format!("无法写入 {}", profile.display()))?;
        writeln!(file, "\n{block}")?;
    } else {
        fs::write(&profile, format!("{block}\n"))
            .with_context(|| format!("无法写入 {}", profile.display()))?;
    }

    println!(
        "{}",
        crate::ui::success(&format!("已将 {install_dir} 写入 {}", profile.display()))
    );
    println!(
        "{}",
        crate::ui::warn(&format!("请重新打开终端，或运行: source {}", profile.display()))
    );
    Ok(())
}

#[cfg(not(windows))]
fn detect_shell_profile() -> Result<PathBuf> {
    let home = dirs::home_dir().context("无法定位用户主目录")?;
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = shell.rsplit('/').next().unwrap_or("");

    let profile = match shell_name {
        "zsh" => home.join(".zshrc"),
        "bash" => {
            if cfg!(target_os = "macos") && home.join(".bash_profile").is_file() {
                home.join(".bash_profile")
            } else {
                home.join(".bashrc")
            }
        }
        "fish" => home.join(".config").join("fish").join("config.fish"),
        _ => {
            if home.join(".profile").is_file() {
                home.join(".profile")
            } else if home.join(".bashrc").is_file() {
                home.join(".bashrc")
            } else {
                home.join(".profile")
            }
        }
    };
    Ok(profile)
}

#[cfg(not(windows))]
fn path_block_for_shell(install_dir: &str) -> String {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = shell.rsplit('/').next().unwrap_or("");
    if shell_name == "fish" {
        format!(
            "{PATH_MARKER_BEGIN}\nfish_add_path -a \"{install_dir}\"\n{PATH_MARKER_END}"
        )
    } else {
        format!(
            "{PATH_MARKER_BEGIN}\nexport PATH=\"{install_dir}:$PATH\"\n{PATH_MARKER_END}"
        )
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

fn pause_or_notify_if_gui_launch(install_dir: &Path) {
    if stdin().is_terminal() {
        return;
    }

    #[cfg(target_os = "macos")]
    {
        let msg = format!(
            "OneMini 已安装到 {}。\\n\\n请新开终端，运行：\\nonemini config setup",
            install_dir.display()
        );
        let script = format!(
            "display dialog \"{msg}\" with title \"OneMini\" buttons {{\"OK\"}} default button 1"
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status();
        return;
    }

    #[cfg(windows)]
    {
        eprintln!("\n按 Enter 关闭此窗口…");
        let _ = stdin().read_line(&mut String::new());
        return;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        eprintln!("\n按 Enter 关闭…");
        let _ = stdin().read_line(&mut String::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_install_dir_under_home() {
        let dir = default_install_dir().expect("home dir");
        assert!(dir.ends_with(".local/bin") || dir.ends_with(".local\\bin"));
    }

    #[test]
    fn auto_install_requires_no_tty_and_bundle() {
        // 单元测试环境通常有 TTY；仅验证未安装时不会 panic
        let _ = should_auto_install();
    }
}
