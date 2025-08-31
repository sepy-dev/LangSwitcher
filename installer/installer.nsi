;--------------------------------
; LangSwitcher Installer - Ultra Modern
;--------------------------------

!define APP_NAME "LangSwitcher"
!define INSTALL_DIR "$PROGRAMFILES\LangSwitcher"
!define ICON_PATH "assets\icon.ico"
!define SHORTCUT_NAME "LangSwitcher.lnk"
!define UNINSTALLER_NAME "Uninstall_LangSwitcher.exe"

OutFile "LangSwitcher_Installer.exe"
InstallDir ${INSTALL_DIR}
RequestExecutionLevel admin

;--------------------------------
; Pages
;--------------------------------
Page directory        ; انتخاب مسیر نصب
Page instfiles        ; نمایش پیشرفت نصب
UninstPage uninstConfirm
UninstPage instfiles

;--------------------------------
; Sections - Install
;--------------------------------
Section "Install"

    ; Create installation directory
    SetOutPath ${INSTALL_DIR}

    ; Copy executables
    File "lang_switcher_rust.exe"
    File "watcher.exe"

    ; Copy assets folder recursively
    SetOutPath "${INSTALL_DIR}\assets"
    File /r "assets\*.*"

    ; Copy icons folder recursively
    SetOutPath "${INSTALL_DIR}\icons"
    File /r "icons\*.*"

    ; Create Desktop Shortcut pointing to lang_switcher_rust.exe
    CreateShortcut "$DESKTOP\${SHORTCUT_NAME}" "${INSTALL_DIR}\lang_switcher_rust.exe" "" "${INSTALL_DIR}\${ICON_PATH}" 0

    ; Create Start Menu Shortcut
    CreateDirectory "$SMPROGRAMS\${APP_NAME}"
    CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "${INSTALL_DIR}\lang_switcher_rust.exe" "" "${INSTALL_DIR}\${ICON_PATH}" 0

    ; Write Uninstaller
    WriteUninstaller "${INSTALL_DIR}\${UNINSTALLER_NAME}"

SectionEnd

;--------------------------------
; Sections - Uninstall
;--------------------------------
Section "Uninstall"

    ; Delete shortcuts
    Delete "$DESKTOP\${SHORTCUT_NAME}"
    Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
    RMDir "$SMPROGRAMS\${APP_NAME}"

    ; Delete executables
    Delete "${INSTALL_DIR}\lang_switcher_rust.exe"
    Delete "${INSTALL_DIR}\watcher.exe"

    ; Delete folders recursively
    RMDir /r "${INSTALL_DIR}\assets"
    RMDir /r "${INSTALL_DIR}\icons"

    ; Delete uninstaller
    Delete "${INSTALL_DIR}\${UNINSTALLER_NAME}"

    ; Remove installation directory
    RMDir ${INSTALL_DIR}

SectionEnd

