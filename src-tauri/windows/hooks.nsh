; 0.1.x shipped as "Flitty" in its own folder. Remove that install first
; (silently, which closes it and keeps settings) so users don't end up with two apps.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $R9 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Flitty" "UninstallString"
  ReadRegStr $R8 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Flitty" "InstallLocation"
  ; InstallLocation is stored quoted, but _?= needs a bare path.
  StrCpy $R7 $R8 1
  ${If} $R7 == '"'
    StrCpy $R8 $R8 -1 1
  ${EndIf}
  ${If} $R9 != ""
  ${AndIf} $R8 != ""
    ; A silent uninstall aborts if the old app is running, so close it first.
    nsExec::Exec '"$SYSDIR\taskkill.exe" /F /IM flitty.exe'
    Pop $R6
    Sleep 1000
    ExecWait '$R9 /S _?=$R8'
    Sleep 1000
    Delete "$R8\uninstall.exe"
    RMDir "$R8"
  ${EndIf}
!macroend
