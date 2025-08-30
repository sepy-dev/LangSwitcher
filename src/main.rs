// src/main.rs
mod core;
use core::LangState;

use eframe::egui;
use egui::{ColorImage, TextureHandle};
use image;
use std::path::PathBuf;
use std::process::{Child, Command};

use sysinfo::{ProcessExt, System, SystemExt};

const ICON_FOLDER: &str = "icons";
const DEFAULT_ICON_NAME: &str = "default.png";
// src/main.rs


#[cfg(target_os = "windows")]
fn extract_icon_rgba_from_exe(exe: &PathBuf, size: i32) -> Option<(Vec<u8>, u32, u32)> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::null_mut;
    use std::mem::{size_of, zeroed};

    use winapi::ctypes::c_void;
    use winapi::shared::windef::{HICON, HBITMAP, HDC};
    use winapi::um::wingdi::{
        BI_RGB, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, BITMAPINFO,
        BITMAPINFOHEADER, DIB_RGB_COLORS,
    };
    use winapi::um::winuser::{DrawIconEx, DestroyIcon};
    use winapi::um::shellapi::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};

    // DI_NORMAL موجود نیسـت تو winapi نسخه‌ی تو — مقدار ثابت را خودمان تعریف می‌کنیم:
    const DI_NORMAL: u32 = 0x0003;

    // convert path to wide str
    let mut wide: Vec<u16> = OsStr::new(&exe.as_os_str())
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        let mut shfi: SHFILEINFOW = zeroed();
        let flags = SHGFI_ICON | SHGFI_LARGEICON;
        let res = SHGetFileInfoW(
            wide.as_ptr(),
            0,
            &mut shfi as *mut SHFILEINFOW,
            size_of::<SHFILEINFOW>() as u32,
            flags,
        );
        if res == 0 || shfi.hIcon.is_null() {
            return None;
        }
        let hicon: HICON = shfi.hIcon;

        // Create compatible DC
        let hdc: HDC = CreateCompatibleDC(null_mut());
        if hdc.is_null() {
            DestroyIcon(hicon);
            return None;
        }

        // Prepare DIB (32-bit, top-down)
        let width = size;
        let height = size;
        let mut bmi: BITMAPINFO = zeroed();
        bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = width;
        bmi.bmiHeader.biHeight = -height; // top-down
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB;

        let mut pixels_ptr: *mut c_void = null_mut();
        let hbitmap: HBITMAP = CreateDIBSection(
            hdc,
            &mut bmi,
            DIB_RGB_COLORS,
            &mut pixels_ptr,
            null_mut(),
            0,
        );
        if hbitmap.is_null() || pixels_ptr.is_null() {
            DeleteDC(hdc);
            DestroyIcon(hicon);
            return None;
        }

        // select bitmap and draw icon into it
        let old = SelectObject(hdc, hbitmap as _);
        // استفاده از DI_NORMAL که ما تعریفش کردیم
        let ok = DrawIconEx(hdc, 0, 0, hicon, width, height, 0, null_mut(), DI_NORMAL);
        if ok == 0 {
            // cleanup
            SelectObject(hdc, old);
            DeleteObject(hbitmap as _);
            DeleteDC(hdc);
            DestroyIcon(hicon);
            return None;
        }

        // read pixels: BGRA -> convert to RGBA
        let byte_count = (width * height * 4) as usize;
        let src_slice = std::slice::from_raw_parts(pixels_ptr as *const u8, byte_count);
        let mut rgba = Vec::with_capacity(byte_count);
        for chunk in src_slice.chunks_exact(4) {
            // chunk = [B, G, R, A]
            rgba.push(chunk[2]); // R
            rgba.push(chunk[1]); // G
            rgba.push(chunk[0]); // B
            rgba.push(chunk[3]); // A
        }

        // cleanup
        SelectObject(hdc, old);
        DeleteObject(hbitmap as _);
        DeleteDC(hdc);
        DestroyIcon(hicon);

        Some((rgba, width as u32, height as u32))
    }
}


