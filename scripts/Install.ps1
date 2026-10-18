param([switch]$NoStart)
$ErrorActionPreference = 'Stop'
$package = Split-Path $PSScriptRoot -Parent
if (-not (Test-Path (Join-Path $package 'VoiceToText.exe'))) { throw 'Extract the entire Windows package first.' }
& (Join-Path $PSScriptRoot 'Verify-Package.ps1')
if (Get-Process VoiceToText -ErrorAction SilentlyContinue) { throw 'Exit VoiceToText from its tray before installing/updating.' }
$destination = Join-Path $env:LOCALAPPDATA 'Programs\VoiceToText'
New-Item -ItemType Directory -Force -Path $destination | Out-Null
Get-ChildItem -LiteralPath $package | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $destination -Force -Recurse }
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) 'VoiceToText.lnk'))
$shortcut.TargetPath = Join-Path $destination 'VoiceToText.exe'
$shortcut.WorkingDirectory = $destination
$shortcut.Save()
Write-Host "Installed locally at $destination. Settings remain in LOCALAPPDATA\VoiceToText."
if (-not $NoStart) { Start-Process -FilePath (Join-Path $destination 'VoiceToText.exe') }
