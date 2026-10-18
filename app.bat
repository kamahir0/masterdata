@echo off
setlocal
node "%~dp0scripts\local-app.mjs" %*
set "STATUS=%ERRORLEVEL%"
if not "%STATUS%"=="0" (
  echo.
  echo Build or install failed. Press any key to close.
  if not defined CI pause >nul
)
exit /b %STATUS%
