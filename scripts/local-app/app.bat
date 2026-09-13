@echo off
setlocal
set "SCRIPT_DIR=%~dp0"
for %%I in ("%SCRIPT_DIR%\..\..") do set "REPOSITORY_ROOT=%%~fI"
pushd "%REPOSITORY_ROOT%"
cargo xtask app reinstall
set "STATUS=%ERRORLEVEL%"
popd
if not "%STATUS%"=="0" (
  echo.
  echo local app workflow failed (exit %STATUS%). Press any key to close.
  pause >nul
)
exit /b %STATUS%
