$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
Push-Location $projectDirectory
try {
    node scripts/tauri.mjs --cargo test --manifest-path src-tauri/Cargo.toml --no-run
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    # Native Test Executables Need The Common Controls Manifest Used By The App.
    $manifest = @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0"><dependency><dependentAssembly><assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*" /></dependentAssembly></dependency></assembly>
'@
    Get-ChildItem -LiteralPath 'src-tauri/target/debug/deps' -Filter 'bridge-*.exe' | ForEach-Object {
        Set-Content -LiteralPath ($_.FullName + '.manifest') -Value $manifest -Encoding UTF8
    }

    node scripts/tauri.mjs --cargo test --manifest-path src-tauri/Cargo.toml
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
finally {
    Pop-Location
}
