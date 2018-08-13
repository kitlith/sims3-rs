@echo off
:: Change this if your custom content is in a different directory.
SET CCDIR="%USERPROFILE%\Documents\Electronic Arts\CC Magic\Content\Packages"

if "%~1"=="" (
    echo Usage: Drag and drop a sim package file onto this batch file.
) else (
    find_merged_cc.exe %1 %CCDIR%
)
pause
