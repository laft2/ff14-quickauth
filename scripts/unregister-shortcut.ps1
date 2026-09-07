$startMenu = [System.IO.Path]::Combine($env:APPDATA, 'Microsoft', 'Windows', 'Start Menu', 'Programs')
$shortcutPath = Join-Path $startMenu 'FF14 Companion.lnk'

if (Test-Path $shortcutPath) {
    Remove-Item $shortcutPath
    Write-Host "✅ スタートメニューからショートカットを削除しました: $shortcutPath"
} else {
    Write-Host "ℹ️ ショートカットは既に削除されているか存在しません"
}
