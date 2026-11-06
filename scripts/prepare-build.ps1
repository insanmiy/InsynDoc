param(
    [string]$TargetDirectory = (Join-Path $PSScriptRoot '..\src-tauri\target\release'),
    [int]$WaitSeconds = 120
)

$ErrorActionPreference = 'Stop'
$buildDirectory = [System.IO.Path]::GetFullPath($TargetDirectory)
$executablePath = Join-Path $buildDirectory 'insyndoc.exe'
$deadline = [DateTime]::UtcNow.AddSeconds($WaitSeconds)
$announced = $false
$requestedProcesses = @{}

while ($true) {
    $runningApps = @(Get-Process -Name insyndoc -ErrorAction SilentlyContinue | Where-Object {
        $_.Path -and [string]::Equals($_.Path, $executablePath, [StringComparison]::OrdinalIgnoreCase)
    })
    foreach ($runningApp in $runningApps) {
        if ($requestedProcesses.ContainsKey($runningApp.Id)) { continue }
        Write-Host 'InsynDoc Is Open. Save Any Changes And Close The App To Continue Building.'
        $requestedProcesses[$runningApp.Id] = $true
        $null = $runningApp.CloseMainWindow()
    }

    $lockedFile = $null
    foreach ($filename in @('insyndoc.exe', 'WebView2Loader.dll')) {
        $filePath = Join-Path $buildDirectory $filename
        if (-not (Test-Path -LiteralPath $filePath -PathType Leaf)) { continue }
        try {
            $handle = [System.IO.File]::Open($filePath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
            $handle.Dispose()
        } catch [System.IO.IOException] {
            $errorCode = $_.Exception.HResult -band 65535
            if ($errorCode -ne 32 -and $errorCode -ne 33) { throw }
            $lockedFile = $filename
            break
        }
    }

    if ($runningApps.Count -eq 0 -and $null -eq $lockedFile) { exit 0 }
    if (-not $announced) {
        Write-Host 'Waiting For InsynDoc Build Files To Be Released. No App Will Be Force Closed.'
        $announced = $true
    }
    if ([DateTime]::UtcNow -ge $deadline) {
        Write-Host 'Build Stopped Because InsynDoc Files Are Still In Use. Close The App And Run Start.bat Again.'
        exit 1
    }
    Start-Sleep -Milliseconds 500
}
