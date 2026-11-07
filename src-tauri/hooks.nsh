; Startup Registration Hook for sys4
; This script adds sys4 to Windows startup using the NSIS hook mechanism

; NSIS_HOOK_POSTINSTALL runs after installation is complete
!macro NSIS_HOOK_POSTINSTALL
    ; Ask user if they want to add to startup
    MessageBox MB_YESNO "$(STARTUP_PROMPT)" IDNO skip_startup
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "sys4" "$INSTDIR\sys4.exe --autostart"
    skip_startup:
!macroend

; NSIS_HOOK_POSTUNINSTALL runs after uninstallation
!macro NSIS_HOOK_POSTUNINSTALL
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "sys4"
!macroend

; Custom language string for the startup prompt
LangString STARTUP_PROMPT ${LANG_KOREAN} "시작 프로그램에 등록하시겠습니까?$\nWould you like to run sys4 at startup?"
LangString STARTUP_PROMPT ${LANG_ENGLISH} "Would you like to run sys4 at startup?"
