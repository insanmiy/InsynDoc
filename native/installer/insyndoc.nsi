Unicode true
!include "MUI2.nsh"

!ifndef APP_VERSION
  !error "Provide APP_VERSION From native/Cargo.toml."
!endif
!ifndef APP_BINARY
  !define APP_BINARY "${__FILEDIR__}\..\target\release\insyndoc-native.exe"
!endif
!ifndef OUTPUT_FILE
  !define OUTPUT_FILE "${__FILEDIR__}\..\target\InsynDoc-Native-Preview-Setup.exe"
!endif
!define APP_NAME "InsynDoc Native Preview"
!define APP_EXE "insyndoc-native.exe"
!define APP_PROGID "InsynDoc.NativePreview.Document"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\InsynDocNativePreview"

Name "${APP_NAME}"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\InsynDocNativePreview"
InstallDirRegKey HKCU "Software\InsynDocNativePreview" "InstallPath"
RequestExecutionLevel user
Icon "${__FILEDIR__}\..\..\src-tauri\icons\insyndoc\icon.ico"
UninstallIcon "${__FILEDIR__}\..\..\src-tauri\icons\insyndoc\icon.ico"
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "FileDescription" "${APP_NAME} Installer"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${__FILEDIR__}\..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File /oname=${APP_EXE} "${APP_BINARY}"
  File /oname=LICENSE.txt "${__FILEDIR__}\..\..\LICENSE"
  File /oname=Inter-OFL.txt "${__FILEDIR__}\..\..\licenses\Inter-OFL.txt"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\InsynDoc Native Preview"
  CreateShortcut "$SMPROGRAMS\InsynDoc Native Preview\InsynDoc Native Preview.lnk" "$INSTDIR\${APP_EXE}"
  CreateShortcut "$SMPROGRAMS\InsynDoc Native Preview\Uninstall.lnk" "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "Software\InsynDocNativePreview" "InstallPath" "$INSTDIR"
  WriteRegStr HKCU "Software\Classes\${APP_PROGID}" "" "Word Document"
  WriteRegStr HKCU "Software\Classes\${APP_PROGID}\DefaultIcon" "" '$"$INSTDIR\${APP_EXE}$",0'
  WriteRegStr HKCU "Software\Classes\${APP_PROGID}\shell\open\command" "" '$"$INSTDIR\${APP_EXE}$" $"%1$"'
  WriteRegStr HKCU "Software\Classes\.docx\OpenWithProgids" "${APP_PROGID}" ""
  WriteRegStr HKCU "Software\Classes\Applications\${APP_EXE}" "FriendlyAppName" "${APP_NAME}"
  WriteRegStr HKCU "Software\Classes\Applications\${APP_EXE}\SupportedTypes" ".docx" ""
  WriteRegStr HKCU "Software\Classes\Applications\${APP_EXE}\shell\open\command" "" '$"$INSTDIR\${APP_EXE}$" $"%1$"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "InsynDoc"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" '$INSTDIR\${APP_EXE}'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '$"$INSTDIR\Uninstall.exe$"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$SMPROGRAMS\InsynDoc Native Preview\InsynDoc Native Preview.lnk"
  Delete "$SMPROGRAMS\InsynDoc Native Preview\Uninstall.lnk"
  RMDir "$SMPROGRAMS\InsynDoc Native Preview"
  Delete "$INSTDIR\${APP_EXE}"
  Delete "$INSTDIR\LICENSE.txt"
  Delete "$INSTDIR\Inter-OFL.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegValue HKCU "Software\Classes\.docx\OpenWithProgids" "${APP_PROGID}"
  DeleteRegKey /ifempty HKCU "Software\Classes\.docx\OpenWithProgids"
  DeleteRegKey /ifempty HKCU "Software\Classes\.docx"
  DeleteRegKey HKCU "Software\Classes\${APP_PROGID}"
  DeleteRegKey HKCU "Software\Classes\Applications\${APP_EXE}"
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "Software\InsynDocNativePreview"
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
SectionEnd
