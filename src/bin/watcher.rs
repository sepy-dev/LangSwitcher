// src/bin/watcher.rs
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

use image::GenericImageView;
use sysinfo::{ProcessExt, PidExt, System, SystemExt};

use tray_icon::{Icon, TrayIconBuilder};
use tray_icon::menu::{Menu, MenuItem, MenuEvent};

use winapi::shared::minwindef::UINT; // only used on Windows for PeekMessage constants

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

    pub fn set_layout_for_pid(_pid: u32, lang: &str) -> bool {
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

    pub fn watch_loop(running: Arc<AtomicBool>) {
        println!("Windows watcher started.");
        let mut sys = System::new_all();
        let mut last_pid: Option<u32> = None;
        let mut last_req_lang: Option<String> = None;

        while running.load(Ordering::SeqCst) {
            let cfg = load_config();

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
                                    println!(
                                        "Requested layout {} for process {} (pid {})",
                                        lang, proc_name, pid
                                    );
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

        println!("Windows watcher exiting.");
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use std::process::Command;

    fn get_active_window_pid() -> Option<u32> {
        let out = Command::new("xdotool").arg("getactivewindow").output();
        if let Ok(out) = out {
            if !out.status.success() {
                return None;
            }
            let winid = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if winid.is_empty() {
                return None;
            }
            let out2 = Command::new("xdotool").arg("getwindowpid").arg(&winid).output();
            if let Ok(out2) = out2 {
                if !out2.status.success() {
                    return None;
                }
                let pid_str = String::from_utf8_lossy(&out2.stdout).trim().to_string();
                if let Ok(pid) = pid_str.parse::<u32>() {
                    return Some(pid);
                }
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
            Err(e) => {
                eprintln!("Failed to call setxkbmap: {}", e);
                false
            }
        }
    }

    pub fn watch_loop(running: Arc<AtomicBool>) {
        println!("Linux watcher started (requires xdotool & setxkbmap).");
        let mut sys = System::new_all();
        let mut last_pid: Option<u32> = None;
        let mut last_req_lang: Option<String> = None;

        while running.load(Ordering::SeqCst) {
            let cfg = load_config();

            if let Some(pid) = get_active_window_pid() {
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

        println!("Linux watcher exiting.");
    }
}

// commands from tray -> main
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

fn main() {
    println!("Layout watcher (Rust). Reading config from '{}'", CONFIG_FILE);
    if !Path::new(CONFIG_FILE).exists() {
        eprintln!("Warning: {} not found. Create it through GUI or manually.", CONFIG_FILE);
    }

    // running state + handle
    let running = Arc::new(AtomicBool::new(true));
    let watcher_handle: Arc<Mutex<Option<JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    // spawn helper
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

    // start
    spawn_watcher();

    // --- icon path
    let mut icon_path = PathBuf::from("assets/icon.ico");
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let cand = parent.join("assets").join("icon.png");
            if cand.exists() {
                icon_path = cand;
            } else {
                let cand2 = parent.join("icon.png");
                if cand2.exists() {
                    icon_path = cand2;
                }
            }
        }
    }

    let dynimg = image::open(&icon_path).expect(&format!("icon not found: {:?}", icon_path));
    let rgba = dynimg.to_rgba8();
    let (w, h) = dynimg.dimensions();
    let icon_bytes = rgba.into_raw();

    // channel
    let (tx, rx): (Sender<MenuCommand>, Receiver<MenuCommand>) = mpsc::channel();

    // spawn tray thread: build menu and process its events here (no 'set_event_handler')
    {
        let tx = tx.clone();
        let icon_bytes_clone = icon_bytes.clone();
        thread::spawn(move || {
            // icon
            let icon = Icon::from_rgba(icon_bytes_clone, w, h).expect("failed to build icon");

            // menu + items (live in this thread)
            let mut menu = Menu::new();
            let toggle_item = MenuItem::new("Toggle watcher", true, None);
            let settings_item = MenuItem::new("Settings", true, None);
            let quit_item = MenuItem::new("Quit", true, None);

            menu.append(&toggle_item).expect("append toggle");
            menu.append(&settings_item).expect("append settings");
            menu.append(&quit_item).expect("append quit");

            // build tray
            let _tray = TrayIconBuilder::new()
                .with_icon(icon)
                .with_tooltip("Lang Watcher")
                .with_menu(Box::new(menu))
                .build()
                .expect("failed to build tray");

            // receiver for menu events
            let menu_rx = MenuEvent::receiver();

            // On Windows: we need to pump Win32 messages so the context menu appears.
            // We'll loop: PeekMessage/DispatchMessage + then drain menu_rx.try_recv().
            #[cfg(target_os = "windows")]
            {
                use winapi::um::winuser::{PeekMessageW, TranslateMessage, DispatchMessageW, PM_REMOVE, MSG};

                loop {
                    // pump windows messages (non-blocking)
                    unsafe {
                        let mut msg: MSG = std::mem::zeroed();
                        while PeekMessageW(&mut msg as *mut MSG, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                            TranslateMessage(&msg);
                            DispatchMessageW(&msg);
                        }
                    }

                    // process any menu events available
                    while let Ok(menu_ev) = menu_rx.try_recv() {
                        let id = menu_ev.id();
                        if id == &toggle_item.id() {
                            let _ = tx.send(MenuCommand::Toggle);
                        } else if id == &settings_item.id() {
                            let _ = tx.send(MenuCommand::Settings);
                        } else if id == &quit_item.id() {
                            let _ = tx.send(MenuCommand::Quit);
                        }
                    }

                    // small sleep to avoid busy loop
                    std::thread::sleep(Duration::from_millis(50));
                }
            }

            // On non-Windows: just block on menu_rx.recv() and forward
            #[cfg(not(target_os = "windows"))]
            {
                while let Ok(menu_ev) = menu_rx.recv() {
                    let id = menu_ev.id();
                    if id == &toggle_item.id() {
                        let _ = tx.send(MenuCommand::Toggle);
                    } else if id == &settings_item.id() {
                        let _ = tx.send(MenuCommand::Settings);
                    } else if id == &quit_item.id() {
                        let _ = tx.send(MenuCommand::Quit);
                    }
                }
            }
        });
    }

    // main polling loop: process MenuCommand messages
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
                                if let Err(_e) = Command::new(&gui).spawn() {
                                    let gui2 = parent.join("lang_switcher_rust");
                                    if let Err(e2) = Command::new(&gui2).spawn() {
                                        eprintln!("Failed to spawn GUI {:?} and fallback {:?}: {:?}", gui, gui2, e2);
                                    }
                                }
                            }
                        }
                    } else {
                        println!("Settings already running");
                    }
                }
                MenuCommand::Quit => {
                    running.store(false, Ordering::SeqCst);
                    if let Some(h) = watcher_handle.lock().unwrap().take() {
                        let _ = h.join();
                    }
                    println!("Exiting watcher as requested by Quit menu.");
                    std::process::exit(0);
                }
            }
        }

        std::thread::sleep(Duration::from_millis(150));
    }
}

