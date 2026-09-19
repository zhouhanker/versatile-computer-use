# CU-D-170: Windows Settings is observe-only (no click/type, no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-170: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-170-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

& $vcu --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

Start-Process "ms-settings:" | Out-Null
Start-Sleep -Seconds 2

function Get-VcuSettingsProcess {
  $named = Get-Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ProcessName -match '^(SystemSettings|SystemSettingsAdminFlows)$'
  } | Select-Object -First 1
  if ($named) { return $named }
  return Get-Process -ErrorAction SilentlyContinue | Where-Object {
    $_.MainWindowTitle -match 'Settings'
  } | Select-Object -First 1
}

$settings = $null
for ($i = 0; $i -lt 20; $i++) {
  $settings = Get-VcuSettingsProcess
  if ($settings) { break }
  Start-Sleep -Milliseconds 250
}
if (-not $settings) { throw "settings process missing" }

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

  $tab = "win:SystemSettings:$($settings.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK session={0} tab={1}" -f $sid, $tab)

  $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
  Write-Host $snapRaw
  $snap = $snapRaw | ConvertFrom-Json
  if (-not $snap.ok) { throw "snapshot failed" }
  $refs = @($snap.data.dom_refs)
  if ($refs.Count -lt 1) { throw "empty settings scene" }
  $ref = [string]$refs[0].ref
  Write-Host ("SNAP_OK ref={0} count={1}" -f $ref, $refs.Count)

  $clickRaw = & $vcu --user-dir $ud click --session $sid --tab $tab --ref $ref --json
  Write-Host $clickRaw
  $clicked = $clickRaw | ConvertFrom-Json
  if ($clicked.ok) { throw "settings click must be refused" }
  $code = [string]$clicked.error.code
  if ($code -ne "FocusPolicyViolation") { throw "settings click expected FocusPolicyViolation" }
  Write-Host "CLICK_DENIED"

  $typeRaw = & $vcu --user-dir $ud type --session $sid --tab $tab --ref $ref --text "do-not-type" --json
  Write-Host $typeRaw
  $typed = $typeRaw | ConvertFrom-Json
  if ($typed.ok) { throw "settings type must be refused" }
  $tcode = [string]$typed.error.code
  if ($tcode -ne "FocusPolicyViolation") { throw "settings type expected FocusPolicyViolation" }
  Write-Host "TYPE_DENIED"

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-170 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  Get-Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ProcessName -match '^(SystemSettings|SystemSettingsAdminFlows)$'
  } | Stop-Process -Force -ErrorAction SilentlyContinue
}