#[cfg(not(target_os = "windows"))]
fn extract_icon_rgba_from_exe(_exe: &PathBuf, _size: i32) -> Option<(Vec<u8>, u32, u32)> {
    // برای غیر ویندوز فعلاً نَمود نداریم — برگرد None تا fallback استفاده شود
    None
}

/// load texture for program: 1) try mapped icons in icons/ 2) try extract from exe (windows) 3) fallback default
fn load_icon_texture(ctx: &egui::Context, texture_id: &str, prog_name: &str, exe_path: Option<&PathBuf>) -> Option<TextureHandle> {
    // known-file mapping first
    let map = vec![
        ("chrome.exe", "chrome.png"),
        ("firefox.exe", "firefox.png"),
        ("Code.exe", "code.png"),
        ("PyCharm.exe", "pycharm.png"),
        ("Opera.exe", "opera.png"),
    ];

    if let Some(fname) = map.iter().find(|(k, _)| k.eq(&prog_name)).map(|(_, v)| *v) {
        let path = PathBuf::from(ICON_FOLDER).join(fname);
        if path.exists() {
            if let Ok(img) = image::open(&path) {
                let img = img.to_rgba8();
                let (w, h) = img.dimensions();
                let pixels: Vec<u8> = img.into_raw();
                let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
                return Some(ctx.load_texture(texture_id, color_image, egui::TextureOptions::default()));
            }
        }
    }

    // try extract from exe (windows)
    if let Some(exe) = exe_path {
        if let Some((rgba, w, h)) = extract_icon_rgba_from_exe(exe, 64) {
            let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
            return Some(ctx.load_texture(texture_id, color_image, egui::TextureOptions::default()));
        }

        // also try some image files near exe (icon.png / exe_stem.png)
        if let Some(parent) = exe.parent() {
            let exe_stem = exe.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            let candidates = vec![
                parent.join("icon.png"),
                parent.join("icon.ico"),
                parent.join(format!("{}.png", exe_stem)),
                parent.join(format!("{}.ico", exe_stem)),
            ];
            for cand in candidates {
                if cand.exists() {
                    if let Ok(img) = image::open(&cand) {
                        let img = img.to_rgba8();
                        let (w, h) = img.dimensions();
                        let pixels: Vec<u8> = img.into_raw();
                        let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
                        return Some(ctx.load_texture(texture_id, color_image, egui::TextureOptions::default()));
                    }
                }
            }
        }
    }

    // fallback default in icons/
    let default_path = PathBuf::from(ICON_FOLDER).join(DEFAULT_ICON_NAME);
    if default_path.exists() {
        if let Ok(img) = image::open(default_path) {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            let pixels: Vec<u8> = img.into_raw();
            let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
            return Some(ctx.load_texture(texture_id, color_image, egui::TextureOptions::default()));
        }
    }

    None
}

// remainder of UI (draw_toggle + LangApp etc.) — همان کد قبلی با تغییرات لازم برای فراخوانی load_icon_texture
// برای خلاصه بودن، ادامه فایل را همان‌طور که پیشتر نهایی شده بودی نگه دار.
// فقط دقت کن که هنگام ساخت textures از exe_path استفاده میشود، مثلاً:
//
// let textures: Vec<Option<TextureHandle>> = st
//     .programs
//     .iter()
//     .enumerate()
//     .map(|(i, p)| {
//         let tid = format!("icon-{}", i);
//         load_icon_texture(&cc.egui_ctx, &tid, &p.name, p.exe_path.as_ref())
//     })
//     .collect();
//
// و همچنین در Refresh همان الگو استفاده شود.


