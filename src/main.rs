// src/main.rs
#![windows_subsystem = "windows"]

mod core;
use core::LangState;

use eframe::egui;
use egui::{ColorImage, TextureHandle, RichText};
use image;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

use sysinfo::{ProcessExt, System, SystemExt};

use std::os::windows::process::CommandExt; // فقط ویندوز
use std::sync::Arc;

const CREATE_NO_WINDOW: u32 = 0x08000000;

const ICON_FOLDER: &str = "icons";
const DEFAULT_ICON_NAME: &str = "default.png";

/// بارگذاری تکسچر آیکون — مسیرها نسبت به مسیر فایل اجرایی مشخص می‌شوند
fn load_icon_texture(ctx: &egui::Context, texture_id: &str, prog_name: &str) -> Option<TextureHandle> {
    // map exe -> pre-bundled png
    let map = vec![
        ("chrome.exe", "chrome.png"),
        ("firefox.exe", "firefox.png"),
        ("Code.exe", "code.png"),
        ("PyCharm.exe", "pycharm.png"),
        ("Opera.exe", "opera.png"),
    ];

    // base dir = executable directory (important for shortcuts/installers)
    let exe_base = std::env::current_exe().ok().and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // try known mapping inside exe_dir/icons
    if let Some(fname) = map.iter().find(|(k, _)| k.eq(&prog_name)).map(|(_, v)| *v) {
        let path = exe_base.join(ICON_FOLDER).join(fname);
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

    // fallback: default icon inside exe_dir/icons/default.png
    let default_path = exe_base.join(ICON_FOLDER).join(DEFAULT_ICON_NAME);
    if default_path.exists() {
        if let Ok(img) = image::open(&default_path) {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            let pixels: Vec<u8> = img.into_raw();
            let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
            return Some(ctx.load_texture(texture_id, color_image, egui::TextureOptions::default()));
        }
    }

    // نه mapping نه default یافت شد
    None
}

fn draw_toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(56.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let rounding = rect.height() / 2.0;

    let track_color = if *on {
        egui::Color32::from_rgb(130, 90, 255)
    } else {
        egui::Color32::from_gray(50)
    };

    ui.painter().rect(rect, rounding, track_color, egui::Stroke::NONE);

    let pad = 4.0;
    let inner = rect.shrink2(egui::Vec2::splat(pad));
    let thumb_radius = inner.height() / 2.0 - 1.0;
    let cx = if *on { inner.right() - thumb_radius } else { inner.left() + thumb_radius };
    let thumb_center = egui::pos2(cx, inner.center().y);

    let shadow_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 48);
    ui.painter().circle(thumb_center + egui::Vec2::new(0.0, 1.0), thumb_radius + 0.6, shadow_color, egui::Stroke::NONE);

    ui.painter().circle(
        thumb_center,
        thumb_radius,
        egui::Color32::from_rgb(240, 240, 245),
        egui::Stroke::new(1.0, egui::Color32::from_gray(100)),
    );

    if resp.clicked() {
        *on = !*on;
    }

    resp
}

fn apply_cyberpunk_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.panel_fill = egui::Color32::from_rgb(12, 10, 25);
    style.visuals.window_fill = egui::Color32::from_rgb(14, 12, 30);
    style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(18, 16, 36);
    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(30, 24, 60);
    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(90, 60, 200);
    style.visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(160, 160, 255);
    style.visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(255, 220, 240);
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(200, 200, 255));
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.visuals.window_rounding = 8.0.into();
    style.visuals.menu_rounding = 8.0.into();
    ctx.set_style(style);
}

struct LangApp {
    state: LangState,
    textures: Vec<Option<TextureHandle>>,
    watcher: Option<Child>,   // اگر خود اپ spawn کرد نگه داشته میشه
    watcher_enabled: bool,    // نشان‌دهنده وضعیت دکمه (on/off)
}

