// src/main.rs
mod core;
use core::LangState;

use eframe::egui;
use egui::{ColorImage, TextureHandle};
use image;
use std::path::PathBuf;
use std::process::{Child, Command};

const ICON_FOLDER: &str = "icons";
const DEFAULT_ICON_NAME: &str = "default.png";

fn load_icon_texture(ctx: &egui::Context, prog_name: &str) -> Option<TextureHandle> {
    // نگاشت exe -> filename داخل فولدر icons
    let map = vec![
        ("chrome.exe", "chrome.png"),
        ("firefox.exe", "firefox.png"),
        ("Code.exe", "code.png"),
        ("PyCharm.exe", "pycharm.png"),
        ("Opera.exe", "opera.png"),
    ];
    let filename = map.iter().find(|(k, _)| k.eq(&prog_name)).map(|(_, v)| *v);
    let path = filename
        .map(|n| PathBuf::from(ICON_FOLDER).join(n))
        .unwrap_or_else(|| PathBuf::from(ICON_FOLDER).join(DEFAULT_ICON_NAME));

    if path.exists() {
        if let Ok(img) = image::open(path) {
            let img = img.to_rgba8();
            let (w, h) = img.dimensions();
            let pixels: Vec<u8> = img.into_raw();
            let color_image = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels);
            return Some(ctx.load_texture(prog_name, color_image, egui::TextureOptions::default()));
        }
    }
    None
}

/// رسم یک toggle مدرن که با کلیک حالت را جابه‌جا می‌کند.
/// - اندازه ثابت است (عرض 56, ارتفاع 28)
fn draw_toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(56.0, 28.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let rounding = rect.height() / 2.0;

    // رنگ پس‌زمینه track
    let track_color = if *on {
        egui::Color32::from_rgb(100, 170, 255) // آبی روشن برای on
    } else {
        egui::Color32::from_gray(200) // خاکستری برای off
    };

    // draw track
    ui.painter().rect(rect, rounding, track_color, egui::Stroke::NONE);

    // داخلی کوچکتر برای padding
    let pad = 4.0;
    let inner = rect.shrink2(egui::Vec2::splat(pad));

    // موقعیت thumb
    let thumb_radius = inner.height() / 2.0 - 1.0;
    let cx = if *on {
        inner.right() - thumb_radius
    } else {
        inner.left() + thumb_radius
    };
    let thumb_center = egui::pos2(cx, inner.center().y);

    // shadow for thumb (subtle)
    let shadow_color = egui::Color32::from_rgba_unmultiplied(0, 0, 0, 28);
    ui.painter().circle(thumb_center + egui::Vec2::new(0.0, 1.0), thumb_radius + 0.6, shadow_color, egui::Stroke::NONE);

    // draw thumb
    ui.painter().circle(
        thumb_center,
        thumb_radius,
        egui::Color32::WHITE,
        egui::Stroke::new(1.0, egui::Color32::from_gray(200)),
    );

    // کلیک => toggle
    if resp.clicked() {
        *on = !*on;
    }

    resp
}

struct LangApp {
    state: LangState,
    textures: Vec<Option<TextureHandle>>, // parallel to state.programs
    watcher: Option<Child>, // handle to the spawned watcher binary
}

impl LangApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // load state
        let st = LangState::new();
        // load textures for each program
        let textures = st.programs.iter()
            .map(|p| load_icon_texture(&cc.egui_ctx, &p.name))
            .collect();
        Self { state: st, textures, watcher: None }
    }

    /// try to locate bin/watcher[.exe] next to the running exe and spawn it
    fn start_watcher(&mut self) {
        if self.watcher.is_some() {
            // already running (or we have a handle)
            return;
        }

        // probe for watcher binary next to current exe: <exe_dir>/bin/watcher(.exe)
        if let Ok(mut exe_path) = std::env::current_exe() {
            if let Some(parent) = exe_path.parent() {
                let bin_dir = parent;
                let watcher_name = if cfg!(windows) { "watcher.exe" } else { "watcher" };
                let watcher_path = bin_dir.join(watcher_name);

                if watcher_path.exists() {
                    match Command::new(watcher_path).spawn() {
                        Ok(child) => {
                            self.watcher = Some(child);
                            println!("started watcher");
                        }
                        Err(e) => {
                            eprintln!("failed to spawn watcher: {}", e);
                        }
                    }
                } else {
                    println!("watcher binary not found at expected path: {:?}", bin_dir);
                }
            }
        }
    }

    /// kill the watcher process (best-effort)
    fn stop_watcher(&mut self) {
        if let Some(mut child) = self.watcher.take() {
            if let Err(e) = child.kill() {
                eprintln!("failed to kill watcher: {}", e);
            }
            let _ = child.wait();
        }
    }
}

