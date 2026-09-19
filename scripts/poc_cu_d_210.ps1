# CU-D-210: wait miss times out honestly (wrong value / missing ref). No SendInput.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-210: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-210-" + [guid]::NewGuid().ToString())
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

  $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
  Write-Host $snapRaw
  $snap = $snapRaw | ConvertFrom-Json
  if (-not $snap.ok) { throw "snapshot failed" }
  $ref = $null
  foreach ($r in @($snap.data.dom_refs)) {
    if ([string]$r.role -like "*Edit*") { $ref = [string]$r.ref; break }
  }
  if (-not $ref) { throw "no Edit ref" }
  Write-Host ("SNAP_OK ref={0}" -f $ref)

  $have = "VCU-D-210-HAVE"
  $typeRaw = & $vcu --user-dir $ud type --session $sid --tab $tab --ref $ref --text $have --json
  Write-Host $typeRaw
  $typed = $typeRaw | ConvertFrom-Json
  if (-not $typed.ok) { throw "type failed" }

  $miss = "VCU-D-210-MISS"
  $actionPath = Join-Path $ud "wait-miss.json"
  $action = @{
    type = "wait"
    target = @{ tab_id = $tab }
    args = @{ ms = 800; value = $miss; tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $actionPath -Value $action -Encoding ASCII
  $waitRaw = & $vcu --user-dir $ud act --session $sid --action-json $actionPath --json
  Write-Host $waitRaw
  $waited = $waitRaw | ConvertFrom-Json
  if ($waited.ok) { throw "wait miss must fail" }
  $code = [string]$waited.error.code
  $msg = [string]$waited.error.message
  if ($code -ne "ActionFailed") { throw "wait miss expected ActionFailed" }
  if ($msg -notlike "*timed out*") { throw "wait miss missing timed out" }
  if ($msg -notlike "*VCU-D-210-MISS*") { throw "wait miss missing value in timeout" }
  Write-Host "WAIT_MISS_OK"

  $refPath = Join-Path $ud "wait-ref.json"
  $refAction = @{
    type = "wait"
    target = @{ tab_id = $tab; ref = "e999" }
    args = @{ ms = 800; tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $refPath -Value $refAction -Encoding ASCII
  $refRaw = & $vcu --user-dir $ud act --session $sid --action-json $refPath --json
  Write-Host $refRaw
  $refWait = $refRaw | ConvertFrom-Json
  if ($refWait.ok) { throw "missing ref wait must fail" }
  $rcode = [string]$refWait.error.code
  $rmsg = [string]$refWait.error.message
  if ($rcode -ne "ActionFailed") { throw "missing ref expected ActionFailed" }
  if ($rmsg -notlike "*timed out*") { throw "missing ref missing timed out" }
  if ($rmsg -notlike "*e999*") { throw "missing ref not in timeout" }
  Write-Host "WAIT_REF_MISS_OK"

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-210 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
