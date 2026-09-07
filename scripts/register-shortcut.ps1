$ErrorActionPreference = 'Stop'
$startMenu = [System.IO.Path]::Combine($env:APPDATA, 'Microsoft', 'Windows', 'Start Menu', 'Programs')
$exePath = (Resolve-Path 'src-tauri/target/release/ff14-companion.exe').Path
$shortcutPath = Join-Path $startMenu 'FF14 Companion.lnk'

$wsh = New-Object -ComObject WScript.Shell
$shortcut = $wsh.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exePath
$shortcut.WorkingDirectory = (Split-Path $exePath)
$shortcut.Description = 'FF14 Companion 認証補完マネージャー'
$shortcut.Save()

Write-Host "✅ スタートメニューにショートカットを追加しました: $shortcutPath"