fn draw_toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(56.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let rounding = rect.height() / 2.0;
    let track_color = if *on {
        egui::Color32::from_rgb(100, 170, 255)
    } else {
        egui::Color32::from_gray(200)
    };
    ui.painter().rect(rect, rounding, track_color, egui::Stroke::NONE);
    let pad = 4.0;
    let inner = rect.shrink2(egui::Vec2::splat(pad));
    let thumb_radius = inner.height() / 2.0 - 1.0;
    let cx = if *on { inner.right() - thumb_radius } else { inner.left() + thumb_radius };
    let thumb_center = egui::pos2(cx, inner.center().y);
    let shadow_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 28);
    ui.painter().circle(thumb_center + egui::Vec2::new(0.0, 1.0), thumb_radius + 0.6, shadow_color, egui::Stroke::NONE);
    ui.painter().circle(thumb_center, thumb_radius, egui::Color32::WHITE, egui::Stroke::new(1.0, egui::Color32::from_gray(200)));
    if resp.clicked() {
        *on = !*on;
    }
    resp
}

struct LangApp {
    state: LangState,
    textures: Vec<Option<TextureHandle>>,
    watcher: Option<Child>,
    watcher_enabled: bool,
}

impl LangApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let st = LangState::new();
        let textures = st.programs.iter()
            .enumerate()
            .map(|(i, p)| {
                let tid = format!("icon-{}", i);
                load_icon_texture(&cc.egui_ctx, &tid, &p.name, p.exe_path.as_ref())
            })
            .collect();

        let mut app = Self { state: st, textures, watcher: None, watcher_enabled: true };
        if app.watcher_enabled {
            app.start_watcher();
        }
        app
    }

    fn is_watcher_running_process(&self) -> bool {
        let mut sys = System::new_all();
        sys.refresh_processes();
        for (_pid, proc_) in sys.processes() {
            let name = proc_.name().to_string();
            if name.eq_ignore_ascii_case("watcher.exe") || name.eq_ignore_ascii_case("watcher") {
                return true;
            }
        }
        false
    }

    fn start_watcher(&mut self) {
        if self.watcher.is_some() { return; }
        if self.is_watcher_running_process() {
            println!("watcher already running (external), not spawning a new one.");
            return;
        }
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let watcher_name = if cfg!(windows) { "watcher.exe" } else { "watcher" };
                let candidates = vec![
                    parent.join("bin").join(watcher_name),
                    parent.join(watcher_name),
                    parent.join("..").join("target").join("debug").join(watcher_name),
                    parent.join("..").join("target").join("release").join(watcher_name),
                ];
                for cand in candidates {
                    if cand.exists() {
                        match Command::new(&cand).spawn() {
                            Ok(child) => { self.watcher = Some(child); println!("started watcher from {:?}", cand); return; }
                            Err(e) => { eprintln!("failed to spawn watcher {:?}: {}", cand, e); }
                        }
                    }
                }
                println!("watcher binary not found in candidates.");
            }
        } else {
            eprintln!("could not determine current exe path to locate watcher binary.");
        }
    }

    fn stop_watcher(&mut self) {
        if let Some(mut child) = self.watcher.take() {
            if let Err(e) = child.kill() { eprintln!("failed to kill spawned watcher: {}", e); }
            let _ = child.wait();
            println!("killed spawned watcher (if any).");
            return;
        }
        if self.is_watcher_running_process() {
            if cfg!(windows) {
                let _ = Command::new("taskkill").args(&["/IM", "watcher.exe", "/F"]).spawn();
            } else {
                let _ = Command::new("pkill").arg("-f").arg("watcher").spawn();
            }
            println!("attempted to kill watcher processes by name (best-effort).");
        } else {
            println!("no watcher process found to kill.");
        }
    }
}

