// src/core.rs
// src/lib.rs یا src/core.rs
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{ProcessExt, System, SystemExt, PidExt};
use std::env;

pub fn get_config_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| env::temp_dir());
    dir.push("LangSwitcher");
    fs::create_dir_all(&dir).ok();
    let cfg_path = dir.join("lang_config.json");

    // نسخه پیش‌فرض را از assets کپی کن
    if !cfg_path.exists() {
        if let Ok(exe) = env::current_exe() {
            if let Some(parent) = exe.parent() {
                let default = parent.join("assets").join("lang_config.json");
                if default.exists() {
                    let _ = fs::copy(default, &cfg_path);
                }
            }
        }
    }
    cfg_path
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config(pub HashMap<String, String>);

#[derive(Debug, Clone)]
pub struct Program {
    pub name: String,
    pub lang: String,
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

    #[cfg(target_os = "windows")]
    fn visible_window_pids() -> HashSet<u32> {
        use std::collections::HashSet;
        use std::ptr::null_mut;
        use winapi::shared::minwindef::{BOOL, DWORD, LPARAM, TRUE};
        use winapi::shared::windef::HWND;
        use winapi::um::winuser::{EnumWindows, GetWindowTextLengthW, IsWindowVisible, GetWindowThreadProcessId};

        extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
            unsafe {
                if IsWindowVisible(hwnd) == 0 { return TRUE; }
                let len = GetWindowTextLengthW(hwnd);
                if len == 0 { return TRUE; }
                let mut pid: DWORD = 0;
                GetWindowThreadProcessId(hwnd, &mut pid);
                if pid != 0 {
                    let set_ptr = lparam as *mut HashSet<u32>;
                    if !set_ptr.is_null() { (*set_ptr).insert(pid as u32); }
                }
                TRUE
            }
        }

