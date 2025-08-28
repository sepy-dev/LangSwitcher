// src/bin/watcher.rs
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

use sysinfo::{ProcessExt, System, SystemExt};

const CONFIG_FILE: &str = "lang_config.json";
const POLL_MS: u64 = 300;

fn load_config() -> HashMap<String, String> {
    if !Path::new(CONFIG_FILE).exists() {
        return HashMap::new();
    }
    match fs::read_to_string(CONFIG_FILE) {
        Ok(txt) => match serde_json::from_str::<HashMap<String, String>>(&txt) {
            Ok(map) => map,
            Err(e) => {
                eprintln!("Failed to parse {}: {}", CONFIG_FILE, e);
                HashMap::new()
            }
        },
        Err(e) => {
            eprintln!("Failed to read {}: {}", CONFIG_FILE, e);
            HashMap::new()
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use widestring::U16CString;
    use winapi::shared::minwindef::{DWORD, LPARAM, WPARAM};
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{
        GetForegroundWindow, GetWindowThreadProcessId, LoadKeyboardLayoutW, PostMessageW,
        WM_INPUTLANGCHANGEREQUEST,
    };

    fn get_foreground_pid() -> Option<u32> {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid: DWORD = 0;
            GetWindowThreadProcessId(hwnd, &mut pid as *mut u32);
            if pid == 0 {
                None
            } else {
                Some(pid as u32)
            }
        }
    }

    fn lang_to_klid(lang: &str) -> Option<&'static str> {
        match lang {
            "en" => Some("00000409"), // English (US)
            "fa" => Some("00000429"), // Persian
            _ => None,
        }
    }

    pub fn set_layout_for_pid(pid: u32, lang: &str) -> bool {
        if let Some(klid) = lang_to_klid(lang) {
            unsafe {
                let wide = U16CString::from_str(klid).unwrap();
                let hkl = LoadKeyboardLayoutW(wide.as_ptr(), 1); // KLF_ACTIVATE = 1
                let hwnd = GetForegroundWindow();
                if hwnd.is_null() {
                    eprintln!("Foreground window null");
                    return false;
                }
                PostMessageW(hwnd, WM_INPUTLANGCHANGEREQUEST, 0 as WPARAM, hkl as LPARAM);
                return true;
            }
        }
        false
    }

    pub fn watch_loop() {
        println!("Windows watcher started.");
        let mut sys = System::new_all();
        let mut last_pid: Option<u32> = None;
        let mut last_req_lang: Option<String> = None;

        loop {
            let cfg = load_config();

            if let Some(pid) = get_foreground_pid() {
                if Some(pid) != last_pid {
                    // refresh processes
                    sys.refresh_processes();

                    // find process by pid
                    let mut proc_name_opt: Option<String> = None;
                    for (_k, proc_) in sys.processes() {
                        if format!("{}", proc_.pid()) == format!("{}", pid) {
                            proc_name_opt = Some(proc_.name().to_string());
                            break;
                        }
                    }

                    if let Some(proc_name) = proc_name_opt {
                        // find mapping (case-insensitive)
                        let mut desired: Option<String> = None;
                        if let Some(v) = cfg.get(&proc_name) {
                            desired = Some(v.clone());
                        } else {
                            for (k, v) in cfg.iter() {
                                if k.eq_ignore_ascii_case(&proc_name) {
                                    desired = Some(v.clone());
                                    break;
                                }
                            }
                        }

                        if let Some(lang) = desired {
                            if last_req_lang.as_ref().map(|s| s.as_str()) != Some(lang.as_str()) {
                                if set_layout_for_pid(pid, &lang) {
                                    println!("Requested layout {} for process {} (pid {})", lang, proc_name, pid);
                                    last_req_lang = Some(lang);
                                } else {
                                    eprintln!("Failed to set layout for {}", proc_name);
                                }
                            }
                        } else {
                            last_req_lang = None;
                        }
                    }
                    last_pid = Some(pid);
                }
            }

            thread::sleep(Duration::from_millis(POLL_MS));
        }
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::process::Command;

    fn get_active_window_pid() -> Option<u32> {
        let out = Command::new("xdotool").arg("getactivewindow").output();
        if let Ok(out) = out {
            if !out.status.success() { return None; }
            let winid = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if winid.is_empty() { return None; }
            let out2 = Command::new("xdotool").arg("getwindowpid").arg(&winid).output();
            if let Ok(out2) = out2 {
                if !out2.status.success() { return None; }
                let pid_str = String::from_utf8_lossy(&out2.stdout).trim().to_string();
                if let Ok(pid) = pid_str.parse::<u32>() { return Some(pid); }
            }
        }
        None
    }

    fn set_layout_for_lang(lang: &str) -> bool {
        let args = match lang {
            "en" => vec!["us"],
            "fa" => vec!["ir"],
            _ => return false,
        };
        match Command::new("setxkbmap").args(&args).status() {
            Ok(s) => s.success(),
            Err(e) => { eprintln!("Failed to call setxkbmap: {}", e); false }
        }
    }

    pub fn watch_loop() {
        println!("Linux watcher started (requires xdotool & setxkbmap).");
        let mut sys = System::new_all();
        let mut last_pid: Option<u32> = None;
        let mut last_req_lang: Option<String> = None;

        loop {
            let cfg = load_config();

            if let Some(pid) = get_active_window_pid() {
                if Some(pid) != last_pid {
                    sys.refresh_processes();
                    let mut proc_name_opt: Option<String> = None;
                    for (_k, proc_) in sys.processes() {
                        if format!("{}", proc.pid()) == format!("{}", pid) {
                            proc_name_opt = Some(proc.name().to_string());
                            break;
                        }
                    }
                    if let Some(proc_name) = proc_name_opt {
                        let mut desired: Option<String> = None;
                        if let Some(v) = cfg.get(&proc_name) {
                            desired = Some(v.clone());
                        } else {
                            for (k, v) in cfg.iter() {
                                if k.eq_ignore_ascii_case(&proc_name) {
                                    desired = Some(v.clone());
                                    break;
                                }
                            }
                        }

                        if let Some(lang) = desired {
                            if last_req_lang.as_ref().map(|s| s.as_str()) != Some(lang.as_str()) {
                                if set_layout_for_lang(&lang) {
                                    println!("Switch to {} for {}", lang, proc_name);
                                    last_req_lang = Some(lang);
                                } else {
                                    eprintln!("Failed to set layout for {}", proc_name);
                                }
                            }
                        } else {
                            last_req_lang = None;
                        }
                    }
                    last_pid = Some(pid);
                }
            }

            thread::sleep(Duration::from_millis(POLL_MS));
        }
    }
}

fn main() {
    println!("Layout watcher (Rust). Reading config from '{}'", CONFIG_FILE);
    if !Path::new(CONFIG_FILE).exists() {
        eprintln!("Warning: {} not found. Create it through GUI or manually.", CONFIG_FILE);
    }

    #[cfg(target_os = "windows")]
    platform::watch_loop();

    #[cfg(target_os = "linux")]
    platform::watch_loop();

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    eprintln!("Watcher supports only Windows and Linux (X11) for now.");
}
