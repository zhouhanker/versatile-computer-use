# CU-D-160: vcu clicks throwaway Calculator without SendInput.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-160: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-160-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

& $vcu --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

$calcPath = Join-Path $env:SystemRoot "System32\calc.exe"
if (-not (Test-Path $calcPath)) { $calcPath = "calc.exe" }
$started = Start-Process -FilePath $calcPath -PassThru -WindowStyle Normal
Start-Sleep -Seconds 2

function Get-VcuCalculatorProcess {
  $named = Get-Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ProcessName -match '^(calc|Calculator|CalculatorApp)$'
  } | Select-Object -First 1
  if ($named) { return $named }
  return Get-Process -ErrorAction SilentlyContinue | Where-Object {
    $_.MainWindowTitle -match 'Calculator'
  } | Select-Object -First 1
}

$calc = $null
for ($i = 0; $i -lt 20; $i++) {
  $calc = Get-VcuCalculatorProcess
  if ($calc) { break }
  Start-Sleep -Milliseconds 250
}
if (-not $calc) { throw "calculator process missing" }

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

  $tab = "win:calculator:$($calc.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK session={0} tab={1}" -f $sid, $tab)

  $ref = $null
  for ($i = 0; $i -lt 15; $i++) {
    $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
    Write-Host $snapRaw
    $snap = $snapRaw | ConvertFrom-Json
    if (-not $snap.ok) { throw "snapshot failed" }
    foreach ($r in @($snap.data.dom_refs)) {
      $nm = [string]$r.name
      $role = [string]$r.role
      # UWP: One/num1Button. Win32 calc (Server): LegacyIAccessible 131 = digit 1.
      if ($nm -eq "One" -or $nm -eq "1" -or $nm -eq "num1Button" -or $nm -like "*num1Button*" -or $nm -eq "131") {
        $ref = [string]$r.ref
        break
      }
      if (-not $ref -and $role -like "*Button*" -and $nm -ne "" -and $nm -ne "Calculator") {
        $ref = [string]$r.ref
        break
      }
    }
    if ($ref) { break }
    Start-Sleep -Milliseconds 300
  }
  if (-not $ref) { throw "empty calculator scene" }
  Write-Host ("SNAP_OK ref={0}" -f $ref)

  $clickRaw = & $vcu --user-dir $ud click --session $sid --tab $tab --ref $ref --json
  Write-Host $clickRaw
  $clicked = $clickRaw | ConvertFrom-Json
  if (-not $clicked.ok) { throw "click failed" }
  $path = [string]$clicked.data.detail.input_path
  $cursor = [bool]$clicked.data.detail.os_cursor_used
  if ($cursor) { throw "os_cursor_used true" }
  if ($path -ne "uia_invoke" -and $path -ne "legacy_invoke" -and $path -ne "bm_click") {
    throw "unexpected input_path"
  }
  Write-Host ("INVOKE_OK path={0} os_cursor_used={1}" -f $path, $cursor)

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-160 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($calc -and -not $calc.HasExited) {
    Stop-Process -Id $calc.Id -Force -ErrorAction SilentlyContinue
  }
  if ($started -and -not $started.HasExited) {
    Stop-Process -Id $started.Id -Force -ErrorAction SilentlyContinue
  }
  Get-Process -ErrorAction SilentlyContinue | Where-Object {
    $_.ProcessName -match '^(calc|Calculator|CalculatorApp)$'
  } | Stop-Process -Force -ErrorAction SilentlyContinue
}
