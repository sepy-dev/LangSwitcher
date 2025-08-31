// src/bin/watcher.rs
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
// src/bin/watcher.rs
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use image::{GenericImageView}; // لازم برای dimensions()
use sysinfo::{ProcessExt, PidExt, System, SystemExt};

use tray_icon::{Icon, TrayIconBuilder};
use tray_icon::menu::{Menu, MenuItem, MenuEvent};

use winapi::shared::minwindef::UINT; // فقط ویندوز برای PeekMessage

const CONFIG_FILE: &str = "lang_config.json";
const POLL_MS: u64 = 300;
// src/bin/watcher.rs


fn get_config_path() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap_or_else(|| std::env::temp_dir());
    dir.push("LangSwitcher");
    fs::create_dir_all(&dir).ok();
    let cfg_path = dir.join("lang_config.json");
    if !cfg_path.exists() {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(parent) = exe.parent() {
                let default = parent.join("assets").join("lang_config.json");
                if default.exists() { let _ = fs::copy(default, &cfg_path); }
            }
        }
    }
    cfg_path
}

fn load_config() -> HashMap<String, String> {
    let cfg_path = get_config_path();
    if !cfg_path.exists() { return HashMap::new(); }
    match fs::read_to_string(&cfg_path) {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or_default(),
        Err(_) => HashMap::new()
    }
}

// ... بقیه کد watcher بدون تغییر، فقط CONFIG_FILE را با get_config_path() جایگزین کن



// ------------------ PLATFORM SPECIFIC ---------------------
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

    pub fn set_layout_for_pid(_pid: u32, lang: &str) -> bool {
        if let Some(klid) = lang_to_klid(lang) {
            unsafe {
                let wide = U16CString::from_str(klid).unwrap();
                let hkl = LoadKeyboardLayoutW(wide.as_ptr(), 1); // KLF_ACTIVATE = 1
                let hwnd = GetForegroundWindow();
                if hwnd.is_null() {
                    return false;
                }
                PostMessageW(hwnd, WM_INPUTLANGCHANGEREQUEST, 0 as WPARAM, hkl as LPARAM);
                return true;
            }
        }
        false
    }

    pub fn watch_loop(running: Arc<AtomicBool>) {
        println!("Windows watcher started.");
        let mut sys = System::new_all();
        let mut last_pid: Option<u32> = None;
        let mut last_req_lang: Option<String> = None;

        while running.load(Ordering::SeqCst) {
            let cfg = super::load_config();

            if let Some(pid) = get_foreground_pid() {
                if Some(pid) != last_pid {
                    sys.refresh_processes();
                    let mut proc_name_opt: Option<String> = None;
                    for proc in sys.processes().values() {
                        if proc.pid().as_u32() == pid {
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
                                if set_layout_for_pid(pid, &lang) {
                                    println!("Requested layout {} for {}", lang, proc_name);
                                    last_req_lang = Some(lang);
                                }
                            }
                        } else {
                            last_req_lang = None;
                        }
                    }
                    last_pid = Some(pid);
                }
            }
            thread::sleep(Duration::from_millis(super::POLL_MS));
        }
        println!("Windows watcher exiting.");
    }
}

// (Linux کد مشابه داره — همونی که قبلاً داشتی، حذف نکردم)

// ---------------------- Tray Commands ---------------------
#[derive(Debug)]
enum MenuCommand {
    Toggle,
    Settings,
    Quit,
}

fn is_gui_running() -> bool {
    let targets = ["lang_switcher_rust.exe", "lang_switcher_rust"];
    let mut sys = System::new_all();
    sys.refresh_processes();
    for (_pid, p) in sys.processes() {
        let name = p.name();
        for t in &targets {
            if name.eq_ignore_ascii_case(t) {
                return true;
            }
        }
    }
    false
}

