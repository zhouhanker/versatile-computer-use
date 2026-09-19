# CU-D-200: wait until Notepad scene value contains typed marker (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-200: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-200-" + [guid]::NewGuid().ToString())
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

  $mark = "VCU-D-200-MARK"
  $typeRaw = & $vcu --user-dir $ud type --session $sid --tab $tab --ref $ref --text $mark --json
  Write-Host $typeRaw
  $typed = $typeRaw | ConvertFrom-Json
  if (-not $typed.ok) { throw "type failed" }

  $actionPath = Join-Path $ud "wait.json"
  $action = @{
    type = "wait"
    target = @{ tab_id = $tab }
    args = @{ ms = 4000; value = $mark; tab_id = $tab }
  } | ConvertTo-Json -Compress
  Set-Content -LiteralPath $actionPath -Value $action -Encoding ASCII
  $waitRaw = & $vcu --user-dir $ud act --session $sid --action-json $actionPath --json
  Write-Host $waitRaw
  $waited = $waitRaw | ConvertFrom-Json
  if (-not $waited.ok) { throw "wait failed" }
  $path = [string]$waited.data.detail.input_path
  $found = [string]$waited.data.detail.found_ref
  $cursor = [bool]$waited.data.detail.os_cursor_used
  if ($cursor) { throw "os_cursor_used true" }
  if ($path -ne "scene_wait") { throw "unexpected input_path" }
  if (-not $found) { throw "wait found_ref empty" }
  Write-Host ("WAIT_OK path={0} found_ref={1} os_cursor_used={2}" -f $path, $found, $cursor)

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-200 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
