param(
    [string]$Executable = "$PSScriptRoot\..\src-tauri\target\release\insyndoc.exe",
    [switch]$Dialogs,
    [switch]$WindowOnly
)

$ErrorActionPreference = 'Stop'
if ([Environment]::OSVersion.Platform -ne 'Win32NT' -or -not [Environment]::UserInteractive) {
    throw 'Native Smoke Tests Require An Interactive Windows Desktop.'
}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class NativeSmokeDialogs {
    public delegate bool EnumChildCallback(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, EnumChildCallback callback, IntPtr parameter);
    [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr parameter, string text);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, IntPtr parameter, IntPtr data);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr window, int command);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
}
'@

function Find-NativeControl($Dialog, [int]$Identifier, [string]$ClassName) {
    $script:controlHandle = [IntPtr]::Zero
    $callback = [NativeSmokeDialogs+EnumChildCallback] {
        param($control, $parameter)
        $classBuffer = [Text.StringBuilder]::new(256)
        $null = [NativeSmokeDialogs]::GetClassName($control, $classBuffer, $classBuffer.Capacity)
        if ([NativeSmokeDialogs]::GetDlgCtrlID($control) -eq $Identifier -and $classBuffer.ToString() -eq $ClassName) {
            $script:controlHandle = $control
        }
        return $true
    }
    $null = [NativeSmokeDialogs]::EnumChildWindows([IntPtr]$Dialog.Current.NativeWindowHandle, $callback, [IntPtr]::Zero)
    return $script:controlHandle
}

function Wait-Until([scriptblock]$Check, [string]$Description) {
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        $result = & $Check
        if ($result) { return $result }
        Start-Sleep -Milliseconds 150
    } while ([DateTime]::UtcNow -lt $deadline)
    throw "Timed Out: $Description"
}

function Find-Element($Root, [string]$Name, $ControlType) {
    $condition = [System.Windows.Automation.AndCondition]::new(
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, $Name),
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ControlTypeProperty, $ControlType)
    )
    return $Root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
}

function Invoke-Button($Root, [string]$Name) {
    if ($Root.Current.ClassName -eq '#32770' -and $Name -in @('Cancel', 'Open', 'Save')) {
        $identifier = if ($Name -eq 'Cancel') { 2 } else { 1 }
        $control = Wait-Until {
            $candidate = Find-NativeControl $Root $identifier 'Button'
            if ($candidate -ne [IntPtr]::Zero) { $candidate }
        } "Native Dialog Button $Name"
        if (-not [NativeSmokeDialogs]::PostMessage($control, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)) {
            throw "Native Dialog Command Failed: $Name"
        }
        return
    }
    try {
        $button = Wait-Until { Find-Element $Root $Name ([System.Windows.Automation.ControlType]::Button) } "Button $Name"
    } catch {
        $Root.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition) |
            ForEach-Object { Write-Output "$($_.Current.ControlType.ProgrammaticName): $($_.Current.Name) [$($_.Current.AutomationId)]" }
        throw
    }
    if (-not $button.Current.IsEnabled) { throw "Button Is Disabled: $Name" }
    $pattern = $button.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
    $pattern.Invoke()
}

function Get-Dialog([string]$Name) {
    $dialog = Wait-Until {
        $windows = [System.Windows.Automation.AutomationElement]::RootElement.FindAll(
            [System.Windows.Automation.TreeScope]::Children,
            [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $script:application.Id)
        )
        $windows | Where-Object { $_.Current.Name -eq $Name -and $_.Current.ControlType -eq [System.Windows.Automation.ControlType]::Window } | Select-Object -First 1
    } "Native Dialog $Name"
    Write-Output "Native Dialog: $($dialog.Current.Name), Class: $($dialog.Current.ClassName), Offscreen: $($dialog.Current.IsOffscreen)" | Out-Host
    return $dialog
}

function Set-DialogPath($Dialog, [string]$Path) {
    $field = $Dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants,
        [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '1001'))
    $valuePattern = $null
    if ($field -and $field.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$valuePattern)) {
        $valuePattern.SetValue($Path)
        return
    }
    $control = Wait-Until {
        $candidate = Find-NativeControl $Dialog 1001 'Edit'
        if ($candidate -ne [IntPtr]::Zero) { $candidate }
    } 'Native File Name Field'
    $null = [NativeSmokeDialogs]::SendMessage($control, 0x000C, [IntPtr]::Zero, $Path)
}

function Send-AppKeys([string]$Keys) {
    $button = Wait-Until {
        $candidate = Find-Element $script:window 'Import DOCX' ([System.Windows.Automation.ControlType]::Button)
        if ($candidate -and $candidate.Current.IsEnabled -and $script:window.Current.IsEnabled) { $candidate }
    } 'Application Ready For Keyboard Input'
    $null = [NativeSmokeDialogs]::SetForegroundWindow([IntPtr]$script:window.Current.NativeWindowHandle)
    $button.SetFocus()
    [System.Windows.Forms.SendKeys]::SendWait($Keys)
}

function Wait-AppText([string]$Text) {
    $null = Wait-Until {
        $condition = [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty, $Text)
        $script:window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
    } "Application Text $Text"
}

