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

pub struct UninstallOptions {
    pub install_dir: Option<PathBuf>,
    pub keep_path: bool,
    pub purge: bool,
    pub yes: bool,
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

pub fn run_uninstall(opts: UninstallOptions) -> Result<()> {
    let install_dir = opts
        .install_dir
        .or_else(default_install_dir)
        .context("无法确定安装目录（可设置 ONEMINI_INSTALL_DIR 或 --dir）")?;

    let binary = binary_path(&install_dir);
    let mut actions = Vec::new();
    if binary.is_file() {
        actions.push(format!("删除二进制: {}", binary.display()));
    } else {
        actions.push(format!("二进制不存在（跳过）: {}", binary.display()));
    }
    if !opts.keep_path {
        actions.push(format!("移除 PATH 配置（{}）", install_dir.display()));
    }
    if opts.purge {
        if let Some(dir) = onemini_config_dir() {
            actions.push(format!("删除配置目录: {}", dir.display()));
        }
        if let Some(dir) = onemini_data_dir() {
            actions.push(format!("删除数据目录: {}", dir.display()));
        }
    }

    if !opts.yes {
        println!("{}", crate::ui::warn("即将卸载 OneMini："));
        for line in &actions {
            println!("  - {line}");
        }
        if !confirm_uninstall()? {
            println!("{}", crate::ui::dim("已取消卸载"));
            return Ok(());
        }
    }

    let mut removed_any = false;

    if binary.is_file() {
        if is_current_binary(&binary)? {
            println!(
                "{}",
                crate::ui::warn(&format!(
                    "当前正在运行 {}，卸载后本进程仍可用；请在新终端验证 onemini 已不可用",
                    binary.display()
                ))
            );
        }
        match fs::remove_file(&binary) {
            Ok(()) => {
                println!(
                    "{}",
                    crate::ui::success(&format!("已删除 {}", binary.display()))
                );
                removed_any = true;
            }
            Err(e) => {
                println!(
                    "{}",
                    crate::ui::warn(&format!("无法删除 {}: {e}", binary.display()))
                );
            }
        }
    }

    #[cfg(windows)]
    {
        let staged = binary.with_extension("exe.new");
        if staged.is_file() {
            let _ = fs::remove_file(&staged);
        }
    }

    if !opts.keep_path {
        if remove_path_config(&install_dir)? {
            removed_any = true;
        }
    }

    if opts.purge {
        if purge_user_data()? {
            removed_any = true;
        }
    }

    if removed_any {
        println!("{}", crate::ui::success("卸载完成"));
    } else {
        println!(
            "{}",
            crate::ui::warn("未找到可卸载的 OneMini 安装项（可能已通过其他方式安装）")
        );
    }
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

fn is_current_binary(path: &Path) -> Result<bool> {
    let current = std::env::current_exe().context("无法定位当前可执行文件")?;
    same_binary_location(&current, path)
}

fn confirm_uninstall() -> Result<bool> {
    if !stdin().is_terminal() {
        anyhow::bail!("非交互环境请添加 --yes 确认卸载");
    }
    print!("确认卸载？[y/N] ");
    std::io::stdout().flush()?;
    let mut line = String::new();
    stdin().read_line(&mut line)?;
    let answer = line.trim().to_ascii_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

fn onemini_config_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("onemini"))
}

fn onemini_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("onemini"))
}

fn remove_path_config(install_dir: &Path) -> Result<bool> {
    #[cfg(windows)]
    {
        return remove_windows_user_path(install_dir);
    }
    #[cfg(not(windows))]
    {
        remove_unix_shell_path(install_dir)
    }
}

#[cfg(not(windows))]
fn remove_unix_shell_path(install_dir: &Path) -> Result<bool> {
    let profile = detect_shell_profile()?;
    if !profile.is_file() {
        return Ok(false);
    }

    let existing = fs::read_to_string(&profile)
        .with_context(|| format!("无法读取 {}", profile.display()))?;
    let Some(updated) = strip_onemini_path_block(&existing) else {
        println!(
            "{}",
            crate::ui::dim(&format!(
                "未在 {} 中找到 onemini PATH 配置块",
                profile.display()
            ))
        );
        return Ok(false);
    };

    fs::write(&profile, updated)
        .with_context(|| format!("无法写入 {}", profile.display()))?;
    println!(
        "{}",
        crate::ui::success(&format!(
            "已从 {} 移除 onemini PATH 配置",
            profile.display()
        ))
    );
    println!(
        "{}",
        crate::ui::warn(&format!("请重新打开终端，或运行: source {}", profile.display()))
    );
    let _ = install_dir;
    Ok(true)
}

#[cfg(windows)]
fn remove_windows_user_path(install_dir: &Path) -> Result<bool> {
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
    let entries: Vec<&str> = user_path
        .split(';')
        .filter(|entry| !entry.is_empty())
        .collect();
    let filtered: Vec<&str> = entries
        .iter()
        .copied()
        .filter(|entry| entry.trim_end_matches(['\\', '/']) != normalized)
        .collect();

    if filtered.len() == entries.len() {
        println!(
            "{}",
            crate::ui::dim(&format!("用户 PATH 中未找到 {normalized}"))
        );
        return Ok(false);
    }

    let new_path = filtered.join(";");
    env.set_value("Path", &new_path)
        .context("无法写入用户 PATH")?;
    println!(
        "{}",
        crate::ui::success(&format!("已从用户 PATH 移除 {normalized}"))
    );
    Ok(true)
}

fn strip_onemini_path_block(content: &str) -> Option<String> {
    let begin = content.find(PATH_MARKER_BEGIN)?;
    let end = content.find(PATH_MARKER_END)?;
    if end < begin {
        return None;
    }
    let after_end = end + PATH_MARKER_END.len();
    let tail_start = content[after_end..]
        .strip_prefix("\r\n")
        .or_else(|| content[after_end..].strip_prefix('\n'))
        .map(|rest| after_end + (content[after_end..].len() - rest.len()))
        .unwrap_or(after_end);

    let mut head = content[..begin].to_string();
    while head.ends_with('\n') || head.ends_with('\r') {
        head.pop();
    }
    let tail = &content[tail_start..];
    let mut out = head;
    if !out.is_empty() && !tail.is_empty() {
        out.push('\n');
    }
    out.push_str(tail);
    Some(out)
}

fn purge_user_data() -> Result<bool> {
    let mut removed = false;
    for dir in [onemini_config_dir(), onemini_data_dir()] {
        let Some(dir) = dir else { continue };
        if !dir.exists() {
            continue;
        }
        fs::remove_dir_all(&dir)
            .with_context(|| format!("无法删除 {}", dir.display()))?;
        println!(
            "{}",
            crate::ui::success(&format!("已删除 {}", dir.display()))
        );
        removed = true;
    }
    Ok(removed)
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

    #[test]
    fn strip_path_block_removes_markers() {
        let input = "export FOO=1\n\n# >>> onemini >>>\nexport PATH=\"/home/u/.local/bin:$PATH\"\n# <<< onemini <<<\n\nalias ll='ls -l'\n";
        let out = strip_onemini_path_block(input).expect("block");
        assert!(!out.contains(PATH_MARKER_BEGIN));
        assert!(!out.contains(PATH_MARKER_END));
        assert!(out.contains("export FOO=1"));
        assert!(out.contains("alias ll"));
    }

    #[test]
    fn strip_path_block_returns_none_without_markers() {
        assert!(strip_onemini_path_block("export PATH=1").is_none());
    }
}
