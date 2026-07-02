@echo off
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=amd64
if errorlevel 1 exit /b 1
set "PATH=%USERPROFILE%\.cargo\bin;C:\Program Files\nodejs;C:\Program Files\Git\cmd;%PATH%"
where link
where cl
cargo --version
npm.cmd run tauri build
