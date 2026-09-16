@echo off
rem Configure and build VPP with the Visual Studio 2019 toolchain, CMake, and Ninja.
rem Usage: build.cmd [Debug|Release]

setlocal
set CONFIG=%1
if "%CONFIG%"=="" set CONFIG=Release

set VS=C:\Program Files (x86)\Microsoft Visual Studio\2019\Professional
call "%VS%\VC\Auxiliary\Build\vcvars64.bat" >nul
set PATH=%VS%\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin;%VS%\Common7\IDE\CommonExtensions\Microsoft\CMake\Ninja;%PATH%

cmake -S "%~dp0." -B "%~dp0build" -G Ninja -DCMAKE_BUILD_TYPE=%CONFIG% || exit /b 1
cmake --build "%~dp0build" || exit /b 1

echo.
echo Built: %~dp0build\viewer\vpp_viewer.exe
endlocal
