// src/core.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use sysinfo::{ProcessExt, System, SystemExt, PidExt};

pub const CONFIG_FILE: &str = "lang_config.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config(pub HashMap<String, String>); // prog_name -> "en"|"fa"

#[derive(Debug, Clone)]
pub struct Program {
    pub name: String,
    pub lang: String,
    /// مسیر اجرایی برنامه در صورت موجود بودن (برای تلاش برای گرفتن آیکون یا اطلاعات بیشتر)
    pub exe_path: Option<PathBuf>,
}

pub struct LangState {
    pub programs: Vec<Program>,
}

impl LangState {
    fn predefined_list() -> Vec<&'static str> {
        vec![
            "Code.exe",
            "PyCharm.exe",
            "chrome.exe",
            "firefox.exe",
            "Opera.exe",
            "WINWORD.EXE",
            "EXCEL.EXE",
        ]
    }

    /// Returns a set of PIDs that have visible top-level windows (Windows only).
    #[cfg(target_os = "windows")]
    fn visible_window_pids() -> HashSet<u32> {
        use std::ptr::null_mut;
        use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
        use winapi::shared::windef::HWND;
        use winapi::um::winuser::{EnumWindows, GetWindowTextLengthW, IsWindowVisible, GetWindowThreadProcessId};

        extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                // only visible windows
                if IsWindowVisible(hwnd) == 0 {
                    return TRUE;
                }

                // must have non-zero title length
                let len = GetWindowTextLengthW(hwnd);
                if len == 0 {
                    return TRUE;
                }

                let mut pid: DWORD = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid != 0 {
                    let set_ptr = lparam as *mut HashSet<u32>;
                    if !set_ptr.is_null() {
                        (*set_ptr).insert(pid as u32);
                    }
                }
                TRUE
            }
        }

        let mut set: HashSet<u32> = HashSet::new();
        unsafe {
            let ptr = &mut set as *mut HashSet<u32>;
            EnumWindows(Some(enum_proc), ptr as LPARAM);
        }
        set
    }

    /// On non-windows keep behavior: return empty set so no PID filtering applied.
    #[cfg(not(target_os = "windows"))]
    fn visible_window_pids() -> HashSet<u32> {
        HashSet::new()
    }

    /// Helper: should we skip this process from listing?
    /// Rules:
    /// - if name is in blacklist (common shell/system names) -> skip
    /// - if exe path is clearly inside Windows system dirs -> skip
    /// - if exe path corresponds to current running exe (self) -> skip
    fn should_skip_process(proc_name: &str, exe_path_opt: Option<&PathBuf>) -> bool {
        // blacklist by name (lowercase)
        let blacklist_names = [
            "explorer.exe",
            "shellexperiencehost.exe",
            "systemsettings.exe",
            "applicationframehost.exe",
            "searchui.exe",
            "startmenuexperiencehost.exe",
            "sihost.exe",
            "runtimebroker.exe",
            "audiodg.exe",
            "wsappx.exe",
            "smss.exe",
            "csrss.exe",
            "wininit.exe",
            "services.exe",
            "lsass.exe",
            "dwm.exe", // Desktop Window Manager often shouldn't be listed
            "taskhostw.exe",
        ];

        let lower_name = proc_name.to_lowercase();
        if blacklist_names.iter().any(|s| *s == lower_name) {
            return true;
        }

        // skip if exe path is in Windows system folders
        if let Some(exe_path) = exe_path_opt {
            if let Some(s) = exe_path.to_str() {
                let low = s.to_lowercase();
                if low.contains("\\windows\\") || low.contains("/windows/")
                    || low.contains("\\system32\\") || low.contains("/system32/")
                    || low.contains("\\syswow64\\") || low.contains("/syswow64/")
                {
                    return true;
                }
            }
        }

        // skip the current executable (don't list self)
        if let Ok(current) = env::current_exe() {
            if let Some(cur_name) = current.file_name().and_then(|n| n.to_str()) {
                if cur_name.eq_ignore_ascii_case(proc_name) {
                    return true;
                }
            }
            if let Some(cur_path) = current.to_str() {
                if let Some(exe_path) = exe_path_opt {
                    if let Some(exe_s) = exe_path.to_str() {
                        // same path -> skip
                        if exe_s.eq_ignore_ascii_case(cur_path) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_processes();

        let cfg: Config = fs::read_to_string(CONFIG_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        // get visible window pids (if available)
        let visible_pids = Self::visible_window_pids();
        let filter_by_windows = !visible_pids.is_empty();

        let mut seen: HashSet<String> = HashSet::new(); // dedupe key
        let mut progs: Vec<Program> = Vec::new();

        // Prefer predefined list first (if those processes have windows / exist)
        for &pname in Self::predefined_list().iter() {
            for (_pid, proc_) in sys.processes() {
                if proc_.name().eq_ignore_ascii_case(pname) {
                    let pid_u = proc_.pid().as_u32();
                    if filter_by_windows && !visible_pids.contains(&pid_u) {
                        continue; // skip processes without visible window
                    }

                    let proc_name = proc_.name().to_string();
                    let exe = proc_.exe();
                    let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };

                    if Self::should_skip_process(&proc_name, exe_opt.as_ref()) {
                        break; // don't include this predefined process
                    }

                    let key = exe_opt
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string().to_lowercase())
                        .unwrap_or_else(|| proc_name.to_lowercase());
                    if seen.contains(&key) {
                        break;
                    }

                    let lang = cfg.0.get(&proc_name).cloned().unwrap_or_else(|| "en".to_string());
                    progs.push(Program { name: proc_name.clone(), lang, exe_path: exe_opt });
                    seen.insert(key);
                    break;
                }
            }
        }

        // Now add other running processes (deduped) — only those with window if filter active
        let mut other: Vec<(String, Option<PathBuf>, u32)> = Vec::new();
        for (_pid, proc_) in sys.processes() {
            let pid_u = proc_.pid().as_u32();
            if filter_by_windows && !visible_pids.contains(&pid_u) {
                continue;
            }

            let name = proc_.name().to_string();
            if name.trim().is_empty() {
                continue;
            }

            let exe = proc_.exe();
            let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };

            if Self::should_skip_process(&name, exe_opt.as_ref()) {
                continue;
            }

            let key = exe_opt
                .as_ref()
                .map(|p| p.to_string_lossy().to_string().to_lowercase())
                .unwrap_or_else(|| name.to_lowercase());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key.clone());
            other.push((name, exe_opt, pid_u));
        }

        other.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        for (name, exe_opt, _pid) in other {
            let lang = cfg.0.get(&name).cloned().unwrap_or_else(|| "en".to_string());
            progs.push(Program { name, lang, exe_path: exe_opt });
        }

        Self { programs: progs }
    }

    pub fn toggle_by_index(&mut self, idx: usize) {
        if let Some(p) = self.programs.get_mut(idx) {
            p.lang = if p.lang == "en" { "fa".to_string() } else { "en".to_string() };
        }
    }

    pub fn save_config(&self) -> std::io::Result<()> {
        let mut cfg = Config::default();
        for p in &self.programs {
            cfg.0.insert(p.name.clone(), p.lang.clone());
        }
        let txt = serde_json::to_string_pretty(&cfg)?;
        fs::write(CONFIG_FILE, txt)?;
        Ok(())
    }

    pub fn refresh(&mut self) {
        let mut sys = System::new_all();
        sys.refresh_processes();

        let cfg: Config = fs::read_to_string(CONFIG_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let visible_pids = Self::visible_window_pids();
        let filter_by_windows = !visible_pids.is_empty();

        let mut seen: HashSet<String> = HashSet::new();
        let mut progs: Vec<Program> = Vec::new();

        for &pname in Self::predefined_list().iter() {
            for (_pid, proc_) in sys.processes() {
                if proc_.name().eq_ignore_ascii_case(pname) {
                    let pid_u = proc_.pid().as_u32();
                    if filter_by_windows && !visible_pids.contains(&pid_u) {
                        continue;
                    }
                    let proc_name = proc_.name().to_string();
                    let exe = proc_.exe();
                    let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };

                    if Self::should_skip_process(&proc_name, exe_opt.as_ref()) {
                        break;
                    }

                    let key = exe_opt
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string().to_lowercase())
                        .unwrap_or_else(|| proc_name.to_lowercase());
                    if seen.contains(&key) {
                        break;
                    }
                    let lang = self.programs.iter().find(|x| x.name.eq_ignore_ascii_case(&proc_name)).map(|x| x.lang.clone())
                        .or_else(|| cfg.0.get(&proc_name).cloned())
                        .unwrap_or_else(|| "en".to_string());
                    progs.push(Program { name: proc_name.clone(), lang, exe_path: exe_opt });
                    seen.insert(key);
                    break;
                }
            }
        }

        let mut other: Vec<(String, Option<PathBuf>, u32)> = Vec::new();
        for (_pid, proc_) in sys.processes() {
            let pid_u = proc_.pid().as_u32();
            if filter_by_windows && !visible_pids.contains(&pid_u) {
                continue;
            }
            let name = proc_.name().to_string();
            if name.trim().is_empty() {
                continue;
            }
            let exe = proc_.exe();
            let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };

            if Self::should_skip_process(&name, exe_opt.as_ref()) {
                continue;
            }

            let key = exe_opt
                .as_ref()
                .map(|p| p.to_string_lossy().to_string().to_lowercase())
                .unwrap_or_else(|| name.to_lowercase());
            if seen.contains(&key) {
                continue;
            }
            seen.insert(key.clone());
            other.push((name, exe_opt, pid_u));
        }

        other.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        for (name, exe_opt, _pid) in other {
            let lang = self.programs.iter().find(|x| x.name.eq_ignore_ascii_case(&name)).map(|x| x.lang.clone())
                .or_else(|| cfg.0.get(&name).cloned())
                .unwrap_or_else(|| "en".to_string());
            progs.push(Program { name, lang, exe_path: exe_opt });
        }

        self.programs = progs;
    }
}
