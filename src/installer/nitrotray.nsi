; @module installer
; @description NSIS script for NitroTray-<version>-setup.exe: installs the tray app and the LocalSystem
;              helper service for all users into %ProgramFiles%\NitroTray, with an uninstaller.
;
; @input  Defines passed by bin/package.ps1: VERSION (x.y.z), VERSION_NUM (x.y.z, numeric only),
;         SRCDIR (folder holding NitroTray.exe and NitroTrayService.exe), LICENSE_FILE (CRLF text), OUTFILE.
; @output One setup executable: Unicode, 64-bit Windows 10/11 only, runs as administrator.
; @dependencies NSIS 3 (MUI2, LogicLib, x64, WinVer, FileFunc, Sections); nitrotray.ico beside this script.
;
; The install folder is fixed. The helper runs as SYSTEM, so its binary must sit where only
; administrators can write (D-20260930-019): there is no directory page and /D= is ignored.
; Silent installs: /S [/NOHELPER] [/NOAUTOSTART]. Silent uninstall: Uninstall.exe /S (keeps settings).

!ifndef VERSION | VERSION_NUM | SRCDIR | LICENSE_FILE | OUTFILE
  !error "Build with bin/package.ps1 (it passes VERSION, VERSION_NUM, SRCDIR, LICENSE_FILE and OUTFILE)."
!endif

!define APP_NAME "NitroTray"
!define TRAY_EXE "NitroTray.exe"
!define HELPER_EXE "NitroTrayService.exe"
!define SERVICE_NAME "NitroTrayHelper"
!define UNINSTALLER "Uninstall.exe"
!define PUBLISHER "Brandon Stonebridge"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define SERVICE_KEY "SYSTEM\CurrentControlSet\Services\${SERVICE_NAME}"
!define WEBVIEW2_GUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"

Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel admin
Name "${APP_NAME}"
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\${APP_NAME}"
BrandingText "${APP_NAME} ${VERSION}"
ShowInstDetails show
ShowUninstDetails show

!include MUI2.nsh
!include LogicLib.nsh
!include x64.nsh
!include WinVer.nsh
!include FileFunc.nsh
!include Sections.nsh

!define MUI_ICON "nitrotray.ico"
!define MUI_UNICON "nitrotray.ico"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_SMALLDESC
!define MUI_FINISHPAGE_RUN
!define MUI_FINISHPAGE_RUN_TEXT "Start ${APP_NAME} now"
!define MUI_FINISHPAGE_RUN_FUNCTION StartAsUser

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${LICENSE_FILE}"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

