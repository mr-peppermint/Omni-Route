!macro NSIS_HOOK_PREINSTALL
  DetailPrint "Stopping existing Omni-Route service"
  nsExec::ExecToLog 'sc.exe stop OmniRouteService'
  ; sc stop ritorna subito: il processo del servizio tiene ancora bloccato
  ; omni-route.exe per qualche istante e in modalità silenziosa (/S) NSIS
  ; salterebbe la copia del file senza errori, lasciando installata la
  ; versione vecchia. Aspettiamo (max 20 s) che il processo esca davvero.
  StrCpy $R9 0
  omni_route_wait_service_exit:
    nsExec::ExecToStack 'cmd.exe /c tasklist /FI "IMAGENAME eq omni-route.exe" /FI "SESSION eq 0" /NH | find /I "omni-route.exe"'
    Pop $0
    Pop $1
    StrCmp $0 "0" 0 omni_route_service_exited
    IntOp $R9 $R9 + 1
    IntCmp $R9 40 omni_route_service_exited
    Sleep 500
    Goto omni_route_wait_service_exit
  omni_route_service_exited:
  nsExec::ExecToLog 'sc.exe delete OmniRouteService'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  DetailPrint "Configuring Omni-Route service"
  nsExec::ExecToStack '"$INSTDIR\omni-route.exe" install-service'
  Pop $0
  Pop $1
  ${If} $0 != 0
    MessageBox MB_ICONSTOP|MB_OK "The Omni-Route service could not be installed (code $0). Please reinstall the app as administrator."
    Abort
  ${EndIf}

  ; Per impostazione predefinita Windows "riduce al minimo le connessioni
  ; simultanee": quando si collega l'Ethernet stacca il Wi-Fi (evento WLAN
  ; 8003, "criterio che ha disabilitato la connessione automatica"). Omni-Route
  ; ha bisogno di tutte le connessioni attive insieme, quindi le consente.
  ; Il valore precedente viene salvato una sola volta e ripristinato alla
  ; disinstallazione.
  DetailPrint "Consento a Windows di restare connesso a più reti insieme"
  SetRegView 64
  ClearErrors
  ReadRegStr $2 HKLM "Software\Omni-Route" "WcmMinimizeConnectionsPrevious"
  ${If} ${Errors}
    ClearErrors
    ReadRegDWORD $2 HKLM "SOFTWARE\Policies\Microsoft\Windows\WcmSvc\GroupPolicy" "fMinimizeConnections"
    ${If} ${Errors}
      StrCpy $2 "absent"
    ${EndIf}
    WriteRegStr HKLM "Software\Omni-Route" "WcmMinimizeConnectionsPrevious" $2
  ${EndIf}
  WriteRegDWORD HKLM "SOFTWARE\Policies\Microsoft\Windows\WcmSvc\GroupPolicy" "fMinimizeConnections" 0

  ; Se il servizio va in crash senza lasciare traccia nel log (es. abort del
  ; processo) Windows salva un minidump locale: resta sul PC, non viene
  ; inviato da nessuna parte, al massimo 3 file.
  DetailPrint "Enabling local crash dumps for Omni-Route"
  WriteRegExpandStr HKLM "SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\omni-route.exe" "DumpFolder" "%ProgramData%\Omni-Route\crashdumps"
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\omni-route.exe" "DumpCount" 3
  WriteRegDWORD HKLM "SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\omni-route.exe" "DumpType" 1
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DetailPrint "Removing Omni-Route service"
  nsExec::ExecToLog '"$INSTDIR\omni-route.exe" remove-service'
  SetRegView 64
  DeleteRegKey HKLM "SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\omni-route.exe"

  DetailPrint "Restoring Windows connection management"
  SetRegView 64
  ClearErrors
  ReadRegStr $2 HKLM "Software\Omni-Route" "WcmMinimizeConnectionsPrevious"
  ${IfNot} ${Errors}
    ${If} $2 == "absent"
      DeleteRegValue HKLM "SOFTWARE\Policies\Microsoft\Windows\WcmSvc\GroupPolicy" "fMinimizeConnections"
    ${Else}
      WriteRegDWORD HKLM "SOFTWARE\Policies\Microsoft\Windows\WcmSvc\GroupPolicy" "fMinimizeConnections" $2
    ${EndIf}
    DeleteRegValue HKLM "Software\Omni-Route" "WcmMinimizeConnectionsPrevious"
    DeleteRegKey /ifempty HKLM "Software\Omni-Route"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
!macroend
