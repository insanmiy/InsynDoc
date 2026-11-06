param(
    [Parameter(Mandatory = $true)][int]$ApplicationId,
    [Parameter(Mandatory = $true)][string]$Destination,
    [Parameter(Mandatory = $true)][string]$DialogTitle
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class DownloadDialog {
    public delegate bool Callback(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumChildWindows(IntPtr parent, Callback callback, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern int GetClassName(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr parameter, string text);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, IntPtr parameter, IntPtr data);
}
'@

$deadline = [DateTime]::UtcNow.AddSeconds(20)
$script:dialogHandle = [IntPtr]::Zero
do {
    $findDialog = [DownloadDialog+Callback] {
        param($window, $data)
        [uint32]$owner = 0
        $null = [DownloadDialog]::GetWindowThreadProcessId($window, [ref]$owner)
        if ($owner -ne $ApplicationId) { return $true }
        $title = [Text.StringBuilder]::new(512)
        $null = [DownloadDialog]::GetWindowText($window, $title, $title.Capacity)
        if ($title.ToString() -eq $DialogTitle) { $script:dialogHandle = $window }
        return $true
    }
    $null = [DownloadDialog]::EnumWindows($findDialog, [IntPtr]::Zero)
    if ($script:dialogHandle -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 100
} while ([DateTime]::UtcNow -lt $deadline)
if ($script:dialogHandle -eq [IntPtr]::Zero) { throw 'The Download Save Dialog Did Not Open.' }

$dialog = [System.Windows.Automation.AutomationElement]::FromHandle($script:dialogHandle)
$field = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants,
    [System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '1001'))
$pattern = $null
$setPath = $field -and $field.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern, [ref]$pattern)
if ($setPath) { $pattern.SetValue($Destination) }

$script:saveButton = [IntPtr]::Zero
$script:pathField = [IntPtr]::Zero
$findControls = [DownloadDialog+Callback] {
    param($window, $data)
    $identifier = [DownloadDialog]::GetDlgCtrlID($window)
    $className = [Text.StringBuilder]::new(128)
    $null = [DownloadDialog]::GetClassName($window, $className, $className.Capacity)
    if ($identifier -eq 1 -and $className.ToString() -eq 'Button') { $script:saveButton = $window }
    if ($identifier -eq 1001 -and $className.ToString() -eq 'Edit') { $script:pathField = $window }
    return $true
}
$null = [DownloadDialog]::EnumChildWindows($script:dialogHandle, $findControls, [IntPtr]::Zero)
if (-not $setPath) {
    if ($script:pathField -eq [IntPtr]::Zero) { throw 'The Download File Name Field Is Unavailable.' }
    $null = [DownloadDialog]::SendMessage($script:pathField, 0x000C, [IntPtr]::Zero, $Destination)
}
if ($script:saveButton -eq [IntPtr]::Zero) { throw 'The Download Save Button Is Unavailable.' }
if (-not [DownloadDialog]::PostMessage($script:saveButton, 0x00F5, [IntPtr]::Zero, [IntPtr]::Zero)) { throw 'The Download Save Button Could Not Be Activated.' }
