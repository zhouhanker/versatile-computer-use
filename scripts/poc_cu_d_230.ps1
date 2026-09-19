# CU-D-230: vcu types into throwaway powershell.exe without newline/Return (no SendInput).
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-230: not Windows_NT"
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
$ud = Join-Path $env:TEMP ("vcu-230-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null

& $vcu --user-dir $ud init --json | Out-Null
$cfgPath = Join-Path $ud "config.json"
$cfg = Get-Content $cfgPath | ConvertFrom-Json
$cfg.daemon_port = 19000 + (Get-Random -Maximum 1000)
$cfg | ConvertTo-Json | Set-Content $cfgPath

$psExe = Join-Path $env:SystemRoot "System32\WindowsPowerShell\v1.0\powershell.exe"
$conhost = Join-Path $env:SystemRoot "System32\conhost.exe"
$hostProc = $null
if (Test-Path $conhost) {
  $hostProc = Start-Process -FilePath $conhost -ArgumentList @($psExe, "-NoExit", "-Command", "[console]::Title='VCU-D-230'") -WindowStyle Normal -PassThru
  Start-Sleep -Seconds 1
  $child = Get-CimInstance Win32_Process -Filter ("ParentProcessId={0}" -f $hostProc.Id) -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -eq "powershell.exe" } |
    Select-Object -First 1
  if ($child) {
    $cmd = Get-Process -Id ([int]$child.ProcessId)
  } else {
    $cmd = $hostProc
  }
} else {
  $cmd = Start-Process -FilePath $psExe -ArgumentList @("-NoExit", "-Command", "[console]::Title='VCU-D-230'") -WindowStyle Normal -PassThru
  Start-Sleep -Seconds 1
}
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

  $procName = [string]$cmd.ProcessName
  if ($procName -ne "powershell" -and $procName -ne "conhost") { $procName = "powershell" }
  $tab = "win:${procName}:$($cmd.Id)"
  $startRaw = & $vcu --user-dir $ud session start --surface desktop --app-id $tab --json
  Write-Host $startRaw
  $start = $startRaw | ConvertFrom-Json
  if (-not $start.ok) { throw "session start failed" }
  $sid = [string]$start.data.session_id
  Write-Host ("STAGE_OK session={0}" -f $sid)

  $snap = $null
  $ref = $null
  for ($i = 0; $i -lt 15; $i++) {
    $snapRaw = & $vcu --user-dir $ud snapshot --session $sid --tab $tab --json
    Write-Host $snapRaw
    $snap = $snapRaw | ConvertFrom-Json
    if (-not $snap.ok) { throw "snapshot failed" }
    $ref = $null
    foreach ($r in @($snap.data.dom_refs)) {
      $role = [string]$r.role
      if ($role -like "*Edit*") { $ref = [string]$r.ref; break }
    }
    if (-not $ref) {
      $first = @($snap.data.dom_refs)[0]
      if ($null -ne $first) { $ref = [string]$first.ref }
    }
    if ($ref) { break }
    Start-Sleep -Milliseconds 300
  }
  if (-not $ref) { throw "empty powershell scene" }
  Write-Host ("SNAP_OK ref={0}" -f $ref)

  $typeRaw = & $vcu --user-dir $ud type --session $sid --tab $tab --ref $ref --text "echo-not-run" --json
  Write-Host $typeRaw
  $typed = $typeRaw | ConvertFrom-Json
  if (-not $typed.ok) { throw "type failed" }
  $path = [string]$typed.data.detail.input_path
  $cursor = [bool]$typed.data.detail.os_cursor_used
  if ($cursor) { throw "os_cursor_used true" }
  if ($path -ne "clipboard_paste" -and $path -ne "wm_settext" -and $path -ne "uia_set_value") {
    throw "unexpected input_path"
  }
  Write-Host ("TYPE_OK path={0} os_cursor_used={1}" -f $path, $cursor)

  $nlRaw = & $vcu --user-dir $ud type --session $sid --tab $tab --ref $ref --text "echo-not-run`r`n" --json
  Write-Host $nlRaw
  $nl = $nlRaw | ConvertFrom-Json
  if ($nl.ok) { throw "newline type must be refused" }
  $code = [string]$nl.error.code
  if ($code -ne "FocusPolicyViolation") { throw "newline type expected FocusPolicyViolation" }
  Write-Host "NEWLINE_DENIED"

  & $vcu --user-dir $ud session abort $sid | Out-Null
  Write-Host "CU-D-230 OK"
} finally {
  if ($daemon -and -not $daemon.HasExited) {
    Stop-Process -Id $daemon.Id -Force -ErrorAction SilentlyContinue
  }
  if ($cmd -and -not $cmd.HasExited) {
    Stop-Process -Id $cmd.Id -Force -ErrorAction SilentlyContinue
  }
  if ($hostProc -and -not $hostProc.HasExited) {
    Stop-Process -Id $hostProc.Id -Force -ErrorAction SilentlyContinue
  }
}
