# CU-D-240: Windows session abort tears down WinForms HUD (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-240: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-240-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

function Get-VcuStageScripts {
  @(Get-ChildItem -LiteralPath $env:TEMP -Filter "vcu-stage-*.ps1" -ErrorAction SilentlyContinue)
}

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

  $before = Get-VcuStageScripts
  $tab = "win:notepad:$($notepad.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  if ($start.data.stage_hud -ne $true) { throw "stage_hud was not true" }
  $presenter = [string]$start.data.stage_presenter
  if ($presenter -ne "winforms") { throw "stage_presenter expected winforms" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK presenter={0} session={1}" -f $presenter, $sid)

  $hudScripts = @(Get-VcuStageScripts | Where-Object {
    $n = $_.FullName
    -not ($before | Where-Object { $_.FullName -eq $n })
  })
  if ($hudScripts.Count -lt 1) { throw "WinForms stage script missing after session start" }
  Write-Host ("HUD_UP count={0}" -f $hudScripts.Count)

  $abortRaw = & $vcu --user-dir $ud session abort $sid --json
  Write-Host $abortRaw
  $aborted = $abortRaw | ConvertFrom-Json
  if (-not $aborted.ok) { throw "abort failed" }
  if ($aborted.data.aborted -ne $true) { throw "aborted was not true" }
  if ($aborted.data.hud -ne $false) { throw "hud was not false" }
  Write-Host "ABORT_OK hud=false"

  $listRaw = & $vcu --user-dir $ud session list --json
  Write-Host $listRaw
  $listed = $listRaw | ConvertFrom-Json
  $still = $false
  foreach ($s in @($listed.data)) {
    if ([string]$s.session_id -eq $sid) { $still = $true }
  }
  if ($still) { throw "aborted session still listed" }
  Write-Host "LIST_EMPTY"

  $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
  Write-Host $snapRaw
  $snap = $snapRaw | ConvertFrom-Json
  if ($snap.ok) { throw "snapshot after abort must fail" }
  $code = [string]$snap.error.code
  if ($code -ne "SessionNotFound") { throw "snapshot expected SessionNotFound" }
  Write-Host "ACT_DENIED"

  $gone = $false
  for ($i = 0; $i -lt 20; $i++) {
    $left = @($hudScripts | Where-Object { Test-Path -LiteralPath $_.FullName })
    if ($left.Count -eq 0) { $gone = $true; break }
    Start-Sleep -Milliseconds 100
  }
  if (-not $gone) { throw "WinForms stage script still present after abort" }
  Write-Host "HUD_GONE"

  $start2Raw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $start2Raw
  $start2 = $start2Raw | ConvertFrom-Json
  if (-not $start2.ok) { throw "second session start failed" }
  if ($start2.data.stage_hud -ne $true) { throw "second stage_hud was not true" }
  $sid2 = [string]$start2.data.session_id
  & $vcu --user-dir $ud session abort $sid2 | Out-Null
  Write-Host "STAGE_OK2"

  Write-Host "CU-D-240 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($notepad -and -not $notepad.HasExited) {
    Stop-Process -Id $notepad.Id -Force -ErrorAction SilentlyContinue
  }
}
