//! Platform file I/O for saves: native filesystem, or browser download / file picker on web.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const EXT: &str = "civsave";
pub fn now_string() -> String {
    civ_core::save::format_unix_time(macroquad::miniquad::date::now())
}
pub fn stamp() -> String {
    now_string()
        .replace([':', '-'], "")
        .replace('T', "-")
        .trim_end_matches('Z')
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::path::{Path, PathBuf};
    pub const SAVE_DIR: &str = "saves";
    pub fn write(name: &str, bytes: &[u8]) -> Result<String, String> {
        std::fs::create_dir_all(SAVE_DIR).map_err(|e| e.to_string())?;
        let p = Path::new(SAVE_DIR).join(name);
        let tmp = p.with_extension("tmp");
        std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &p).map_err(|e| e.to_string())?; // atomic-ish replace
        Ok(p.display().to_string())
    }
    pub fn read(path: &str) -> Result<Vec<u8>, String> {
        std::fs::read(path).map_err(|e| format!("{path}: {e}"))
    }
    /// (path, modified unix secs, size), newest first
    pub fn list() -> Vec<(String, u64, u64)> {
        let mut v: Vec<_> = std::fs::read_dir(SAVE_DIR)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                let ext = p.extension()?.to_str()?.to_string();
                if ext != super::EXT && ext != "json" {
                    return None;
                }
                let m = e.metadata().ok()?;
                let t = m
                    .modified()
                    .ok()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .ok()?
                    .as_secs();
                Some((p.display().to_string(), t, m.len()))
            })
            .collect();
        v.sort_by_key(|f| std::cmp::Reverse(f.1));
        v
    }
    /// Autosave rotation: the missing or oldest of autosave-1..=keep.
    pub fn autosave_name(keep: u32) -> String {
        let mut best: Option<(u64, u32)> = None;
        for k in 1..=keep.max(1) {
            let p: PathBuf = Path::new(SAVE_DIR).join(format!("autosave-{k}.{}", super::EXT));
            let t = std::fs::metadata(&p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if best.is_none_or(|(bt, _)| t < bt) {
                best = Some((t, k));
            }
        }
        format!("autosave-{}.{}", best.unwrap().1, super::EXT)
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use imp::*;

#[cfg(target_arch = "wasm32")]
mod imp {
    extern "C" {
        fn civ_download(p: *const u8, len: u32, name: *const u8, name_len: u32);
        fn civ_pick_file();
        fn civ_file_len() -> i32;
        fn civ_file_take(p: *mut u8);
    }
    #[no_mangle]
    pub extern "C" fn civ_io_crate_version() -> u32 {
        1
    }
    pub fn download(name: &str, bytes: &[u8]) {
        unsafe {
            civ_download(
                bytes.as_ptr(),
                bytes.len() as u32,
                name.as_ptr(),
                name.len() as u32,
            )
        }
    }
    pub fn pick_file() {
        unsafe { civ_pick_file() }
    }
    pub fn poll_file() -> Option<Vec<u8>> {
        let n = unsafe { civ_file_len() };
        if n < 0 {
            return None;
        }
        let mut v = vec![0u8; n as usize];
        unsafe { civ_file_take(v.as_mut_ptr()) };
        Some(v)
    }
}
#[cfg(target_arch = "wasm32")]
pub use imp::*;
