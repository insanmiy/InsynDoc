$ErrorActionPreference = 'Stop'
$testDirectory = Join-Path ([System.IO.Path]::GetTempPath()) ('InsynDoc Build Guard ' + [Guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $testDirectory
$loaderPath = Join-Path $testDirectory 'WebView2Loader.dll'
$guardPath = Join-Path $PSScriptRoot '..\scripts\prepare-build.ps1'
$originalBytes = [byte[]](1, 2, 3, 4)
[System.IO.File]::WriteAllBytes($loaderPath, $originalBytes)
$handle = $null

try {
    $handle = [System.IO.File]::Open($loaderPath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $guardPath -TargetDirectory $testDirectory -WaitSeconds 0
    if ($LASTEXITCODE -ne 1) { throw 'The Build Guard Did Not Stop For A Locked WebView2 Loader.' }
    $handle.Dispose()
    $handle = $null
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $guardPath -TargetDirectory $testDirectory -WaitSeconds 0
    if ($LASTEXITCODE -ne 0) { throw 'The Build Guard Did Not Continue After The File Was Released.' }
    if ([Convert]::ToBase64String([System.IO.File]::ReadAllBytes($loaderPath)) -ne [Convert]::ToBase64String($originalBytes)) { throw 'The Build Guard Changed File Contents.' }
    Write-Host 'Passed: Locked Loader Blocks The Build, Released Loader Allows It, And File Contents Stay Intact.'
} finally {
    if ($null -ne $handle) { $handle.Dispose() }
    Remove-Item -LiteralPath $loaderPath -Force
    Remove-Item -LiteralPath $testDirectory
}