VIProductVersion "${VERSION_NUM}.0"
VIFileVersion "${VERSION_NUM}.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "${APP_NAME} Setup"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "${PUBLISHER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "Copyright (c) 2026 ${PUBLISHER}, MIT License"

; ---------------------------------------------------------------------------------------------
; Shared helpers (installer and uninstaller each need their own copy).

!macro SHARED_FUNCTIONS prefix
; Waits up to 15 s for a program file to be released, so it can be replaced or deleted.
; A running image refuses to be opened for writing, which is what this probes.
Function ${prefix}WaitUnlocked
  Exch $R0
  Push $R1
  Push $R2
  StrCpy $R2 0
  ${Do}
    ${IfNot} ${FileExists} "$R0"
      ${Break}
    ${EndIf}
    ClearErrors
    FileOpen $R1 "$R0" a
    ${IfNot} ${Errors}
      FileClose $R1
      ${Break}
    ${EndIf}
    IntOp $R2 $R2 + 1
    ${If} $R2 >= 30
      DetailPrint "Still in use: $R0"
      ${Break}
    ${EndIf}
    Sleep 500
  ${Loop}
  Pop $R2
  Pop $R1
  Pop $R0
FunctionEnd

; Closes the tray and stops the helper so their binaries can be replaced or removed.
Function ${prefix}StopNitroTray
  ${If} ${FileExists} "$INSTDIR\${TRAY_EXE}"
    DetailPrint "Closing ${APP_NAME}"
    ExecWait '"$INSTDIR\${TRAY_EXE}" --exit'
    Push "$INSTDIR\${TRAY_EXE}"
    Call ${prefix}WaitUnlocked
  ${EndIf}
  ReadRegStr $0 HKLM "${SERVICE_KEY}" "ImagePath"
  ${If} $0 != ""
    DetailPrint "Stopping the ${APP_NAME} Helper service"
    nsExec::Exec '"$SYSDIR\sc.exe" stop ${SERVICE_NAME}'
    Pop $0
    Push "$INSTDIR\${HELPER_EXE}"
    Call ${prefix}WaitUnlocked
  ${EndIf}
FunctionEnd

; Removes the sign-in entry, but only when it starts this installation.
Function ${prefix}RemoveAutostart
  ReadRegStr $0 HKCU "${RUN_KEY}" "${APP_NAME}"
  ${If} $0 == '"$INSTDIR\${TRAY_EXE}" --autostart'
    DetailPrint "Removing ${APP_NAME} from the sign-in apps"
    DeleteRegValue HKCU "${RUN_KEY}" "${APP_NAME}"
  ${EndIf}
FunctionEnd
!macroend

!insertmacro SHARED_FUNCTIONS ""
!insertmacro SHARED_FUNCTIONS "un."

; ---------------------------------------------------------------------------------------------
; Install

Section "${APP_NAME}" SEC_APP
  SectionIn RO
  SetShellVarContext all
  Call StopNitroTray
  SetOutPath "$INSTDIR"
  File "${SRCDIR}\${TRAY_EXE}"
  File "${SRCDIR}\${HELPER_EXE}"
  File "nitrotray.ico"
  File "/oname=LICENSE.txt" "${LICENSE_FILE}"
  WriteUninstaller "$INSTDIR\${UNINSTALLER}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\${TRAY_EXE}" "" "$INSTDIR\nitrotray.ico" 0 \
    SW_SHOWNORMAL "" "NitroSense tray companion"

  WriteRegStr HKLM "${UNINST_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKLM "${UNINST_KEY}" "Publisher" "${PUBLISHER}"
  WriteRegStr HKLM "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\nitrotray.ico"
  WriteRegStr HKLM "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINST_KEY}" "UninstallString" '"$INSTDIR\${UNINSTALLER}"'
  WriteRegStr HKLM "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\${UNINSTALLER}" /S'
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${UNINST_KEY}" "EstimatedSize" "$0"
SectionEnd

Section "Helper service" SEC_HELPER
  ; `install` registers (or updates) the LocalSystem service to run this Program Files copy, then starts it.
  DetailPrint "Registering the ${APP_NAME} Helper service"
  nsExec::ExecToLog '"$INSTDIR\${HELPER_EXE}" install'
  Pop $0
  ${If} $0 != 0
    DetailPrint "Helper service install failed (exit code $0)"
    MessageBox MB_ICONEXCLAMATION|MB_OK "The ${APP_NAME} Helper service could not be installed (exit code $0).$\r$\n${APP_NAME} still runs, with hardware controls shown read-only." /SD IDOK
  ${EndIf}
SectionEnd

Section "Start with Windows" SEC_AUTOSTART
  WriteRegStr HKCU "${RUN_KEY}" "${APP_NAME}" '"$INSTDIR\${TRAY_EXE}" --autostart'
SectionEnd

; An unticked component on an upgrade removes what the previous installation set up.
Section "-ApplyChoices"
  ${IfNot} ${SectionIsSelected} ${SEC_HELPER}
    ReadRegStr $0 HKLM "${SERVICE_KEY}" "ImagePath"
    ${If} $0 != ""
      DetailPrint "Removing the ${APP_NAME} Helper service"
      nsExec::ExecToLog '"$INSTDIR\${HELPER_EXE}" uninstall'
      Pop $0
    ${EndIf}
  ${EndIf}
  ${IfNot} ${SectionIsSelected} ${SEC_AUTOSTART}
    Call RemoveAutostart
  ${EndIf}
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_APP} "The tray app: popup, CPU/GPU graph tray icons, monitoring window and settings."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_HELPER} "LocalSystem service with six typed Acer firmware commands. Without it, firmware temperatures, fan speeds and the fan/performance controls are unavailable."
  !insertmacro MUI_DESCRIPTION_TEXT ${SEC_AUTOSTART} "Start ${APP_NAME} when you sign in."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP|MB_OK "${APP_NAME} needs 64-bit Windows." /SD IDOK
    Abort
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_ICONSTOP|MB_OK "${APP_NAME} needs Windows 10 or 11." /SD IDOK
    Abort
  ${EndIf}
  SetRegView 64
  StrCpy $INSTDIR "$PROGRAMFILES64\${APP_NAME}"

  ; On an upgrade, pre-tick only what the current installation has, so the defaults keep it as it is.
  ReadRegStr $0 HKLM "${UNINST_KEY}" "DisplayVersion"
  ${If} $0 != ""
    ReadRegStr $1 HKLM "${SERVICE_KEY}" "ImagePath"
    ${If} $1 == ""
      !insertmacro UnselectSection ${SEC_HELPER}
    ${EndIf}
    ReadRegStr $1 HKCU "${RUN_KEY}" "${APP_NAME}"
    ${If} $1 == ""
      !insertmacro UnselectSection ${SEC_AUTOSTART}
    ${EndIf}
  ${EndIf}

  ${GetParameters} $R0
  ClearErrors
  ${GetOptions} $R0 "/NOHELPER" $R1
  ${IfNot} ${Errors}
    !insertmacro UnselectSection ${SEC_HELPER}
  ${EndIf}
  ClearErrors
  ${GetOptions} $R0 "/NOAUTOSTART" $R1
  ${IfNot} ${Errors}
    !insertmacro UnselectSection ${SEC_AUTOSTART}
  ${EndIf}

  ; WebView2 ships with Windows 11; warn, but do not block, when it is missing.
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${WEBVIEW2_GUID}" "pv"
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\${WEBVIEW2_GUID}" "pv"
  ${EndIf}
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    MessageBox MB_ICONEXCLAMATION|MB_OK "The Microsoft Edge WebView2 Runtime was not found. ${APP_NAME}'s popup and windows need it:$\r$\nhttps://developer.microsoft.com/microsoft-edge/webview2/" /SD IDOK
  ${EndIf}
