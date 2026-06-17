use serde_json::Value;

use super::patterns::{normalize_path_for_match, path_pattern_match};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoDecision {
    Allow,
    Deny,
    Ask,
}

pub fn auto_classify(tool: &str, args: &Value, detail: &str, workdir: &std::path::Path) -> AutoDecision {
    let tool_lc = tool.to_lowercase();

    if matches!(tool_lc.as_str(), "read" | "grep" | "glob") {
        if is_sensitive_path_arg(args, workdir) {
            return AutoDecision::Deny;
        }
        return AutoDecision::Allow;
    }

    if tool_lc == "bash" {
        let cmd = detail.to_lowercase();
        if cmd.contains("sudo ")
            || cmd.contains("curl ")
            || cmd.contains("wget ")
            || cmd.contains("| sh")
            || cmd.contains("| bash")
            || cmd.contains("chmod -r")
            || cmd.contains("chmod -R")
        {
            return AutoDecision::Ask;
        }
        if cmd.starts_with("cargo test")
            || cmd.starts_with("cargo build")
            || cmd.starts_with("cargo check")
            || cmd.starts_with("git status")
            || cmd.starts_with("git diff")
            || cmd.starts_with("git log")
            || cmd.starts_with("npm run build")
            || cmd.starts_with("npm test")
            || cmd.starts_with("npm run test")
        {
            if !cmd.contains("&& rm") && !cmd.contains("; rm") {
                return AutoDecision::Allow;
            }
        }
        return AutoDecision::Ask;
    }

    if matches!(tool_lc.as_str(), "write" | "edit") {
        if let Some(path) = args["path"].as_str() {
            if is_sensitive_path(path, workdir) {
                return AutoDecision::Deny;
            }
            if path_pattern_match(".git/**", path, workdir)
                || path_pattern_match(".onemini/**", path, workdir)
            {
                return AutoDecision::Deny;
            }
            return AutoDecision::Allow;
        }
        return AutoDecision::Ask;
    }

    if tool_lc.starts_with("mcp_") {
        return AutoDecision::Ask;
    }

    AutoDecision::Ask
}

fn is_sensitive_path_arg(args: &Value, workdir: &std::path::Path) -> bool {
    if let Some(path) = args["path"].as_str() {
        return is_sensitive_path(path, workdir);
    }
    if let Some(pattern) = args["pattern"].as_str() {
        let p = pattern.to_lowercase();
        if p.contains(".env") || p.contains("id_rsa") || p.contains("credentials") {
            return true;
        }
    }
    false
}

fn is_sensitive_path(path: &str, workdir: &std::path::Path) -> bool {
    let norm = normalize_path_for_match(path, workdir).to_lowercase();
    norm.contains(".env")
        || norm.contains("id_rsa")
        || norm.ends_with(".pem")
        || norm.contains("credentials.json")
}
