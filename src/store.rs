//! 本地 JSON 存储。写入先落临时文件再替换，断电也不会写坏旧文件。
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::model::{Config, Data};

pub fn data_dir() -> PathBuf {
    if let Some(dir) = env::var_os("NIUMA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    let base = env::var_os("APPDATA").map_or_else(|| PathBuf::from("."), PathBuf::from);
    base.join("NiuMaClock")
}

fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> T {
    let Ok(text) = fs::read_to_string(path) else {
        return T::default();
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(_) => {
            // 文件坏了不覆盖，留一份给用户救数据。
            let stamp = chrono::Local::now().format("%Y%m%d%H%M%S");
            let _ = fs::rename(path, path.with_extension(format!("corrupt-{stamp}.json")));
            T::default()
        }
    }
}

fn save_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

pub fn load_config(dir: &Path) -> Config {
    load_or_default(&dir.join("config.json"))
}

pub fn load_data(dir: &Path) -> Data {
    load_or_default(&dir.join("data.json"))
}

pub fn save_config(dir: &Path, cfg: &Config) -> io::Result<()> {
    save_json(&dir.join("config.json"), cfg)
}

pub fn save_data(dir: &Path, data: &Data) -> io::Result<()> {
    save_json(&dir.join("data.json"), data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("niuma-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn round_trip_creates_directory() {
        let dir = temp_dir("roundtrip");
        let mut cfg = Config::default();
        cfg.salary.monthly = 12345.0;
        save_config(&dir, &cfg).unwrap();
        assert_eq!(load_config(&dir), cfg);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_file_gives_default() {
        let dir = temp_dir("missing");
        assert_eq!(load_data(&dir), Data::default());
    }

    #[test]
    fn corrupt_file_is_kept_aside() {
        let dir = temp_dir("corrupt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("data.json"), "{not json").unwrap();
        assert_eq!(load_data(&dir), Data::default());
        let kept = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains("corrupt"));
        assert!(kept);
        fs::remove_dir_all(&dir).unwrap();
    }
}