impl eframe::App for LangApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Language Switcher — UI (Polished)");
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui.button("Refresh (scan processes)").clicked() {
                    self.state.refresh();
                    // reload textures
                    self.textures = self.state.programs.iter()
                        .map(|p| load_icon_texture(ctx, &p.name))
                        .collect();
                }
                if ui.button("Save Now").clicked() {
                    if let Err(e) = self.state.save_config() {
                        ui.label(format!("Save error: {}", e));
                    } else {
                        ui.label("Saved.");
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("UI polished — toggle to change language");
                });
            });

            ui.separator();
            ui.add_space(8.0);

            // track if need to save at end
            let mut changed_any = false;

            // render each program as a card; use index loop to avoid borrow conflicts
            let len = self.state.programs.len();
            for idx in 0..len {
                // get mutable reference to program in a short scope
                ui.vertical(|ui| {
                    ui.add_space(4.0);

                    let prog = &mut self.state.programs[idx];
                    let tex_opt = &self.textures[idx];

                    // determine border color based on prog.lang
                    let border_color = if prog.lang == "en" {
                        egui::Color32::from_rgb(80, 150, 230)
                    } else {
                        egui::Color32::from_rgb(44, 180, 100)
                    };

                    // neutral card bg
                    let card_bg = egui::Color32::from_rgb(250, 250, 250);

                    // allocate a rect for the whole card with sense hover+click
                    let available_width = ui.available_width();
                    let card_size = egui::vec2(available_width, 64.0);
                    let (card_rect, card_resp) = ui.allocate_exact_size(card_size, egui::Sense::hover());

                    // draw fill and border (border thicker on hover)
                    let thickness = if card_resp.hovered() { 2.6 } else { 1.2 };
                    ui.painter().rect(
                        card_rect.shrink(2.0),
                        8.0,
                        card_bg,
                        egui::Stroke::new(thickness, border_color),
                    );

                    // child UI inside the card: use child_ui and then use the closure param `ui` throughout
                    let mut content_ui = ui.child_ui(card_rect.shrink2(egui::Vec2::splat(8.0)), egui::Layout::left_to_right(egui::Align::Center));
                    content_ui.horizontal(|ui| {
                        // Icon
                        if let Some(tex) = tex_opt {
                            ui.add(egui::Image::new((tex.id(), egui::vec2(44.0,44.0))));
                        } else {
                            ui.add_space(48.0);
                        }

                        // Name + subtitle
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&prog.name).size(15.0).strong());
                            ui.label(egui::RichText::new("Click toggle to set language").color(egui::Color32::from_gray(110)).size(11.0));
                        });

                        ui.add_space(8.0);

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // state pill
                            let pill = if prog.lang == "en" { "EN" } else { "FA" };
                            let pill_color = if prog.lang == "en" { egui::Color32::from_rgb(30,100,170) } else { egui::Color32::from_rgb(20,120,60) };
                            ui.colored_label(pill_color, pill);

                            ui.add_space(8.0);

                            // custom toggle
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

            // after rendering, if any changed => save once
            if changed_any {
                if let Err(e) = self.state.save_config() {
                    ui.label(format!("Save error: {}", e));
                }
            }
        });

        // bottom panel for watcher control (start/stop)
        egui::TopBottomPanel::bottom("watcher_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal_centered(|ui| {
                // try to update watcher status by checking if child already exited
                let mut running = false;
                if let Some(child) = &mut self.watcher {
                    match child.try_wait() {
                        Ok(Some(_status)) => {
                            // child exited; drop the handle
                            self.watcher = None;
                            running = false;
                        }
                        Ok(None) => running = true,
                        Err(_) => running = true,
                    }
                }

                if running {
                    if ui.button("Stop watcher").clicked() {
                        self.stop_watcher();
                    }
                } else {
                    if ui.button("Start watcher").clicked() {
                        self.start_watcher();
                    }
                }

                ui.add_space(8.0);
                ui.label(if running { "Watcher: running" } else { "Watcher: stopped" });
            });
            ui.add_space(6.0);
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Lang Switcher (Rust) — Polished UI",
        native_options,
        Box::new(|cc| Box::new(LangApp::new(cc))),
    );
}
