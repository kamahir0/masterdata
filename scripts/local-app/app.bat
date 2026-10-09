@echo off
setlocal
node "%~dp0app.mjs" %*
set "STATUS=%ERRORLEVEL%"
if not "%STATUS%"=="0" (
  echo.
  echo Local app workflow failed ^(exit %STATUS%^). Press any key to close.
  pause >nul
)
exit /b %STATUS%
