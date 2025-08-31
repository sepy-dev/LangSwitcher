# <p align="center">Language Switcher 🔁</p>

<p align="center">
  <img src="icons/logo.png" alt="Language Switcher Logo" width="120"/>
</p>

<p align="center">
  <b>🔄 یک ابزار سبک، سریع و چشم‌نواز برای تغییر سریع زبان برنامه‌ها (EN ↔ FA)</b><br>
  ساخته‌شده با <a href="https://www.rust-lang.org/" target="_blank">Rust</a> — طراحی: Cyberpunk UI
</p>

<p align="center">
  <!-- دکمهٔ دانلود رنگی -->
  <a href="https://github.com/sepy-dev/LangSwitcher/releases" target="_blank" rel="noopener">
    <img alt="📦 دانلود آخرین نسخه" src="https://img.shields.io/badge/📦%20Download%20Latest%20Release-v%20—-purple?style=for-the-badge&labelColor=111827&color=7a42f4">
  </a>
  &nbsp;
  <a href="https://github.com/sepy-dev/LangSwitcher/stargazers" target="_blank" rel="noopener">
    <img src="https://img.shields.io/github/stars/sepy-dev/LangSwitcher?style=for-the-badge&label=Stars&color=ffb86b&labelColor=111827" alt="Stars">
  </a>
  &nbsp;
  <a href="https://github.com/sepy-dev/LangSwitcher/actions" target="_blank" rel="noopener">
    <img src="https://img.shields.io/github/actions/workflow/status/sepy-dev/LangSwitcher/ci.yml?style=for-the-badge&label=CI&labelColor=111827&color=06b6d4" alt="CI">
  </a>
  &nbsp;
  <a href="https://github.com/sepy-dev/LangSwitcher/blob/main/LICENSE" target="_blank" rel="noopener">
    <img src="https://img.shields.io/badge/License-MIT-00b894?style=for-the-badge&labelColor=111827" alt="License">
  </a>
</p>
🌐 Socials
<p align="center"> <a href="https://github.com/sepy-dev" target="_blank"> <img src="https://img.shields.io/badge/GitHub-%23181717.svg?&style=for-the-badge&logo=github&logoColor=white" /> </a> <a href="https://x.com/sepy_dev" target="_blank"> <img src="https://img.shields.io/badge/X-%23000000.svg?&style=for-the-badge&logo=x&logoColor=white" /> </a> <a href="https://www.instagram.com/sepehr.ramzany" target="_blank"> <img src="https://img.shields.io/badge/Instagram-%23E4405F.svg?&style=for-the-badge&logo=instagram&logoColor=white" /> </a> <a href="[https://linkedin.com/in/sepy-dev](https://www.linkedin.com/in/sepehr-ramzani-133043330/)" target="_blank"> <img src="https://img.shields.io/badge/LinkedIn-%230077B5.svg?&style=for-the-badge&logo=linkedin&logoColor=white" /> </a> </p
---


-

---

## ✨ ویژگی‌ها
<p align="center">
  <img src="https://img.shields.io/badge/UI-Cyberpunk-7a42f4?style=for-the-badge" alt="UI"/>
  <img src="https://img.shields.io/badge/Language-EN%20%7C%20FA-009688?style=for-the-badge" alt="Languages"/>
  <img src="https://img.shields.io/badge/Platform-Windows-blue?style=for-the-badge&logo=windows" alt="Platform"/>
  <img src="https://img.shields.io/badge/Performance-Lightweight-success?style=for-the-badge" alt="Performance"/>
</p>

- 🎨 طراحی مدرن و مینیمال با حال و هوای Cyberpunk  
- ⌨️ سوییچ فوری بین **انگلیسی** و **فارسی** (EN ↔ FA) برای برنامه‌های فعال  
- ⚡ تشخیص خودکار برنامه‌های باز برای اعمال سوییچ هوشمند  
- 👀 حالت **Watcher** برای نظارت پس‌زمینه و اعمال خودکار تغییر زبان  
- 💾 ذخیرهٔ تنظیمات به‌صورت پایدار (فایل کانفیگ)  
- 🧩 پوشهٔ `icons/` برای آیکن برنامه‌ها — قابل سفارشی‌سازی

---

## 🖼️ پیش‌نمایش

<p align="center">
  <img src="docs/Screenshot.png" alt="App Preview 1" width="720"/>
</p>

---

<p align="center">
  <img src="docs/Screenshot2.png" alt="App Preview 2" width="360"/>
</p>


---

## 📦 نصب و اجرا (برای کاربران نهایی)

### روش سریع — دانلود از Releases (رنگی و واضح)
1. به صفحهٔ Releases برو:  
   `https://github.com/sepy-dev/LangSwitcher/releases`  
2. آخرین فایل ویندوزی (`.zip` یا `.exe`) را دانلود کن.  
3. فایل را استخراج کن و مطمئن شو پوشه `icons/` کنار `LanguageSwitcher.exe` قرار دارد.  
4. روی `LanguageSwitcher.exe` دابل‌کلیک کن تا اجرا شود.  
5. (اختیاری) برای اجرای دائمی Watcher، آن را از داخل برنامه فعال کن یا یک میانبر در استارت آپ ویندوز بساز.

---

## 🛠️ برای توسعه‌دهندگان

### پیش‌نیازها
- Rust (stable) — [نصب از سایت رسمی](https://www.rust-lang.org/)  
- Cargo (همراه Rust)

### کلون و ساخت
```bash
# کلون کردن ریپو
git clone https://github.com/sepy-dev/LangSwitcher.git
cd LangSwitcher

# ساخت نسخه release
cargo build --release

# باینری خروجی:
# target/release/LanguageSwitcher(.exe)

اجرای محلی (برای دیباگ)

# اجرا با لاگ و حالت توسعه
cargo run

```



# config.toml (نمونه)
default_language = "EN"      # EN یا FA
hotkey = "Ctrl+Alt+L"       # کلید میانبر برای سوییچ دستی
watcher_enabled = true      # فعال بودن Watcher در پس‌زمینه
exclude_apps = ["Code.exe", "Telegram.exe"]  # برنامه‌هایی که نباید تغییر زبان بخورند
icons_folder = "icons"      # مسیر پوشه آیکن‌ها
