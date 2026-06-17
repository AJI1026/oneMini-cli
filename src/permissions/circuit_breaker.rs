#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitBreaker {
    /// 硬拒绝，无视 bypass
    HardDeny,
    /// 强制询问，无视 bypass/auto
    ForceAsk,
}

pub fn check_circuit_breaker(tool: &str, detail: &str) -> Option<CircuitBreaker> {
    if tool == "bash" {
        let cmd = detail.to_lowercase();
        let compact = cmd.split_whitespace().collect::<Vec<_>>().join(" ");
        if compact.contains("rm -rf /")
            || compact.contains("rm -rf ~")
            || compact.contains("rm -rf $home")
            || compact.contains("git push --force")
            || compact.contains("git push -f ")
            || compact.contains("drop table")
        {
            return Some(CircuitBreaker::HardDeny);
        }
        if compact.contains("rm -rf .")
            || compact.contains("rm -rf $pwd")
            || compact == "rm -rf"
        {
            return Some(CircuitBreaker::ForceAsk);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_deny_rm_root() {
        assert_eq!(
            check_circuit_breaker("bash", "rm -rf /"),
            Some(CircuitBreaker::HardDeny)
        );
    }
}
