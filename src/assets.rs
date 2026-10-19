//! Read-only Source search paths; no archive extraction or executable loading.
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
};
const LIMIT: usize = 64 * 1024 * 1024;
pub fn normalise(s: &str) -> Result<String, String> {
    let s = s.replace('\\', "/").to_ascii_lowercase();
    if s.is_empty()
        || s.starts_with('/')
        || s.contains(':')
        || s.contains('\0')
        || s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err("unsafe asset path".into());
    }
    Ok(s)
}
struct Entry {
    path: PathBuf,
    offset: u64,
    length: usize,
    preload: Vec<u8>,
}
pub struct Assets {
    roots: Vec<PathBuf>,
    entries: HashMap<String, Entry>,
    pak: HashMap<String, Vec<u8>>,
    recorded: Option<std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>>,
}
fn zstring(b: &[u8], p: &mut usize) -> Result<String, String> {
    let tail = b.get(*p..).ok_or("VPK tree bounds")?;
    let n = tail
        .iter()
        .position(|x| *x == 0)
        .ok_or("VPK unterminated string")?;
    let s = std::str::from_utf8(&tail[..n])
        .map_err(|e| e.to_string())?
        .to_owned();
    *p += n + 1;
    Ok(s)
}
impl Assets {
    /// Mount asynchronously prefetched browser assets; BSP pak entries override them.
    pub fn from_memory(files: Vec<(String, Vec<u8>)>, bsp: Option<&[u8]>) -> Result<Self, String> {
        let mut fs = Self { roots: vec![], entries: HashMap::new(), pak: HashMap::new(), recorded: None };
        let mut total = 0usize;
        for (name, bytes) in files {
            let key = normalise(&name)?;
            total = total.checked_add(bytes.len()).ok_or("asset pack size overflow")?;
            if bytes.len() > LIMIT || total > 512 * 1024 * 1024 { return Err("oversized memory asset pack".into()); }
            if fs.pak.insert(key, bytes).is_some() { return Err("duplicate memory asset".into()); }
        }
        fs.mount_bsp(bsp)?;
        Ok(fs)
    }
    fn mount_bsp(&mut self, bsp: Option<&[u8]>) -> Result<(), String> {
        if let Some(b) = bsp {
            let pak = crate::lump(b, 40, 1)?;
            if !pak.is_empty() {
                let mut zip = zip::ZipArchive::new(std::io::Cursor::new(pak)).map_err(|e| e.to_string())?;
                let mut total = 0u64;
                for n in 0..zip.len() {
                    let mut file = zip.by_index(n).map_err(|e| e.to_string())?;
                    if file.is_dir() { continue; }
                    let key = normalise(file.name())?;
                    total += file.size();
                    if file.size() > LIMIT as u64 || total > 256 * 1024 * 1024 { return Err("oversized map asset pack".into()); }
                    let mut bytes = vec![];
                    file.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
                    self.pak.insert(key, bytes);
                }
            }
        }
        Ok(())
    }
    pub fn from_game(game: impl AsRef<Path>, bsp: Option<&[u8]>) -> Result<Self, String> {
        let game = game.as_ref().canonicalize().map_err(|e| e.to_string())?;
        let base = game.parent().ok_or("game directory has no parent")?;
        let info = std::fs::read_to_string(game.join("gameinfo.txt"))
            .map_err(|e| format!("gameinfo.txt: {e}"))?;
        let tree = crate::kv::parse(&info)?;
        let paths = crate::kv::child(&tree, "GameInfo")
            .and_then(|t| crate::kv::child(t, "FileSystem"))
            .and_then(|t| crate::kv::child(t, "SearchPaths"))
            .ok_or("missing gameinfo SearchPaths")?;
        let mut fs = Self {
            roots: vec![],
            entries: HashMap::new(),
            pak: HashMap::new(),
            recorded: None,
        };
        for (kind, value) in paths {
            if !kind
                .split('+')
                .any(|k| ["game", "mod", "platform"].contains(&k.to_ascii_lowercase().as_str()))
            {
                continue;
            }
            let crate::kv::Value::Text(value) = value else {
                return Err("invalid SearchPath value".into());
            };
            let value = value.replace('\\', "/");
            let path = if value.to_ascii_lowercase().starts_with("|gameinfo_path|") {
                game.join(&value[15..])
            } else if value
                .to_ascii_lowercase()
                .starts_with("|all_source_engine_paths|")
            {
                base.join(&value[25..])
            } else if value.contains('|') {
                return Err(format!("unsupported SearchPath token {value}"));
            } else {
                base.join(&value)
            };
            let mut candidates = if value.ends_with("/*") {
                let parent = path.parent().ok_or("wildcard path")?;
                if !parent.is_dir() {
                    continue;
                }
                let mut paths = std::fs::read_dir(parent)
                    .map_err(|e| e.to_string())?
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_dir()
                            || p.extension().is_some_and(|s| s == "vpk")
                                && !p.file_stem().is_some_and(|s| {
                                    s.to_string_lossy().rsplit('_').next().is_some_and(|n| {
                                        n.len() == 3 && n.bytes().all(|c| c.is_ascii_digit())
                                    })
                                })
                    })
                    .collect::<Vec<_>>();
                paths.sort();
                paths
            } else {
                vec![path]
            };
            for path in &mut candidates {
                if path.extension().is_some_and(|s| s == "vpk") && !path.exists() {
                    *path = path.with_file_name(format!(
                        "{}_dir.vpk",
                        path.file_stem().unwrap().to_string_lossy()
                    ));
                }
                if !path.exists() {
                    continue;
                }
                let path = path.canonicalize().map_err(|e| e.to_string())?;
                if !path.starts_with(base) {
                    return Err("SearchPath escapes installation root".into());
                }
                if path.is_file() {
                    fs.index(&path)?;
                }
                fs.roots.push(path);
            }
        }
        if fs.roots.is_empty() {
            return Err("no existing game search paths".into());
        }
        fs.mount_bsp(bsp)?;
        Ok(fs)
    }
    /// BSP names visible through the same ordered search paths used by `read`.
    pub fn maps(&self) -> Vec<String> {
        let mut names = std::collections::BTreeSet::new();
        for root in &self.roots {
            if root.is_dir() {
                if let Ok(entries) = std::fs::read_dir(root.join("maps")) {
                    for entry in entries.flatten() {
                        if entry.path().is_file() {
                            if let Some(name) = entry.file_name().to_str().and_then(map_name) {
                                names.insert(name.to_owned());
                            }
                        }
                    }
                }
            } else {
                let prefix = format!("{}\0maps/", root.display());
                for key in self
                    .entries
                    .keys()
                    .filter_map(|key| key.strip_prefix(&prefix))
                {
                    if let Some(name) = map_name(key) {
                        names.insert(name.to_owned());
                    }
                }
            }
        }
        for key in self.pak.keys().filter_map(|key| key.strip_prefix("maps/")) {
            if let Some(name) = map_name(key) { names.insert(name.to_owned()); }
        }
        names.into_iter().collect()
    }
    pub fn search_paths(&self) -> Vec<String> {
        self.roots
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect()
    }
    fn index(&mut self, path: &Path) -> Result<(), String> {
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut header = [0; 12];
        file.read_exact(&mut header).map_err(|e| e.to_string())?;
        let u = |o| u32::from_le_bytes(header[o..o + 4].try_into().unwrap());
        if u(0) != 0x55aa1234 || ![1, 2].contains(&u(4)) {
            return Err("invalid VPK header/version".into());
        }
        let head = if u(4) == 1 { 12 } else { 28 };
        let len = u(8) as usize;
        if len > LIMIT {
            return Err("oversized VPK tree".into());
        }
        file.seek(SeekFrom::Start(head))
            .map_err(|e| e.to_string())?;
        let mut tree = vec![0; len];
        file.read_exact(&mut tree).map_err(|e| e.to_string())?;
        let mut p = 0;
        loop {
            let ext = zstring(&tree, &mut p)?;
            if ext.is_empty() {
                break;
            }
            loop {
                let dir = zstring(&tree, &mut p)?;
                if dir.is_empty() {
                    break;
                }
                loop {
                    let name = zstring(&tree, &mut p)?;
                    if name.is_empty() {
                        break;
                    }
                    let a = tree.get(p..p + 18).ok_or("VPK entry bounds")?;
                    p += 18;
                    let pre = u16::from_le_bytes(a[4..6].try_into().unwrap()) as usize;
                    let arc = u16::from_le_bytes(a[6..8].try_into().unwrap());
                    let off = u32::from_le_bytes(a[8..12].try_into().unwrap());
                    let length = u32::from_le_bytes(a[12..16].try_into().unwrap()) as usize;
                    if a[16..18] != [255, 255] || length + pre > LIMIT {
                        return Err("invalid VPK entry".into());
                    }
                    let preload = tree.get(p..p + pre).ok_or("VPK preload bounds")?.to_vec();
                    p += pre;
                    let key = normalise(&format!(
                        "{}{}.{}",
                        if dir == " " {
                            String::new()
                        } else {
                            format!("{dir}/")
                        },
                        name,
                        ext
                    ))?;
                    let archive = if arc == 0x7fff {
                        path.to_owned()
                    } else {
                        PathBuf::from(
                            path.to_string_lossy()
                                .replace("_dir.vpk", &format!("_{arc:03}.vpk")),
                        )
                    };
                    let offset = off as u64 + if arc == 0x7fff { head + len as u64 } else { 0 };
                    self.entries
                        .entry(format!("{}\0{key}", path.display()))
                        .or_insert(Entry {
                            path: archive,
                            offset,
                            length,
                            preload,
                        });
                }
            }
        }
        Ok(())
    }
    pub fn read(&self, key: &str) -> Result<Vec<u8>, String> {
        let bytes = self.read_inner(key)?;
        if let Some(recorded) = &self.recorded {
            recorded.lock().map_err(|_| "asset recording lock poisoned")?.insert(normalise(key)?, bytes.clone());
        }
        Ok(bytes)
    }
    /// Opt-in dependency capture for private browser bundles; normal reads do not retain bytes.
    pub fn record_reads(&mut self) {
        self.recorded = Some(std::sync::Mutex::new(std::collections::BTreeMap::new()));
    }
    pub fn recorded_files(&self) -> Vec<(String, Vec<u8>)> {
        self.recorded.as_ref().map(|r| r.lock().expect("asset recording lock poisoned").iter().map(|(k,v)| (k.clone(),v.clone())).collect()).unwrap_or_default()
    }
    fn read_inner(&self, key: &str) -> Result<Vec<u8>, String> {
        let key = normalise(key)?;
        if let Some(b) = self.pak.get(&key) {
            return Ok(b.clone());
        }
        for root in &self.roots {
            if let Some(e) = self.entries.get(&format!("{}\0{key}", root.display())) {
                let mut f = std::fs::File::open(&e.path).map_err(|e| e.to_string())?;
                let path = e.path.canonicalize().map_err(|e| e.to_string())?;
                if !path.starts_with(root.parent().ok_or("archive parent")?) {
                    return Err("VPK archive symlink escapes directory".into());
                }
                if e.offset
                    .checked_add(e.length as u64)
                    .is_none_or(|end| end > f.metadata().map(|m| m.len()).unwrap_or(0))
                {
                    return Err("VPK archive bounds".into());
                }
                f.seek(SeekFrom::Start(e.offset))
                    .map_err(|e| e.to_string())?;
                let mut b = e.preload.clone();
                b.resize(b.len() + e.length, 0);
                f.read_exact(&mut b[e.preload.len()..])
                    .map_err(|e| e.to_string())?;
                return Ok(b);
            }
            if !root.is_dir() {
                continue;
            }
            let path = root.join(&key);
            if path.is_file() {
                let path = path.canonicalize().map_err(|e| e.to_string())?;
                if !path.starts_with(root) {
                    return Err("asset symlink escapes search path".into());
                }
                if path.metadata().map_err(|e| e.to_string())?.len() > LIMIT as u64 {
                    return Err("oversized loose asset".into());
                }
                return std::fs::read(path).map_err(|e| e.to_string());
            }
        }
        Err(format!("missing asset {key}"))
    }
}

/// Source map identifiers accepted by the UI and map loader.
pub fn map_name(path: &str) -> Option<&str> {
    let name = path.strip_suffix(".bsp")?;
    if !name.is_empty() && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
        Some(name)
    } else {
        None
    }
}

/// Resolve a map selector through the same asset lookup used at startup.
pub fn read_selected_map(fs: &Assets, map: &str) -> Result<Vec<u8>, String> {
    let name = map.strip_prefix("maps/").unwrap_or(map);
    let name = name.strip_suffix(".bsp").unwrap_or(name);
    if map_name(&format!("{name}.bsp")) != Some(name) {
        return Err("unsafe map name".into());
    }
    fs.read(&format!("maps/{name}.bsp"))
}