        let mut set: HashSet<u32> = HashSet::new();
        unsafe {
            EnumWindows(Some(enum_proc), &mut set as *mut _ as LPARAM);
        }
        set
    }

    #[cfg(not(target_os = "windows"))]
    fn visible_window_pids() -> HashSet<u32> { HashSet::new() }

    fn should_skip_process(proc_name: &str, exe_path_opt: Option<&PathBuf>) -> bool {
        let blacklist_names = [
            "explorer.exe", "shellexperiencehost.exe", "systemsettings.exe",
            "applicationframehost.exe", "searchui.exe", "startmenuexperiencehost.exe",
            "sihost.exe", "runtimebroker.exe", "audiodg.exe", "wsappx.exe",
            "smss.exe", "csrss.exe", "wininit.exe", "services.exe", "lsass.exe",
            "dwm.exe", "taskhostw.exe",
        ];

        let lower_name = proc_name.to_lowercase();
        if blacklist_names.iter().any(|s| *s == lower_name) { return true; }

        if let Some(exe_path) = exe_path_opt {
            if let Some(s) = exe_path.to_str() {
                let low = s.to_lowercase();
                if low.contains("\\windows\\") || low.contains("/windows/") ||
                   low.contains("\\system32\\") || low.contains("/system32/") ||
                   low.contains("\\syswow64\\") || low.contains("/syswow64/") {
                    return true;
                }
            }
        }

        if let Ok(current) = env::current_exe() {
            if let Some(cur_name) = current.file_name().and_then(|n| n.to_str()) {
                if cur_name.eq_ignore_ascii_case(proc_name) { return true; }
            }
        }

        false
    }

    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_processes();

        let cfg: Config = fs::read_to_string(get_config_path())
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
                    if filter_by_windows && !visible_pids.contains(&pid_u) { continue; }

                    let proc_name = proc_.name().to_string();
                    let exe = proc_.exe();
                    let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };

                    if Self::should_skip_process(&proc_name, exe_opt.as_ref()) { break; }

                    let key = exe_opt.as_ref()
                        .map(|p| p.to_string_lossy().to_string().to_lowercase())
                        .unwrap_or_else(|| proc_name.to_lowercase());
                    if seen.contains(&key) { break; }

                    let lang = cfg.0.get(&proc_name).cloned().unwrap_or_else(|| "en".to_string());
                    progs.push(Program { name: proc_name.clone(), lang, exe_path: exe_opt });
                    seen.insert(key);
                    break;
                }
            }
        }

        self_fill_other_processes(&mut progs, &mut seen, filter_by_windows, &cfg, &mut sys);

        Self { programs: progs }
    }

    pub fn save_config(&self) -> std::io::Result<()> {
        let mut cfg = Config::default();
        for p in &self.programs { cfg.0.insert(p.name.clone(), p.lang.clone()); }
        let txt = serde_json::to_string_pretty(&cfg)?;
        fs::write(get_config_path(), txt)?;
        Ok(())
    }

    pub fn refresh(&mut self) {
        let cfg: Config = fs::read_to_string(get_config_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        // همان منطق ساخت پروسه‌ها
        let mut sys = System::new_all();
        sys.refresh_processes();

        let visible_pids = Self::visible_window_pids();
        let filter_by_windows = !visible_pids.is_empty();

        let mut seen: HashSet<String> = HashSet::new();
        let mut progs: Vec<Program> = Vec::new();

        for &pname in Self::predefined_list().iter() {
            for (_pid, proc_) in sys.processes() {
                if proc_.name().eq_ignore_ascii_case(pname) {
                    let pid_u = proc_.pid().as_u32();
                    if filter_by_windows && !visible_pids.contains(&pid_u) { continue; }
                    let proc_name = proc_.name().to_string();
                    let exe = proc_.exe();
                    let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };
                    if Self::should_skip_process(&proc_name, exe_opt.as_ref()) { break; }

                    let key = exe_opt.as_ref()
                        .map(|p| p.to_string_lossy().to_string().to_lowercase())
                        .unwrap_or_else(|| proc_name.to_lowercase());
                    if seen.contains(&key) { break; }

                    let lang = self.programs.iter().find(|x| x.name.eq_ignore_ascii_case(&proc_name))
                        .map(|x| x.lang.clone())
                        .or_else(|| cfg.0.get(&proc_name).cloned())
                        .unwrap_or_else(|| "en".to_string());

                    progs.push(Program { name: proc_name.clone(), lang, exe_path: exe_opt });
                    seen.insert(key);
                    break;
                }
            }
        }

        self_fill_other_processes(&mut progs, &mut seen, filter_by_windows, &cfg, &mut sys);

        self.programs = progs;
    }
}

// helper function برای سایر پروسه‌ها
fn self_fill_other_processes(progs: &mut Vec<Program>, seen: &mut HashSet<String>, filter_by_windows: bool, cfg: &Config, sys: &mut System) {
    let mut other: Vec<(String, Option<PathBuf>, u32)> = Vec::new();
    for (_pid, proc_) in sys.processes() {
        let pid_u = proc_.pid().as_u32();
        if filter_by_windows && !LangState::visible_window_pids().contains(&pid_u) { continue; }
        let name = proc_.name().to_string();
        if name.trim().is_empty() { continue; }
        let exe = proc_.exe();
        let exe_opt = if exe.as_os_str().is_empty() { None } else { Some(exe.to_path_buf()) };
        if LangState::should_skip_process(&name, exe_opt.as_ref()) { continue; }
        let key = exe_opt.as_ref()
            .map(|p| p.to_string_lossy().to_string().to_lowercase())
            .unwrap_or_else(|| name.to_lowercase());
        if seen.contains(&key) { continue; }
        seen.insert(key.clone());
        other.push((name, exe_opt, pid_u));
    }
    other.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    for (name, exe_opt, _) in other {
        let lang = cfg.0.get(&name).cloned().unwrap_or_else(|| "en".to_string());
        progs.push(Program { name, lang, exe_path: exe_opt });
    }
}
