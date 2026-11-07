@echo off
setlocal
cd /d "%~dp0"
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
if exist "%~dp0.tools\mingw64\bin\gcc.exe" set "PATH=%~dp0.tools\mingw64\bin;%PATH%"
if /i "%~1"=="build" goto build
if not exist "native\target\release\insyndoc-native.exe" goto build

powershell.exe -NoProfile -Command "$ErrorActionPreference = 'Stop'; $built = (Get-Item -LiteralPath 'native/target/release/insyndoc-native.exe').LastWriteTimeUtc; $inputs = Get-ChildItem -LiteralPath 'native/src','native/fonts','public/branding','src-tauri/icons/insyndoc/icon.ico','native/build.rs','native/Cargo.toml','native/Cargo.lock' -Recurse -File; if ($inputs | Where-Object { $_.LastWriteTimeUtc -gt $built } | Select-Object -First 1) { exit 1 }; exit 0"
if errorlevel 1 goto build
goto launch

:build
where cargo.exe >nul 2>nul
if errorlevel 1 goto missing_rust
where gcc.exe >nul 2>nul
if errorlevel 1 goto missing_linker
echo Building The Latest InsynDoc. Future Launches Skip This Unless Files Change.
call cargo +stable-x86_64-pc-windows-gnu build --release --locked --manifest-path native/Cargo.toml
if errorlevel 1 goto failed

:launch
start "" "%~dp0native\target\release\insyndoc-native.exe"
exit /b 0

:missing_rust
echo Rust Is Required. Install Rust And The stable-x86_64-pc-windows-gnu Toolchain.
goto failed

:missing_linker
echo The MinGW GNU Linker Is Required. Add MinGW-w64 bin To PATH Or Place It In .tools\mingw64\bin.
echo See native\README.md For The Microsoft Build Tools Alternative.
goto failed

:failed
echo.
echo InsynDoc Could Not Start. Review The Error Above.
pause
exit /b 1
