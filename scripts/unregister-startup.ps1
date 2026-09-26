$ErrorActionPreference = 'Stop'
$startupFolder = [System.IO.Path]::Combine($env:APPDATA, 'Microsoft', 'Windows', 'Start Menu', 'Programs', 'Startup')
$shortcuts = @('FF14 QuickAuth.lnk', 'FF14 Companion.lnk')

foreach ($name in $shortcuts) {
    $path = Join-Path $startupFolder $name
    if (Test-Path $path) {
        Remove-Item $path -Force
        Write-Host "✅ スタートアップフォルダからショートカットを削除しました: $path"
    }
}

