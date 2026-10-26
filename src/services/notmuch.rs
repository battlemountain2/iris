#![allow(dead_code)]

// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use std::process::Command;

/// Checks if the `notmuch` CLI utility is installed and functional on the system.
pub fn is_notmuch_available() -> bool {
    Command::new("notmuch")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Executes a `notmuch search` query returning matching thread IDs (e.g. `thread:0000000000000001`).
pub fn search_thread_ids(query: &str) -> Result<Vec<String>, String> {
    if !is_notmuch_available() {
        return Err("notmuch CLI is not installed in system PATH".to_string());
    }

    let output = Command::new("notmuch")
        .arg("search")
        .arg("--output=threads")
        .arg(query)
        .output()
        .map_err(|e| format!("Failed to execute notmuch search: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("notmuch search error: {}", err.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let threads = stdout
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    Ok(threads)
}

/// Executes a `notmuch search` query returning paths of matching message files.
pub fn search_message_files(query: &str) -> Result<Vec<PathBuf>, String> {
    if !is_notmuch_available() {
        return Err("notmuch CLI is not installed in system PATH".to_string());
    }

    let output = Command::new("notmuch")
        .arg("search")
        .arg("--output=files")
        .arg(query)
        .output()
        .map_err(|e| format!("Failed to execute notmuch search: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("notmuch search error: {}", err.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let files = stdout
        .lines()
        .map(|l| PathBuf::from(l.trim()))
        .filter(|p| p.as_os_str().len() > 0)
        .collect();

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notmuch_detection() {
        // Just verify it returns a boolean without panicking
        let _ = is_notmuch_available();
    }
}
