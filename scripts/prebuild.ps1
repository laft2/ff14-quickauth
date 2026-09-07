# Terminate any running ff14-companion process to prevent Windows file lock during build
$procs = Get-Process ff14-companion -ErrorAction SilentlyContinue
if ($procs) {
    Write-Host "ℹ️ ビルド前に実行中の FF14 Companion プロセスを終了します..."
    $procs | Stop-Process -Force
}
