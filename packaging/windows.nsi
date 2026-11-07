Unicode True
!include "MUI2.nsh"
Name "Bolum"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\Bolum"
InstallDirRegKey HKCU "Software\Bolum" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma
VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "Bolum"
VIAddVersionKey "FileDescription" "Bolum voice dictation installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "Copyright 2026 shivraj598"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\bolum.exe"
!define MUI_FINISHPAGE_RUN_NOTCHECKED
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "${STAGE}\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "Bolum"
  SetShellVarContext current
  IfFileExists "$INSTDIR\bolum.exe" 0 +3
    ExecWait '"$INSTDIR\bolum.exe" quit'
    Sleep 500
  SetOutPath "$INSTDIR"
  File "${STAGE}\*"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Bolum"
  CreateShortCut "$SMPROGRAMS\Bolum\Bolum.lnk" "$INSTDIR\bolum.exe"
  CreateShortCut "$SMPROGRAMS\Bolum\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Bolum" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "DisplayName" "Bolum"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "Publisher" "shivrajcode"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "DisplayIcon" "$INSTDIR\bolum.exe"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum" "NoRepair" 1
SectionEnd
Section "Uninstall"
  SetShellVarContext current
  ExecWait '"$INSTDIR\bolum.exe" quit'
  Sleep 500
  Delete "$SMPROGRAMS\Bolum\Bolum.lnk"
  Delete "$SMPROGRAMS\Bolum\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Bolum"
  Delete "$INSTDIR\bolum.exe"
  Delete "$INSTDIR\*.dll"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.txt"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Bolum"
  DeleteRegKey HKCU "Software\Bolum"
  ; Deliberately preserve downloaded models, settings, and unrelated files.
SectionEnd
