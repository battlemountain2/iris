// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mail_parser::MimeHeaders;

/// Sanitizes an attachment filename to prevent directory traversal or filesystem exploits.
pub fn sanitize_filename(raw: &str) -> String {
    // 1. Strip all directory components (both Unix / and Windows \)
    let trimmed = raw.trim();
    let basename = Path::new(trimmed)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(trimmed);

    // Also handle Windows backslashes if on non-Windows platforms
    let clean_basename = basename
        .rsplit('\\')
        .next()
        .unwrap_or(basename)
        .trim();

    // 2. Remove null bytes and control characters
    let mut safe_name: String = clean_basename
        .chars()
        .filter(|&c| c != '\0' && !c.is_control())
        .collect();

    // 3. Prevent hidden files starting with '.' from untrusted senders
    while safe_name.starts_with('.') {
        safe_name.remove(0);
    }

    let safe_name = safe_name.trim();

    // 4. Fallback if empty or sanitized away
    if safe_name.is_empty() {
        "attachment.bin".to_string()
    } else {
        // Truncate to maximum 200 characters to respect filesystem limits
        if safe_name.len() > 200 {
            safe_name[..200].to_string()
        } else {
            safe_name.to_string()
        }
    }
}

/// Resolves the default destination directory for downloads (Hermes Downloads folder).
pub fn get_hermes_downloads_dir() -> PathBuf {
    if let Some(custom) = std::env::var_os("IRIS_DOWNLOADS_DIR") {
        let path = PathBuf::from(custom);
        let _ = fs::create_dir_all(&path);
        return path;
    }

    if let Ok(home) = std::env::var("HOME") {
        let hermes_downloads = PathBuf::from(home.clone()).join("Downloads").join("Hermes");
        if fs::create_dir_all(&hermes_downloads).is_ok() {
            return hermes_downloads;
        }
        let standard_downloads = PathBuf::from(home).join("Downloads");
        if fs::create_dir_all(&standard_downloads).is_ok() {
            return standard_downloads;
        }
    }

    let fallback = std::env::temp_dir().join("iris_downloads");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// Resolves a collision-safe file path in the target directory (e.g. `report (1).pdf`).
pub fn get_unique_destination(dir: &Path, raw_filename: &str) -> PathBuf {
    let clean = sanitize_filename(raw_filename);
    let initial_path = dir.join(&clean);
    if !initial_path.exists() {
        return initial_path;
    }

    let stem = Path::new(&clean)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("attachment");

    let extension = Path::new(&clean)
        .extension()
        .and_then(|e| e.to_str());

    let mut counter = 1;
    loop {
        let candidate_name = match extension {
            Some(ext) => format!("{stem} ({counter}).{ext}"),
            None => format!("{stem} ({counter})"),
        };
        let candidate_path = dir.join(candidate_name);
        if !candidate_path.exists() {
            return candidate_path;
        }
        counter += 1;
    }
}

/// Saves raw attachment bytes to a collision-safe path inside the destination folder.
pub fn save_attachment_data(dest_dir: &Path, filename: &str, data: &[u8]) -> Result<PathBuf, String> {
    let _ = fs::create_dir_all(dest_dir);
    let target_path = get_unique_destination(dest_dir, filename);
    fs::write(&target_path, data).map_err(|e| format!("Failed to write attachment: {e}"))?;
    Ok(target_path)
}

/// Parses an RFC 822 email file from disk, extracts the attachment at `attachment_index`,
/// and writes it to disk.
pub fn extract_and_save_attachment(
    message_file_path: &Path,
    attachment_index: usize,
    custom_dest_dir: Option<&Path>,
) -> Result<(PathBuf, String), String> {
    let raw_bytes = fs::read(message_file_path)
        .map_err(|e| format!("Failed to read message file {}: {e}", message_file_path.display()))?;

    let parsed = mail_parser::MessageParser::default()
        .parse(&raw_bytes)
        .ok_or_else(|| "Failed to parse MIME message".to_string())?;

    let attachment_part = parsed
        .attachments()
        .nth(attachment_index)
        .ok_or_else(|| format!("Attachment index {attachment_index} not found in message"))?;

    let filename = attachment_part
        .attachment_name()
        .unwrap_or("attachment")
        .to_string();

    let data = attachment_part.contents();

    let target_dir = match custom_dest_dir {
        Some(d) => d.to_path_buf(),
        None => get_hermes_downloads_dir(),
    };

    let saved_path = save_attachment_data(&target_dir, &filename, data)?;
    Ok((saved_path, filename))
}

/// Dispatches an asynchronous desktop notification informing the user of the saved attachment.
pub fn notify_attachment_saved(filename: &str, saved_path: &Path) {
    let path_str = saved_path.display().to_string();
    let body = format!("Saved {filename} to {path_str}");
    let _ = Command::new("notify-send")
        .args([
            "--app-name=Iris",
            "--icon=document-save-symbolic",
            "Attachment Saved",
            &body,
        ])
        .spawn();
}

/// Opens the containing folder or file in Hermes/Strata if available, or system file manager.
pub fn open_in_hermes(saved_path: &Path) -> Result<(), String> {
    let target = if saved_path.is_file() {
        saved_path.parent().unwrap_or(saved_path)
    } else {
        saved_path
    };

    let target_str = target.to_string_lossy().to_string();

    // Check if hermes or strata is in PATH
    let launchers = ["hermes", "strata", "dolphin", "nautilus", "xdg-open"];
    for launcher in launchers {
        if Command::new(launcher).arg(&target_str).spawn().is_ok() {
            return Ok(());
        }
    }

    Err("No compatible file manager launcher found".to_string())
}

/// Guesses the MIME content type for an outgoing attachment based on file extension.
pub fn guess_content_type(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".flac") {
        "audio/flac"
    } else if lower.ends_with(".wav") {
        "audio/wav"
    } else if lower.ends_with(".mp3") {
        "audio/mpeg"
    } else if lower.ends_with(".ogg") {
        "audio/ogg"
    } else if lower.ends_with(".txt") || lower.ends_with(".md") || lower.ends_with(".rs") || lower.ends_with(".toml") {
        "text/plain; charset=utf-8"
    } else if lower.ends_with(".html") || lower.ends_with(".htm") {
        "text/html; charset=utf-8"
    } else if lower.ends_with(".zip") {
        "application/zip"
    } else if lower.ends_with(".tar.gz") || lower.ends_with(".tgz") {
        "application/gzip"
    } else if lower.ends_with(".json") {
        "application/json"
    } else {
        "application/octet-stream"
    }
}

