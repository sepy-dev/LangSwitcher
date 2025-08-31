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
     <a href="https://github.com/sepy-dev/LangSwitcher/releases" target="_blank" rel="noopener">
    <img alt="📦 دانلود آخرین نسخه" src="https://img.shields.io/badge/📦%20Download%20Latest%20Release-v%20—-purple?style=for-the-badge&labelColor=111827&color=7a42f4">
  </a>
2.  imstaller رو‌ دانلود کن
3. نصب کن چ سپس با شرتکات اجرا

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



LangSwitcher — Quick Project Overview

LangSwitcher is a lightweight and modern tool designed to switch keyboard languages instantly (EN ↔ FA) on Windows. Built with Rust and inspired by Cyberpunk UI, it provides a sleek, minimalistic interface while running efficiently in the background.

Key Features

🎨 Modern Cyberpunk-inspired design

⌨️ Instant language switching between English and Persian

⚡ Automatic detection of running applications

👀 Background Watcher for real-time monitoring

💾 Persistent settings for a seamless experience

Perfect for developers, translators, and power users who need a fast and visually appealing language switcher on Windows
