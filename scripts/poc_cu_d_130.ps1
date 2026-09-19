# CU-D-130: vcu desktop reveal_path selects a throwaway file in Explorer (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-130: not Windows_NT"
  exit 0
}

$root = (Get-Location).Path
$bin = Join-Path $root "target\debug"
$vcu = Join-Path $bin "vcu.exe"
$daemonBin = Join-Path $bin "vcu-daemon.exe"
if (-not (Test-Path $vcu)) {
  throw "missing vcu.exe under target/debug; build vcu-cli and vcu-daemon first"
}
$env:Path = "$bin;$env:Path"
$ud = Join-Path $env:TEMP ("vcu-130-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null
$openme = Join-Path $ud "OPENME"
New-Item -ItemType Directory -Force -Path $openme | Out-Null
$marker = Join-Path $openme "MARKER.txt"
Set-Content -LiteralPath $marker -Value "vcu-130" -Encoding ASCII

& $vcu --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

$notepad = Start-Process -FilePath "notepad.exe" -PassThru
Start-Sleep -Seconds 1
$daemon = $null
try {
  $daemon = Start-Process -FilePath $daemonBin -ArgumentList @("--user-dir", $ud) -PassThru -WindowStyle Hidden
  $ok = $false
  for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 200
    & $vcu --user-dir $ud daemon status | Out-Null
    if ($LASTEXITCODE -eq 0) { $ok = $true; break }
  }
  if (-not $ok) { throw "daemon health failed" }

  $tab = "win:notepad:$($notepad.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK session={0}" -f $sid)

  $actionPath = Join-Path $ud "action.json"
  $action = @{
    type = "reveal"
    target = @{ tab_id = $tab }
    args = @{ path = $marker; tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $actionPath -Value $action -Encoding ASCII
  $actRaw = & $vcu --user-dir $ud act --session $sid --action-json $actionPath --json
  Write-Host $actRaw
  $acted = $actRaw | ConvertFrom-Json
  if (-not $acted.ok) { throw "reveal failed" }
  $pathName = [string]$acted.data.detail.input_path
  $cursor = [bool]$acted.data.detail.os_cursor_used
  if ($cursor) { throw "os_cursor_used true" }
  if ($pathName -ne "explorer_reveal") { throw "unexpected input_path" }
  Write-Host ("REVEAL_OK path={0} os_cursor_used={1}" -f $pathName, $cursor)

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-130 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
