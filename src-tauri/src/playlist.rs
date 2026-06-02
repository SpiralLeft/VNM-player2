use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LoopMode {
    None,
    Single,
    List,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaylistEntry {
    pub path: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistState {
    pub entries: Vec<PlaylistEntry>,
    pub current_index: Option<usize>,
    pub loop_mode: LoopMode,
}

pub struct Playlist {
    pub entries: Vec<PlaylistEntry>,
    pub current_index: Option<usize>,
    pub loop_mode: LoopMode,
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_index: None,
            loop_mode: LoopMode::None,
        }
    }

    pub fn add(&mut self, path: String, duration_ms: u64) -> usize {
        // 去重
        if let Some(pos) = self.entries.iter().position(|e| e.path == path) {
            return pos;
        }
        let idx = self.entries.len();
        self.entries.push(PlaylistEntry { path, duration_ms });
        idx
    }

    pub fn update_duration(&mut self, path: &str, duration_ms: u64) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.path == path) {
            entry.duration_ms = duration_ms;
        }
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.entries.len() {
            self.entries.remove(index);
            // 调整当前索引
            if let Some(cur) = self.current_index {
                if cur == index {
                    self.current_index = None;
                } else if cur > index {
                    self.current_index = Some(cur - 1);
                }
            }
        }
    }

    pub fn set_loop_mode(&mut self, mode: LoopMode) {
        self.loop_mode = mode;
    }

    pub fn set_current(&mut self, index: usize) -> Option<&PlaylistEntry> {
        if index < self.entries.len() {
            self.current_index = Some(index);
            Some(&self.entries[index])
        } else {
            None
        }
    }

    pub fn current_entry(&self) -> Option<&PlaylistEntry> {
        self.current_index.and_then(|i| self.entries.get(i))
    }

    pub fn next(&mut self) -> Option<usize> {
        match self.current_index {
            Some(i) if i + 1 < self.entries.len() => {
                self.current_index = Some(i + 1);
                self.current_index
            }
            _ => None,
        }
    }

    pub fn previous(&mut self) -> Option<usize> {
        match self.current_index {
            Some(i) if i > 0 => {
                self.current_index = Some(i - 1);
                self.current_index
            }
            _ => None,
        }
    }

    pub fn state(&self) -> PlaylistState {
        PlaylistState {
            entries: self.entries.clone(),
            current_index: self.current_index,
            loop_mode: self.loop_mode.clone(),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_index = None;
    }
}

/// 扫描文件夹中支持的音频文件，并尝试获取时长
pub fn scan_folder(path: &str) -> Result<Vec<(String, u64)>, String> {
    let mut files = Vec::new();
    let dir = std::fs::read_dir(path).map_err(|e| format!("Cannot read directory: {}", e))?;
    for entry in dir {
        let entry = entry.map_err(|e| format!("{}", e))?;
        let fpath = entry.path();
        if fpath.is_file() {
            if let Some(ext) = fpath.extension().and_then(|e| e.to_str()) {
                let ext = ext.to_lowercase();
                if matches!(ext.as_str(), "mp3" | "wav" | "ogg" | "flac" | "nwa") {
                    let path_str = fpath.to_string_lossy().to_string();
                    let dur = try_get_duration(&path_str);
                    files.push((path_str, dur));
                }
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(files)
}

/// 快速读取音频文件的时长（ms）
fn try_get_duration(path: &str) -> u64 {
    use std::path::Path;
    let p = Path::new(path);
    let path_lower = path.to_lowercase();
    let result: Result<Box<dyn crate::decoder::Decoder>, _> = if path_lower.ends_with(".nwa") {
        crate::nwa_decoder::NwaDecoder::open(p).map(|d| Box::new(d) as Box<dyn crate::decoder::Decoder>)
    } else {
        crate::symphonia_decoder::SymphoniaDecoder::open(p).map(|d| Box::new(d) as Box<dyn crate::decoder::Decoder>)
    };
    result.and_then(|d| d.duration()).unwrap_or(0)
}
