//! Persistent key/value settings stored in `%APPDATA%/<identifier>/config.json`.
//!
//! All windows (main + widget) and the Rust side share one in-memory map guarded by a
//! mutex. Writes are *merges* (never whole-map overwrites from a possibly stale copy) and
//! are persisted atomically (temp file + rename) so a crash mid-write cannot corrupt the file.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub const KEY_MAIN_X: &str = "main_window_pos_x";
pub const KEY_MAIN_Y: &str = "main_window_pos_y";
pub const KEY_WIDGET_X: &str = "widget_pos_x";
pub const KEY_WIDGET_Y: &str = "widget_pos_y";

pub struct ConfigStore {
    path: Option<PathBuf>,
    map: Mutex<HashMap<String, String>>,
}

impl ConfigStore {
    /// Load the config from `path` (missing or invalid file -> empty config).
    pub fn load(path: Option<PathBuf>) -> Self {
        let map = path
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<HashMap<String, String>>(&s).ok())
            .unwrap_or_default();
        Self {
            path,
            map: Mutex::new(map),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, String>> {
        self.map.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        self.lock().clone()
    }

    pub fn get_i32(&self, key: &str) -> Option<i32> {
        self.lock().get(key).and_then(|v| v.trim().parse().ok())
    }

    /// Merge `entries` into the config and persist. Skips the disk write when nothing changed.
    pub fn update<I>(&self, entries: I) -> Result<(), String>
    where
        I: IntoIterator<Item = (String, String)>,
    {
        let mut map = self.lock();
        let mut changed = false;
        for (k, v) in entries {
            if map.get(&k) != Some(&v) {
                map.insert(k, v);
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
        // Serialize while holding the lock so concurrent writers persist in order.
        let json = serde_json::to_string_pretty(&*map).map_err(|e| e.to_string())?;
        match &self.path {
            Some(path) => write_atomic(path, &json),
            None => Ok(()),
        }
    }

    pub fn set_position(&self, keys: (&str, &str), pos: (i32, i32)) {
        let _ = self.update([
            (keys.0.to_string(), pos.0.to_string()),
            (keys.1.to_string(), pos.1.to_string()),
        ]);
    }
}

fn write_atomic(path: &PathBuf, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, contents).map_err(|e| e.to_string())?;
    // On Windows `rename` maps to MoveFileExW(MOVEFILE_REPLACE_EXISTING).
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("sys4_cfg_test_{}_{}", std::process::id(), name));
        p.push("config.json");
        p
    }

    #[test]
    fn update_merges_and_persists() {
        let path = temp_path("merge");
        let _ = fs::remove_file(&path);

        let store = ConfigStore::load(Some(path.clone()));
        store.update([("a".to_string(), "1".to_string())]).unwrap();
        store.update([("b".to_string(), "2".to_string())]).unwrap();

        // A fresh load sees both keys: the second write did not clobber the first.
        let reloaded = ConfigStore::load(Some(path.clone()));
        let snap = reloaded.snapshot();
        assert_eq!(snap.get("a").map(String::as_str), Some("1"));
        assert_eq!(snap.get("b").map(String::as_str), Some("2"));

        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn get_i32_parses_and_rejects_garbage() {
        let store = ConfigStore::load(None);
        store
            .update([
                ("x".to_string(), "-120".to_string()),
                ("y".to_string(), "abc".to_string()),
            ])
            .unwrap();
        assert_eq!(store.get_i32("x"), Some(-120));
        assert_eq!(store.get_i32("y"), None);
        assert_eq!(store.get_i32("missing"), None);
    }

    #[test]
    fn invalid_file_loads_as_empty() {
        let path = temp_path("invalid");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        let store = ConfigStore::load(Some(path.clone()));
        assert!(store.snapshot().is_empty());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }
}
