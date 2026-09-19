# CU-D-110: vcu desktop screenshot of Notepad via PrintWindow (no SendInput / CopyFromScreen).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-110: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-110-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

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

  $shotRaw = & $vcu --user-dir $ud screenshot --session $sid --tab $tab --json
  Write-Host $shotRaw
  $shot = $shotRaw | ConvertFrom-Json
  if (-not $shot.ok) { throw "screenshot failed" }
  $path = [string]$shot.data.path
  $bytes = [int]$shot.data.bytes
  $w = [int]$shot.data.width
  $h = [int]$shot.data.height
  if ($bytes -lt 24) { throw "screenshot too small" }
  if ($w -lt 8 -or $h -lt 8) { throw "screenshot dimensions too small" }
  if (-not (Test-Path -LiteralPath $path)) { throw "screenshot path missing" }
  $fs = [System.IO.File]::OpenRead($path)
  try {
    $b0 = $fs.ReadByte()
    $b1 = $fs.ReadByte()
  } finally { $fs.Close() }
  if ($b0 -ne 0x89 -or $b1 -ne 0x50) { throw "screenshot is not a PNG" }
  Write-Host ("SHOT_OK bytes={0} width={1} height={2}" -f $bytes, $w, $h)

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-110 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
