# CU-D-090: vcu desktop session on Windows Notepad (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-090: not Windows_NT"
  exit 0
}

$root = (Get-Location).Path
$bin = Join-Path $root "target/debug"
if (-not (Test-Path (Join-Path $bin "vcu.exe"))) {
  throw "missing $bin/vcu.exe — build vcu-cli and vcu-daemon first"
}
$env:Path = "$bin;$env:Path"
$ud = Join-Path $env:TEMP ("vcu-090-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

& "$bin/vcu.exe" --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

$notepad = Start-Process -FilePath "notepad.exe" -PassThru
Start-Sleep -Seconds 1
$daemon = $null
try {
  $daemon = Start-Process -FilePath "$bin/vcu-daemon.exe" -ArgumentList @("--user-dir", $ud) -PassThru -WindowStyle Hidden
  $ok = $false
  for ($i = 0; $i -lt 30; $i++) {
    Start-Sleep -Milliseconds 200
    try {
      $h = & "$bin/vcu.exe" --user-dir $ud daemon status 2>$null
      if ($LASTEXITCODE -eq 0) { $ok = $true; break }
    } catch {}
  }
  if (-not $ok) { throw "daemon health failed" }

  $tab = "win:notepad:$($notepad.Id)"
  $startRaw = & "$bin/vcu.exe" --user-dir $ud session start --surface desktop --app-id $tab --json
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed: $startRaw" }
  if ($start.data.stage_hud -ne $true) { throw "stage_hud was not true: $startRaw" }
  $presenter = [string]$start.data.stage_presenter
  if ($presenter -ne "winforms") { throw "stage_presenter=$presenter expected winforms" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK presenter={0} session={1}" -f $presenter, $sid)

  $snapRaw = & "$bin/vcu.exe" --user-dir $ud snapshot --session $sid --tab $tab --json
  $snap = $snapRaw | ConvertFrom-Json
  if (-not $snap.ok) { throw "snapshot failed: $snapRaw" }
  $src = [string]$snap.data.source
  if ($src -ne "uia_scene") { throw "source=$src expected uia_scene" }
  $ref = $null
  foreach ($r in @($snap.data.dom_refs)) {
    $role = [string]$r.role
    if ($role -like "*Edit*") { $ref = [string]$r.ref; break }
  }
  if (-not $ref) { throw "no Edit ref in snapshot: $snapRaw" }
  Write-Host ("SNAP_OK source={0} ref={1}" -f $src, $ref)

  $typeRaw = & "$bin/vcu.exe" --user-dir $ud type --session $sid --tab $tab --ref $ref --text "VCU-D-090" --json
  $typed = $typeRaw | ConvertFrom-Json
  if (-not $typed.ok) { throw "type failed: $typeRaw" }
  $path = [string]$typed.data.detail.input_path
  $cursor = [bool]$typed.data.detail.os_cursor_used
  if ($cursor) { throw "os_cursor_used true" }
  if ($path -ne "wm_settext" -and $path -ne "uia_set_value") {
    throw "unexpected input_path=$path"
  }
  Write-Host ("TYPE_OK path={0} os_cursor_used={1}" -f $path, $cursor)

  & "$bin/vcu.exe" --user-dir $ud session abort $sid --json | Out-Null
  Write-Host "CU-D-090 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
