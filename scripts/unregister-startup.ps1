$ErrorActionPreference = 'Stop'
$startupFolder = [System.IO.Path]::Combine($env:APPDATA, 'Microsoft', 'Windows', 'Start Menu', 'Programs', 'Startup')
$shortcutPath = Join-Path $startupFolder 'FF14 Companion.lnk'

if (Test-Path $shortcutPath) {
    Remove-Item $shortcutPath -Force
    Write-Host "✅ スタートアップフォルダからショートカットを削除しました: $shortcutPath"
} else {
    Write-Host "ℹ️ スタートアップフォルダにショートカットが存在しませんでした: $shortcutPath"
}
