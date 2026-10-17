@echo off
setlocal
title Nightreign Force Stop
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Stop-Nightreign.ps1" %*
set "cleanup_result=%errorlevel%"
echo.
pause
exit /b %cleanup_result%