impl eframe::App for LangApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| { ui.heading("Language Switcher — UI (Polished)"); });
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui.button("Refresh (scan processes)").clicked() {
                    self.state.refresh();
                    self.textures = self.state.programs.iter()
                        .enumerate()
                        .map(|(i, p)| {
                            let tid = format!("icon-{}", i);
                            load_icon_texture(ctx, &tid, &p.name, p.exe_path.as_ref())
                        })
                        .collect();
                }
                if ui.button("Save Now").clicked() {
                    if let Err(e) = self.state.save_config() { ui.label(format!("Save error: {}", e)); } else { ui.label("Saved."); }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.label("UI polished — toggle to change language"); });
            });

            ui.separator();
            ui.add_space(8.0);

            // Scrollable area for program list — prevents bottom panel from covering items and improves scrolling perf
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let mut changed_any = false;
                let len = self.state.programs.len();
                for idx in 0..len {
                    ui.vertical(|ui| {
                        ui.add_space(4.0);

                        let prog = &mut self.state.programs[idx];
                        let tex_opt = &self.textures[idx];

                        // display name: prefer exe stem if available
                        let display_name = if let Some(exe) = &prog.exe_path {
                            exe.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()).unwrap_or_else(|| prog.name.clone())
                        } else {
                            prog.name.clone()
                        };

                        let border_color = if prog.lang == "en" { egui::Color32::from_rgb(80,150,230) } else { egui::Color32::from_rgb(44,180,100) };
                        let card_bg = egui::Color32::from_rgb(250,250,250);
                        let available_width = ui.available_width();
                        let card_size = egui::vec2(available_width, 64.0);
                        let (card_rect, card_resp) = ui.allocate_exact_size(card_size, egui::Sense::hover());
                        let thickness = if card_resp.hovered() { 2.6 } else { 1.2 };
                        ui.painter().rect(card_rect.shrink(2.0), 8.0, card_bg, egui::Stroke::new(thickness, border_color));

                        let mut content_ui = ui.child_ui(card_rect.shrink2(egui::Vec2::splat(8.0)), egui::Layout::left_to_right(egui::Align::Center));
                        content_ui.horizontal(|ui| {
                            if let Some(tex) = tex_opt {
                                ui.add(egui::Image::new((tex.id(), egui::vec2(44.0,44.0))));
                            } else {
                                ui.add_space(48.0);
                            }

                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(display_name).size(15.0).strong());
                                if let Some(exe) = &prog.exe_path {
                                    if let Some(s) = exe.to_str() { ui.label(egui::RichText::new(s).color(egui::Color32::from_gray(110)).size(11.0)); }
                                } else {
                                    ui.label(egui::RichText::new("No exe path").color(egui::Color32::from_gray(110)).size(11.0));
                                }
                            });

                            ui.add_space(8.0);

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let pill = if prog.lang == "en" { "EN" } else { "FA" };
                                let pill_color = if prog.lang == "en" { egui::Color32::from_rgb(30,100,170) } else { egui::Color32::from_rgb(20,120,60) };
                                ui.colored_label(pill_color, pill);
                                ui.add_space(8.0);

                                let mut local_on = prog.lang == "en";
                                let resp = draw_toggle(ui, &mut local_on);
                                if resp.clicked() || resp.double_clicked() {
                                    prog.lang = if local_on { "en".to_string() } else { "fa".to_string() };
                                    changed_any = true;
                                }
                            });
                        });

                        ui.add_space(6.0);
                    });
                }

                if changed_any {
                    if let Err(e) = self.state.save_config() { ui.label(format!("Save error: {}", e)); }
                }
            }); // end ScrollArea
        });

        // bottom panel (fixed)
        egui::TopBottomPanel::bottom("watcher_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(8.0);

            ui.horizontal_centered(|ui| {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut local_on = self.watcher_enabled;
                        let resp = draw_toggle(ui, &mut local_on);
                        if resp.clicked() {
                            self.watcher_enabled = local_on;
                            if self.watcher_enabled { self.start_watcher(); } else { self.stop_watcher(); }
                        }

                        ui.add_space(8.0);
                        let running = self.watcher.is_some() || self.is_watcher_running_process();
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("Watcher").strong());
                            ui.label(egui::RichText::new(if running { "running" } else { "stopped" }).small());
                        });
                    });
                });
            });

            ui.add_space(6.0);
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("Lang Switcher (Rust) — Polished UI", native_options, Box::new(|cc| Box::new(LangApp::new(cc))));
}
