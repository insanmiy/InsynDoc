!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr SHCTX "Software\Classes\InsynDoc.Document" "" "Word Document"
  WriteRegStr SHCTX "Software\Classes\InsynDoc.Document\DefaultIcon" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\",0'
  WriteRegStr SHCTX "Software\Classes\InsynDoc.Document\shell\open" "" "Open With InsynDoc"
  WriteRegStr SHCTX "Software\Classes\InsynDoc.Document\shell\open\command" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\"'
  WriteRegStr SHCTX "Software\Classes\.docx\OpenWithProgids" "InsynDoc.Document" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "InsynDoc"
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" ".docx" ""
  WriteRegStr SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" $\"%1$\"'
  WriteRegStr SHCTX "Software\InsynDoc\Capabilities" "ApplicationName" "InsynDoc"
  WriteRegStr SHCTX "Software\InsynDoc\Capabilities" "ApplicationDescription" "Offline DOCX Editor"
  WriteRegStr SHCTX "Software\InsynDoc\Capabilities\FileAssociations" ".docx" "InsynDoc.Document"
  WriteRegStr SHCTX "Software\RegisteredApplications" "InsynDoc" "Software\InsynDoc\Capabilities"
  ReadRegStr $R0 HKCR ".docx" ""
  ReadRegStr $R1 HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.docx\UserChoice" "ProgId"
  ${If} $R0 == ""
  ${AndIf} $R1 == ""
    WriteRegStr SHCTX "Software\Classes\.docx" "" "InsynDoc.Document"
  ${EndIf}
  !insertmacro UPDATEFILEASSOC
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ReadRegStr $R0 SHCTX "Software\Classes\.docx" ""
  ${If} $R0 == "InsynDoc.Document"
    DeleteRegValue SHCTX "Software\Classes\.docx" ""
  ${EndIf}
  DeleteRegValue SHCTX "Software\Classes\.docx\OpenWithProgids" "InsynDoc.Document"
  DeleteRegKey /ifempty SHCTX "Software\Classes\.docx\OpenWithProgids"
  DeleteRegKey /ifempty SHCTX "Software\Classes\.docx"
  DeleteRegKey SHCTX "Software\Classes\InsynDoc.Document"
  DeleteRegKey SHCTX "Software\Classes\Applications\${MAINBINARYNAME}.exe"
  DeleteRegValue SHCTX "Software\RegisteredApplications" "InsynDoc"
  DeleteRegKey SHCTX "Software\InsynDoc\Capabilities"
  DeleteRegKey /ifempty SHCTX "Software\InsynDoc"
  !insertmacro UPDATEFILEASSOC
!macroend
