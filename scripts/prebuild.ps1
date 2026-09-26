# Terminate any running ff14-companion or ff14-quickauth process to prevent Windows file lock during build
$procs = Get-Process ff14-companion, ff14-quickauth -ErrorAction SilentlyContinue
if ($procs) {
    Write-Host "ℹ️ ビルド前に実行中の FF14 Companion / QuickAuth プロセスを終了します..."
    $procs | Stop-Process -Force
}

