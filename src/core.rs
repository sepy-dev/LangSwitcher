// src/core.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use sysinfo::{ProcessExt, System, SystemExt};

pub const CONFIG_FILE: &str = "lang_config.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config(pub HashMap<String, String>); // prog_name -> "en"|"fa"

#[derive(Debug, Clone)]
pub struct Program {
    pub name: String,
    pub lang: String,
}

pub struct LangState {
    pub programs: Vec<Program>,
}

impl LangState {
    /// فهرست برنامه‌های از پیش تعریف‌شده (در صورت نیاز اضافه/حذف کن)
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

    /// ساختار اولیه: اسکن پروسس‌ها و لود کانفیگ
    pub fn new() -> Self {
        let predefined = Self::predefined_list();

        let mut sys = System::new_all();
        sys.refresh_processes();

        // برنامه‌هایی که فعلاً پروسس‌شون وجود داره
        let mut available: Vec<String> = Vec::new();
        for p in &predefined {
            for (_pid, proc_) in sys.processes() {
                if proc_.name().eq_ignore_ascii_case(p) {
                    available.push(p.to_string());
                    break;
                }
            }
        }

        // لود کانفیگ موجود
        let cfg: Config = fs::read_to_string(CONFIG_FILE)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let mut programs = Vec::new();
        for name in available {
            let lang = cfg.0.get(&name).cloned().unwrap_or_else(|| "en".to_string());
            programs.push(Program { name, lang });
        }

        Self { programs }
    }

    /// toggle زبان برنامه بر اساس اندیس
    pub fn toggle_by_index(&mut self, idx: usize) {
        if let Some(p) = self.programs.get_mut(idx) {
            p.lang = if p.lang == "en" { "fa".to_string() } else { "en".to_string() };
        }
    }

    /// ذخیرهٔ کانفیگ فعلی به فایل
    pub fn save_config(&self) -> std::io::Result<()> {
        let mut cfg = Config::default();
        for p in &self.programs {
            cfg.0.insert(p.name.clone(), p.lang.clone());
        }
        let txt = serde_json::to_string_pretty(&cfg)?;
        fs::write(CONFIG_FILE, txt)?;
        Ok(())
    }

    /// رفرش مجدد لیست برنامه‌ها (مثلاً بعد از نصب یا اجرای جدید)
    pub fn refresh(&mut self) {
        let predefined = Self::predefined_list();

        let mut sys = System::new_all();
        sys.refresh_processes();

        let mut available: Vec<String> = Vec::new();
        for p in &predefined {
            for (_pid, proc_) in sys.processes() {
                if proc_.name().eq_ignore_ascii_case(p) {
                    available.push(p.to_string());
                    break;
                }
            }
        }

        // بازسازی programs بر اساس available، ولی اگر نامی در config بود نگه می‌داریم مقدار قبلی
        // برای سادگی این نسخه مقدار پیش‌فرض en است اگر در لیست نباشد
        let mut new_programs = Vec::new();
        for name in available {
            // سعی کنید مقدار قبلی را حفظ کنید اگر وجود داشته باشد
            if let Some(old) = self.programs.iter().find(|x| x.name == name) {
                new_programs.push(old.clone());
            } else {
                new_programs.push(Program { name, lang: "en".to_string() });
            }
        }
        self.programs = new_programs;
    }
}
