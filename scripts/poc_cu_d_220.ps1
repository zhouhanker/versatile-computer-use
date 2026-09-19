# CU-D-220: Notepad key Return without confirm_send is denied (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-220: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-220-" + [guid]::NewGuid().ToString())
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
  if (-not $ref) {
    $first = @($snap.data.dom_refs)[0]
    if ($null -ne $first) { $ref = [string]$first.ref }
  }
  if (-not $ref) { throw "empty notepad scene" }
  Write-Host ("SNAP_OK ref={0}" -f $ref)

  $actionPath = Join-Path $ud "key.json"
  $action = @{
    type = "key"
    target = @{ tab_id = $tab; ref = $ref }
    args = @{ key = "return"; tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $actionPath -Value $action -Encoding ASCII
  $keyRaw = & $vcu --user-dir $ud act --session $sid --action-json $actionPath --json
  Write-Host $keyRaw
  $keyed = $keyRaw | ConvertFrom-Json
  if ($keyed.ok) { throw "key return must be refused" }
  $code = [string]$keyed.error.code
  if ($code -ne "FocusPolicyViolation") { throw "key return expected FocusPolicyViolation" }
  Write-Host "KEY_DENIED"

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-220 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
