use glob::Pattern;
use std::path::Path;

/// 工具名 glob 匹配（支持 mcp_* 等）。
pub fn tool_name_matches(rule_tool: &str, actual: &str) -> bool {
    let rt = rule_tool.trim();
    if rt == "*" {
        return true;
    }
    if rt.ends_with('*') && !rt.contains('/') {
        let prefix = rt.trim_end_matches('*');
        return actual.starts_with(prefix);
    }
    // 大小写不敏感匹配内置工具
    rt.eq_ignore_ascii_case(actual)
        || normalize_tool_name(rt).eq_ignore_ascii_case(&normalize_tool_name(actual))
}

pub fn normalize_tool_name(name: &str) -> String {
    match name.to_lowercase().as_str() {
        "read" => "read".into(),
        "write" => "write".into(),
        "edit" => "edit".into(),
        "bash" => "bash".into(),
        "grep" => "grep".into(),
        "glob" => "glob".into(),
        "delegate" => "delegate".into(),
        other => other.to_string(),
    }
}

pub fn pattern_match(pattern: &str, text: &str) -> bool {
    let p = pattern.trim();
    if p.is_empty() {
        return false;
    }
    Pattern::new(p)
        .map(|pat| pat.matches(text))
        .unwrap_or_else(|_| text.contains(p))
}

/// 路径 glob 匹配（相对 workdir 的路径字符串）。
pub fn path_pattern_match(pattern: &str, path: &str, workdir: &Path) -> bool {
    let normalized = normalize_path_for_match(path, workdir);
    pattern_match(pattern, &normalized) || pattern_match(pattern, path)
}

pub fn normalize_path_for_match(path: &str, workdir: &Path) -> String {
    let p = Path::new(path.trim());
    if p.is_absolute() {
        if let Ok(rel) = p.strip_prefix(workdir) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    path.trim().trim_start_matches("./").replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_wildcard() {
        assert!(tool_name_matches("mcp_*", "mcp_github_list"));
    }

    #[test]
    fn bash_pattern() {
        assert!(pattern_match("cargo test*", "cargo test -p foo"));
    }
}
