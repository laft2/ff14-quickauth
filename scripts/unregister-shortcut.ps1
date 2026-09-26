$startMenu = [System.IO.Path]::Combine($env:APPDATA, 'Microsoft', 'Windows', 'Start Menu', 'Programs')
$shortcuts = @('FF14 QuickAuth.lnk', 'FF14 Companion.lnk')

foreach ($name in $shortcuts) {
    $path = Join-Path $startMenu $name
    if (Test-Path $path) {
        Remove-Item $path -Force
        Write-Host "✅ スタートメニューからショートカットを削除しました: $path"
    }
}

