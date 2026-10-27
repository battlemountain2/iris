// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TagsData {
    pub thread_tags: HashMap<String, Vec<String>>,
}

static TAGS_STATE: OnceLock<Mutex<TagsData>> = OnceLock::new();

fn get_tags_file_path() -> PathBuf {
    if let Ok(dir) = std::env::var("IRIS_TAGS_DIR") {
        PathBuf::from(dir).join("tags.json")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".config").join("iris").join("tags.json")
    } else {
        PathBuf::from("/tmp/iris-tags.json")
    }
}

fn get_state() -> &'static Mutex<TagsData> {
    TAGS_STATE.get_or_init(|| {
        let path = get_tags_file_path();
        let data = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<TagsData>(&s).ok())
                .unwrap_or_default()
        } else {
            TagsData::default()
        };
        Mutex::new(data)
    })
}

pub fn save_tags_state() -> Result<(), String> {
    let path = get_tags_file_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let state = get_state().lock().unwrap();
    let json = serde_json::to_string_pretty(&*state)
        .map_err(|e| format!("Serialization error: {e}"))?;
    fs::write(&path, json).map_err(|e| format!("Save error: {e}"))
}

pub fn get_tags_for_thread(thread_id: &str) -> Vec<String> {
    let state = get_state().lock().unwrap();
    state.thread_tags.get(thread_id).cloned().unwrap_or_default()
}

pub fn add_tag_to_thread(thread_id: &str, tag: &str) -> Vec<String> {
    let clean = tag.trim().to_string();
    if clean.is_empty() {
        return get_tags_for_thread(thread_id);
    }

    let mut state = get_state().lock().unwrap();
    let entry = state.thread_tags.entry(thread_id.to_string()).or_default();
    if !entry.iter().any(|t| t.eq_ignore_ascii_case(&clean)) {
        entry.push(clean);
        entry.sort();
    }
    let res = entry.clone();
    drop(state);
    let _ = save_tags_state();
    res
}

pub fn remove_tag_from_thread(thread_id: &str, tag: &str) -> Vec<String> {
    let mut state = get_state().lock().unwrap();
    let res = if let Some(entry) = state.thread_tags.get_mut(thread_id) {
        entry.retain(|t| !t.eq_ignore_ascii_case(tag));
        entry.clone()
    } else {
        Vec::new()
    };
    drop(state);
    let _ = save_tags_state();
    res
}

pub fn toggle_tag_for_thread(thread_id: &str, tag: &str) -> (bool, Vec<String>) {
    let current = get_tags_for_thread(thread_id);
    if current.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
        (false, remove_tag_from_thread(thread_id, tag))
    } else {
        (true, add_tag_to_thread(thread_id, tag))
    }
}

pub fn list_all_tags() -> Vec<String> {
    let state = get_state().lock().unwrap();
    let mut all: Vec<String> = state
        .thread_tags
        .values()
        .flat_map(|v| v.iter().cloned())
        .collect();
    all.sort();
    all.dedup();
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_remove_tag() {
        let t_id = "test_thread_123";
        let tags = add_tag_to_thread(t_id, "Project-Alpha");
        assert!(tags.contains(&"Project-Alpha".to_string()));

        let tags2 = add_tag_to_thread(t_id, "Urgent");
        assert_eq!(tags2.len(), 2);
        assert!(tags2.contains(&"Urgent".to_string()));

        let tags3 = remove_tag_from_thread(t_id, "Project-Alpha");
        assert_eq!(tags3.len(), 1);
        assert!(!tags3.contains(&"Project-Alpha".to_string()));
        assert!(tags3.contains(&"Urgent".to_string()));

        let (added, tags4) = toggle_tag_for_thread(t_id, "Finance");
        assert!(added);
        assert!(tags4.contains(&"Finance".to_string()));

        let (added2, tags5) = toggle_tag_for_thread(t_id, "Finance");
        assert!(!added2);
        assert!(!tags5.contains(&"Finance".to_string()));

        let all = list_all_tags();
        assert!(all.contains(&"Urgent".to_string()));
    }
}