/// Formats byte counts into human-readable strings (e.g. 1.2 MB, 345 KB).
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename_traversal() {
        assert_eq!(sanitize_filename("../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("..\\..\\Windows\\System32\\calc.exe"), "calc.exe");
        assert_eq!(sanitize_filename("/var/log/syslog"), "syslog");
        assert_eq!(sanitize_filename("normal_file.pdf"), "normal_file.pdf");
    }

    #[test]
    fn test_sanitize_filename_empty_and_hidden() {
        assert_eq!(sanitize_filename(""), "attachment.bin");
        assert_eq!(sanitize_filename("   "), "attachment.bin");
        assert_eq!(sanitize_filename("..."), "attachment.bin");
        assert_eq!(sanitize_filename(".bashrc"), "bashrc");
    }

    #[test]
    fn test_unique_destination() {
        let temp_dir = std::env::temp_dir().join("iris_test_unique_dest");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let p1 = get_unique_destination(&temp_dir, "song.mp3");
        assert_eq!(p1.file_name().unwrap(), "song.mp3");
        fs::write(&p1, b"first").unwrap();

        let p2 = get_unique_destination(&temp_dir, "song.mp3");
        assert_eq!(p2.file_name().unwrap(), "song (1).mp3");
        fs::write(&p2, b"second").unwrap();

        let p3 = get_unique_destination(&temp_dir, "song.mp3");
        assert_eq!(p3.file_name().unwrap(), "song (2).mp3");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_extract_and_save_mime_attachment() {
        let temp_dir = std::env::temp_dir().join("iris_test_mime_att");
        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::create_dir_all(&temp_dir);

        let email_raw = b"From: Alex <alex@hermes.dev>\r\n\
To: Brayan <bry@battlemountain.dev>\r\n\
Subject: Test with Attachment\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"BOUNDARY\"\r\n\
\r\n\
--BOUNDARY\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Hello with an attachment.\r\n\
--BOUNDARY\r\n\
Content-Type: text/plain; name=\"note.txt\"\r\n\
Content-Disposition: attachment; filename=\"note.txt\"\r\n\
Content-Transfer-Encoding: 7bit\r\n\
\r\n\
Attachment payload data 12345\r\n\
--BOUNDARY--\r\n";

        let mail_path = temp_dir.join("message.eml");
        fs::write(&mail_path, email_raw).unwrap();

        let out_dir = temp_dir.join("Hermes");
        let (saved_path, filename) = extract_and_save_attachment(&mail_path, 0, Some(&out_dir)).unwrap();

        assert_eq!(filename, "note.txt");
        assert!(saved_path.exists());
        let saved_content = fs::read_to_string(&saved_path).unwrap();
        assert!(saved_content.contains("Attachment payload data 12345"));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("document.pdf"), "application/pdf");
        assert_eq!(guess_content_type("photo.PNG"), "image/png");
        assert_eq!(guess_content_type("picture.jpeg"), "image/jpeg");
        assert_eq!(guess_content_type("audio.flac"), "audio/flac");
        assert_eq!(guess_content_type("notes.txt"), "text/plain; charset=utf-8");
        assert_eq!(guess_content_type("script.rs"), "text/plain; charset=utf-8");
        assert_eq!(guess_content_type("archive.tar.gz"), "application/gzip");
        assert_eq!(guess_content_type("data.json"), "application/json");
        assert_eq!(guess_content_type("unknown.xyz"), "application/octet-stream");
    }

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(2048), "2.0 KB");
        assert_eq!(format_file_size(1024 * 1024 * 3), "3.0 MB");
        assert_eq!(format_file_size(1024 * 1024 * 1024 * 2), "2.0 GB");
    }
}