FunctionEnd

; The installer is elevated; Explorer opens the program in the signed-in user's normal context.
Function StartAsUser
  Exec '"$WINDIR\explorer.exe" "$INSTDIR\${TRAY_EXE}"'
FunctionEnd

; ---------------------------------------------------------------------------------------------
; Uninstall

Function un.onInit
  SetRegView 64
FunctionEnd

Section "Uninstall"
  SetShellVarContext all
  Call un.StopNitroTray
  ReadRegStr $0 HKLM "${SERVICE_KEY}" "ImagePath"
  ${If} $0 != ""
    DetailPrint "Removing the ${APP_NAME} Helper service"
    nsExec::ExecToLog '"$INSTDIR\${HELPER_EXE}" uninstall'
    Pop $0
  ${EndIf}
  Push "$INSTDIR\${HELPER_EXE}"
  Call un.WaitUnlocked

  Delete "$INSTDIR\${TRAY_EXE}"
  Delete "$INSTDIR\${HELPER_EXE}"
  Delete "$INSTDIR\nitrotray.ico"
  Delete "$INSTDIR\LICENSE.txt"
  ; The helper's custom-curve marker; a clean service stop already removed it.
  Delete "$INSTDIR\curve-active"
  Delete "$INSTDIR\${UNINSTALLER}"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  DeleteRegKey HKLM "${UNINST_KEY}"

  ; Per-user state belongs to the signed-in user: switch context before touching it.
  SetShellVarContext current
  Call un.RemoveAutostart
  ${If} ${FileExists} "$LOCALAPPDATA\${APP_NAME}\*.*"
    ${If} ${Cmd} `MessageBox MB_YESNO|MB_ICONQUESTION "Also delete your ${APP_NAME} settings?$\r$\n$LOCALAPPDATA\${APP_NAME}" /SD IDNO IDYES`
      RMDir /r "$LOCALAPPDATA\${APP_NAME}"
    ${EndIf}
  ${EndIf}
SectionEnd
