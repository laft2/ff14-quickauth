$ErrorActionPreference = 'Stop'
$startupFolder = [System.IO.Path]::Combine($env:APPDATA, 'Microsoft', 'Windows', 'Start Menu', 'Programs', 'Startup')
$exePath = (Resolve-Path 'src-tauri/target/release/ff14-quickauth.exe').Path
$shortcutPath = Join-Path $startupFolder 'FF14 QuickAuth.lnk'
$oldShortcutPath = Join-Path $startupFolder 'FF14 Companion.lnk'

# Clean up legacy startup shortcut if it exists
if (Test-Path $oldShortcutPath) {
    Remove-Item $oldShortcutPath -Force
    Write-Host "🧹 古いスタートアップショートカットを削除しました: $oldShortcutPath"
}

$wsh = New-Object -ComObject WScript.Shell
$shortcut = $wsh.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $exePath
$shortcut.WorkingDirectory = (Split-Path $exePath)
$shortcut.Description = 'FF14 QuickAuth 認証補完マネージャー'
$shortcut.Save()

Write-Host "✅ スタートアップフォルダにショートカットを追加しました: $shortcutPath"
