//! CLI detection and path resolution for the Grok Build binary.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Minimum CLI version string we expect for M0 (informational; not strictly semver-gated yet).
pub const MIN_CLI_VERSION_HINT: &str = "0.2.0";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub error: Option<String>,
    pub install_hint: String,
}

impl CliStatus {
    fn missing(error: impl Into<String>) -> Self {
        Self {
            installed: false,
            path: None,
            version: None,
            error: Some(error.into()),
            install_hint: default_install_hint(),
        }
    }

    fn found(path: PathBuf, version: Option<String>) -> Self {
        Self {
            installed: true,
            path: Some(path.to_string_lossy().into_owned()),
            version,
            error: None,
            install_hint: default_install_hint(),
        }
    }
}

fn default_install_hint() -> String {
    "请安装 Grok Build CLI：访问 https://x.ai 获取安装方式，或在终端运行官方安装脚本后将 `grok` 加入 PATH。安装完成后点击「重新检测」。"
        .into()
}

/// Candidate paths for the `grok` binary (Windows + Unix).
fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(path_env) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_env) {
            paths.push(dir.join(grok_bin_name()));
        }
    }

    if let Some(home) = dirs_home() {
        paths.push(home.join(".grok").join("bin").join(grok_bin_name()));
        #[cfg(windows)]
        {
            paths.push(
                home.join("AppData")
                    .join("Local")
                    .join("grok")
                    .join("bin")
                    .join(grok_bin_name()),
            );
        }
        #[cfg(not(windows))]
        {
            paths.push(home.join(".local").join("bin").join(grok_bin_name()));
        }
    }

    paths
}

fn grok_bin_name() -> &'static str {
    if cfg!(windows) {
        "grok.exe"
    } else {
        "grok"
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// Resolve the first existing `grok` binary.
pub fn resolve_grok_path() -> Option<PathBuf> {
    // Prefer `which`-style via Command which on Windows finds .exe in PATH.
    if let Ok(output) = Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg("grok")
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().next() {
                let p = PathBuf::from(line.trim());
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }

    candidate_paths().into_iter().find(|p| p.is_file())
}

/// Run `grok version` and parse a short version string.
pub fn read_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("version").output().ok()?;
    if !output.status.success() {
        // Some builds may only print to stderr or use --version
        let output = Command::new(path).arg("--version").output().ok()?;
        if !output.status.success() {
            return None;
        }
        return first_nonempty_line(&output.stdout).or_else(|| first_nonempty_line(&output.stderr));
    }
    first_nonempty_line(&output.stdout).or_else(|| first_nonempty_line(&output.stderr))
}

fn first_nonempty_line(bytes: &[u8]) -> Option<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(str::to_string)
}

/// Detect whether Grok CLI is available and report version.
pub fn detect_cli() -> CliStatus {
    match resolve_grok_path() {
        Some(path) => {
            let version = read_version(&path);
            CliStatus::found(path, version)
        }
        None => CliStatus::missing(
            "未在 PATH 或常见安装路径中找到 `grok`。请先安装 Grok Build CLI。",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_status_has_install_hint() {
        let s = CliStatus::missing("nope");
        assert!(!s.installed);
        assert!(s.install_hint.contains("Grok"));
    }
}
