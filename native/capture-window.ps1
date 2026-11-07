param([string]$ProcessName = "insyndoc-native", [string]$Out = "D:\Github\InsynDoc\native\new-app.png")
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class Win32Cap {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int c);
    public struct RECT { public int Left, Top, Right, Bottom; }
}
"@
$p = Get-Process $ProcessName -ErrorAction Stop | Select-Object -First 1
[Win32Cap]::ShowWindow($p.MainWindowHandle, 9) | Out-Null
[Win32Cap]::SetForegroundWindow($p.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 800
$rect = New-Object Win32Cap+RECT
[Win32Cap]::GetWindowRect($p.MainWindowHandle, [ref]$rect) | Out-Null
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top
Write-Output "rect $($rect.Left),$($rect.Top) ${w}x${h}"
if ($w -le 0 -or $h -le 0) { Write-Output "bad rect"; exit 1 }
$vs = [System.Windows.Forms.SystemInformation]::VirtualScreen
if ($rect.Left -lt $vs.Left) { $rect.Left = $vs.Left }
if ($rect.Top -lt $vs.Top) { $rect.Top = $vs.Top }
if ($rect.Right -gt $vs.Right) { $rect.Right = $vs.Right }
if ($rect.Bottom -gt $vs.Bottom) { $rect.Bottom = $vs.Bottom }
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$bmp.Save($Out)
$g.Dispose(); $bmp.Dispose()
Write-Output "saved $Out"