impl LangApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        apply_cyberpunk_theme(&cc.egui_ctx);

        let st = LangState::new();
        let textures = st.programs.iter().enumerate().map(|(i, p)| {
            let tid = format!("icon-{}", i);
            load_icon_texture(&cc.egui_ctx, &tid, &p.name)
        }).collect();

        // default: watcher should NOT auto-start.
        // اگر watcher قبلاً توسط کاربر یا تری اجرا شده بود، دکمه را روشن کن (ولی spawn نکن)
        let mut app = Self { state: st, textures, watcher: None, watcher_enabled: false };
        if app.is_watcher_running() {
            app.watcher_enabled = true;
        }

        app
    }

    /// آیا watcher در سیستم (حتی اگر توسط ما spawn نشده) در حال اجراست؟
    fn is_watcher_running(&self) -> bool {
        let mut sys = System::new_all();
        sys.refresh_processes();
        sys.processes().values().any(|p| {
            let name = p.name();
            name.eq_ignore_ascii_case("watcher.exe") || name.eq_ignore_ascii_case("watcher")
        })
    }

    /// spawn watcher (فقط وقتی که لازم باشه و ما مسئول اون هستیم)
    fn start_watcher(&mut self) {
        // اگر قبلاً یکی رو خودمون ساختیم، کاری نکن
        if self.watcher.is_some() {
            return;
        }

        // اگر watcher از قبل در سیستم اجراست، ما نباید spawn کنیم — فقط حالت را روشن نگه می‌داریم
        if self.is_watcher_running() {
            println!("watcher already running externally — not spawning.");
            self.watcher = None;
            return;
        }

        // مسیرهای کاندید برای watcher نسبت به exe
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let watcher_name = if cfg!(windows) { "watcher.exe" } else { "watcher" };
                let candidates = vec![
                    parent.join(&watcher_name),
                    parent.join("target").join("release").join(&watcher_name),
                    parent.join("target").join("debug").join(&watcher_name),
                ];

                for cand in candidates {
                    if cand.exists() {
                        println!("Spawning watcher from {:?}", cand);
                        match Command::new(&cand)
                            .creation_flags(CREATE_NO_WINDOW) // نذاریم کنسول باز بشه
                            .spawn() {
                            Ok(child) => {
                                self.watcher = Some(child);
                                println!("Watcher started (spawned by GUI).");
                                return;
                            }
                            Err(e) => {
                                eprintln!("Failed to spawn watcher {:?}: {}", cand, e);
                                // برو بعدی
                            }
                        }
                    } else {
                        println!("Candidate not found: {:?}", cand);
                    }
                }
            }
        } else {
            eprintln!("Could not determine current exe path to locate watcher binary.");
        }

        eprintln!("No watcher binary started.");
    }

    /// stop watcher: اگر ما spawn کرده بودیم kill کنیم، در غیر این صورت تلاش best-effort برای taskkill
    fn stop_watcher(&mut self) {
        if let Some(mut child) = self.watcher.take() {
            if let Err(e) = child.kill() { eprintln!("failed to kill spawned watcher: {}", e); }
            let _ = child.wait();
            println!("killed spawned watcher (we created it).");
            return;
        }

        // اگر watcher توسط خارجی اجرا شده بود، سعی کن با taskkill/ pkill ببندی
        if self.is_watcher_running() {
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
        // bottom panel: watcher toggle
        egui::TopBottomPanel::bottom("watcher_panel").resizable(false).min_height(70.0).show(ctx, |ui| {
            ui.add_space(6.0);
            ui.separator();

            ui.horizontal_centered(|ui| {
                egui::Frame::none().inner_margin(egui::style::Margin::same(8.0)).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // local toggle reflects watcher_enabled
                        let mut local_on = self.watcher_enabled;
                        let resp = draw_toggle(ui, &mut local_on);

                        // وقتی کلیک شد:
                        if resp.clicked() {
                            // apply change to state
                            self.watcher_enabled = local_on;
                            if self.watcher_enabled {
                                // فقط اگر غیرفعال بود، اجرا کن
                                if !self.is_watcher_running() {
                                    self.start_watcher();
                                } else {
                                    println!("watcher already running externally; not spawning.");
                                }
                            } else {
                                // اگر خاموش شد: تلاش برای متوقف کردن (اگر ما ساخته باشیم kill کن، در غیر این صورت taskkill)
                                self.stop_watcher();
                            }
                        }

                        ui.add_space(10.0);
                        let running = self.is_watcher_running() || self.watcher.is_some();
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Watcher").strong().color(egui::Color32::from_rgb(190,170,255)));
                            ui.label(RichText::new(if running { "running" } else { "stopped" }).small());
                        });

                        ui.add_space(24.0);

                        ui.horizontal(|ui| {
                            let gh = ui.add_sized([90.0, 26.0], egui::Button::new(RichText::new("GitHub").underline().size(14.0)));
                            if gh.clicked() { let _ = open::that("https://github.com/sepy-dev"); }
                            ui.add_space(8.0);
                            let tw = ui.add_sized([90.0, 26.0], egui::Button::new(RichText::new("Twitter/X").underline().size(14.0)));
                            if tw.clicked() { let _ = open::that("https://x.com/Sepy_dev"); }
                        });
                    });
                });
            });

            ui.add_space(6.0);
        });

        // central UI (لیست برنامه‌ها ...)
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("Language Switcher").size(22.0).color(egui::Color32::from_rgb(200,180,255)));
                ui.label(RichText::new("Polished — cyberpunk style").small().color(egui::Color32::from_rgb(170,170,255)));
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Refresh (scan processes)").clicked() {
                    self.state.refresh();
                    self.textures = self.state.programs.iter().enumerate().map(|(i, p)| {
                        let tid = format!("icon-{}", i);
                        load_icon_texture(ctx, &tid, &p.name)
                    }).collect();
                }
                if ui.button("Save Now").clicked() {
                    if let Err(e) = self.state.save_config() { ui.label(format!("Save error: {}", e)); } else { ui.label("Saved."); }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("toggle each program to set language").small().color(egui::Color32::from_gray(160)));
                });
            });

            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let mut changed_any = false;
                for (idx, prog) in self.state.programs.iter_mut().enumerate() {
                    ui.add_space(6.0);
                    let available_width = ui.available_width();
                    let card_size = egui::vec2(available_width, 72.0);
                    let (card_rect, card_resp) = ui.allocate_exact_size(card_size, egui::Sense::hover());
                    let border_color = if prog.lang == "en" { egui::Color32::from_rgb(100,150,255) } else { egui::Color32::from_rgb(80,220,140) };
                    let card_bg = egui::Color32::from_rgb(10,8,22);

                    ui.painter().rect(card_rect.shrink(2.0), 8.0, card_bg, egui::Stroke::new(if card_resp.hovered() { 2.6 } else { 1.2 }, border_color));

                    let mut content_ui = ui.child_ui(card_rect.shrink2(egui::Vec2::splat(8.0)), egui::Layout::left_to_right(egui::Align::Center));
                    content_ui.horizontal(|ui| {
                        let tex_opt = self.textures.get(idx).and_then(|t| t.as_ref());
                        if let Some(tex) = tex_opt {
                            ui.add(egui::Image::new((tex.id(), egui::vec2(48.0, 48.0))));
                        } else {
                            let rect = ui.allocate_exact_size(egui::vec2(48.0, 48.0), egui::Sense::hover()).0;
                            ui.painter().rect_filled(rect, 6.0, egui::Color32::from_rgb(30, 24, 60));
                            let letter = prog.name.chars().next().map(|c| c.to_string()).unwrap_or("?".to_string());
                            ui.painter().text(rect.center_top() + egui::vec2(0.0, 12.0), egui::Align2::CENTER_CENTER, letter, egui::FontId::proportional(18.0), egui::Color32::from_rgb(200,200,255));
                        }

                        ui.add_space(8.0);

                        ui.vertical(|ui| {
                            ui.label(RichText::new(&prog.name).size(15.0).strong());
                            ui.label(RichText::new("Click toggle to set language").small().color(egui::Color32::from_gray(140)));
                        });

                        ui.add_space(8.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let pill = if prog.lang == "en" { "EN" } else { "FA" };
                            let pill_color = if prog.lang == "en" { egui::Color32::from_rgb(50,110,190) } else { egui::Color32::from_rgb(40,150,70) };
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
                }

                if changed_any {
                    if let Err(e) = self.state.save_config() { eprintln!("Save error: {}", e); }
                }
            });
        });
    }
}

fn main() {
    // logger برای debug/ info
    env_logger::init();

    let native_options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Lang Switcher (Rust) — Polished UI",
        native_options,
        Box::new(|cc| Box::new(LangApp::new(cc))),
    );
}