$executablePath = (Resolve-Path -LiteralPath $Executable).Path
if ($Dialogs -and (Get-Process -Name insyndoc -ErrorAction SilentlyContinue)) {
    throw 'Close Other InsynDoc Windows Before Running The Interactive Dialog Checks.'
}
$outputDirectory = Join-Path ([IO.Path]::GetTempPath()) ("InsynDoc-Native-Smoke-" + [Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $outputDirectory
$savedPath = Join-Path $outputDirectory 'Native Smoke.docx'
$corruptPath = Join-Path $outputDirectory 'Malformed.docx'
[IO.File]::WriteAllBytes($corruptPath, [byte[]](80, 75, 3, 4, 0, 0))
$previousBrowserArguments = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "$previousBrowserArguments --force-renderer-accessibility".Trim()
$script:application = $null
$completed = $false
try {
    $script:application = Start-Process -FilePath $executablePath -PassThru -WindowStyle Hidden
    $handle = Wait-Until {
        $script:application.Refresh()
        if ($script:application.HasExited) { throw 'The Compiled Application Exited Before Creating A Window.' }
        if ($script:application.MainWindowHandle -ne [IntPtr]::Zero) { $script:application.MainWindowHandle }
    } 'Real Native Window Creation'
    $script:window = [System.Windows.Automation.AutomationElement]::FromHandle($handle)
    $null = [NativeSmokeDialogs]::ShowWindow($handle, 5)
    Write-Output "PASS: Compiled Native Window Created ($executablePath)"
    if ($Dialogs -and -not $WindowOnly) {
        $null = Wait-Until {
            $button = Find-Element $script:window 'Import DOCX' ([System.Windows.Automation.ControlType]::Button)
            if ($button -and $button.Current.IsEnabled) { $button }
        } 'WebView2 Editor Readiness'
        Invoke-Button $script:window 'Import DOCX'
        $dialog = Get-Dialog 'Open Document'
        Invoke-Button $dialog 'Cancel'
        Wait-AppText 'Untitled.docx'
        Write-Output 'PASS: Native Open Cancel Preserved Document Identity'

        Send-AppKeys '^+s'
        $dialog = Get-Dialog 'Export DOCX Copy'
        Invoke-Button $dialog 'Cancel'
        Wait-AppText 'Untitled.docx'
        if (Test-Path -LiteralPath $savedPath) { throw 'Cancelled Export Created A File.' }
        Write-Output 'PASS: Native Export Cancel Preserved Document Identity'

        Send-AppKeys '^+s'
        $dialog = Get-Dialog 'Export DOCX Copy'
        Set-DialogPath $dialog $savedPath
        Invoke-Button $dialog 'Save'
        $null = Wait-Until { Test-Path -LiteralPath $savedPath } 'Real DOCX Export'
        $archive = [IO.Compression.ZipFile]::OpenRead($savedPath)
        try {
            if (-not $archive.GetEntry('word/document.xml')) { throw 'Export Did Not Contain A Word Document.' }
        } finally { $archive.Dispose() }
        Wait-AppText 'Untitled.docx'
        Write-Output 'PASS: Native Save Dialog Wrote A DOCX Package Without Replacing The Working File'

        Invoke-Button $script:window 'Import DOCX'
        $dialog = Get-Dialog 'Open Document'
        Set-DialogPath $dialog $savedPath
        Invoke-Button $dialog 'Open'
        Wait-AppText 'Native Smoke.docx'
        Write-Output 'PASS: Native Open Loaded The Exported File'

        Invoke-Button $script:window 'Import DOCX'
        $dialog = Get-Dialog 'Open Document'
        Set-DialogPath $dialog $corruptPath
        Invoke-Button $dialog 'Open'
        $null = Wait-Until {
            $script:window.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -like '*Could Not*' } | Select-Object -First 1
        } 'Real Malformed File Failure'
        Wait-AppText 'Native Smoke.docx'
        Write-Output 'PASS: Native Malformed Open Failed And Kept The Working File'

        [IO.File]::SetAttributes($savedPath, [IO.FileAttributes]::ReadOnly)
        try {
            Send-AppKeys '^s'
            $null = Wait-Until {
                $script:window.FindAll([System.Windows.Automation.TreeScope]::Descendants,
                    [System.Windows.Automation.Condition]::TrueCondition) | Where-Object { $_.Current.Name -like '*Could Not Replace*' } | Select-Object -First 1
            } 'Real Read Only Save Failure'
            Wait-AppText 'Native Smoke.docx'
        } finally { [IO.File]::SetAttributes($savedPath, [IO.FileAttributes]::Normal) }
        Write-Output 'PASS: Native Read Only Save Reported Failure And Kept The Working File'
    }
    $script:window.GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern).Close()
    if (-not $script:application.WaitForExit(15000)) { throw 'Normal Native Window Close Did Not Exit The Application.' }
    if ($script:application.ExitCode -ne 0) { throw "Native Application Exit Code: $($script:application.ExitCode)" }
    Write-Output 'PASS: Normal Native Window Close Exited Successfully'
    $completed = $true
} finally {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previousBrowserArguments
    if ($script:application -and -not $script:application.HasExited) {
        Stop-Process -Id $script:application.Id
        if (-not $completed) { Write-Output 'Cleanup: Terminated Only The Test Application After Failure' }
    }
    Write-Output "Native Smoke Artifacts: $outputDirectory"
}