// -------------------------- MAIN --------------------------
fn main() {
    println!("Layout watcher. Config: {}", CONFIG_FILE);

    let running = Arc::new(AtomicBool::new(true));
    let watcher_handle: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    // spawn watcher
    let spawn_watcher = {
        let running = Arc::clone(&running);
        let handle = Arc::clone(&watcher_handle);
        move || {
            running.store(true, Ordering::SeqCst);
            let r = Arc::clone(&running);
            let joinh = thread::spawn(move || {
                #[cfg(target_os = "windows")]
                crate::platform::watch_loop(r);
                #[cfg(target_os = "linux")]
                crate::platform::watch_loop(r);
            });
            *handle.lock().unwrap() = Some(joinh);
        }
    };
    spawn_watcher();

    // load icon
    let mut icon_path = PathBuf::from("assets/icon.ico");
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let cand = parent.join("assets").join("icon.png");
            if cand.exists() {
                icon_path = cand;
            }
        }
    }
    let dynimg = image::open(&icon_path).expect("icon not found");
    let rgba = dynimg.to_rgba8();
    let (w, h) = dynimg.dimensions();
    let icon_bytes = rgba.into_raw();

    let (tx, rx): (Sender<MenuCommand>, Receiver<MenuCommand>) = mpsc::channel();

    // tray thread
    {
        let tx = tx.clone();
        let icon_bytes_clone = icon_bytes.clone();
        thread::spawn(move || {
            let icon = Icon::from_rgba(icon_bytes_clone, w, h).unwrap();

            let mut menu = Menu::new();
            let toggle_item = MenuItem::new("Toggle watcher", true, None);
            let settings_item = MenuItem::new("Settings", true, None);
            let quit_item = MenuItem::new("Quit", true, None);

            menu.append(&toggle_item).unwrap();
            menu.append(&settings_item).unwrap();
            menu.append(&quit_item).unwrap();

            let _tray = TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip("Lang Watcher")
                .with_menu(Box::new(menu))
                .build()
                .unwrap();

            let menu_rx = MenuEvent::receiver();

            #[cfg(target_os = "windows")]
            {
                use winapi::um::winuser::{PeekMessageW, TranslateMessage, DispatchMessageW, PM_REMOVE, MSG};
                loop {
                    unsafe {
                        let mut msg: MSG = std::mem::zeroed();
                        while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }
                    while let Ok(ev) = menu_rx.try_recv() {
                        if ev.id() == &toggle_item.id() {
                            let _ = tx.send(MenuCommand::Toggle);
                        } else if ev.id() == &settings_item.id() {
                            let _ = tx.send(MenuCommand::Settings);
                        } else if ev.id() == &quit_item.id() {
                            let _ = tx.send(MenuCommand::Quit);
                        }
                    }
                    thread::sleep(Duration::from_millis(50));
                }
            }

            #[cfg(not(target_os = "windows"))]
            {
                while let Ok(ev) = menu_rx.recv() {
                    if ev.id() == &toggle_item.id() {
                        let _ = tx.send(MenuCommand::Toggle);
                    } else if ev.id() == &settings_item.id() {
                        let _ = tx.send(MenuCommand::Settings);
                    } else if ev.id() == &quit_item.id() {
                        let _ = tx.send(MenuCommand::Quit);
                    }
                }
            }
        });
    }

    // main loop
    loop {
        if let Ok(cmd) = rx.try_recv() {
            match cmd {
                MenuCommand::Toggle => {
                    if running.load(Ordering::SeqCst) {
                        running.store(false, Ordering::SeqCst);
                        if let Some(h) = watcher_handle.lock().unwrap().take() {
                            let _ = h.join();
                        }
                    } else {
                        spawn_watcher();
                    }
                }
                MenuCommand::Settings => {
                    if !is_gui_running() {
                        if let Ok(exe_path) = std::env::current_exe() {
                            if let Some(parent) = exe_path.parent() {
                                let gui = parent.join("lang_switcher_rust.exe");
                                let _ = Command::new(gui).spawn();
                            }
                        }
                    }
                }
                MenuCommand::Quit => {
                    running.store(false, Ordering::SeqCst);
                    if let Some(h) = watcher_handle.lock().unwrap().take() {
                        let _ = h.join();
                    }
                    std::process::exit(0);
                }
            }
        }
        thread::sleep(Duration::from_millis(150));
    }
}
