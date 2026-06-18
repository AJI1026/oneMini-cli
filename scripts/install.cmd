@echo off
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/AJI1026/OneMini-CLI/main/scripts/install.ps1 | iex"
if errorlevel 1 pause
