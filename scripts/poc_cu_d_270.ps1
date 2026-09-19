# CU-D-270: vcu doctor is honest about Windows CI slices vs product CU.
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT") {
  Write-Host "SKIP CU-D-270: not Windows_NT"
  exit 0
}

$root = (Get-Location).Path
$bin = Join-Path $root "target\debug"
$vcu = Join-Path $bin "vcu.exe"
if (-not (Test-Path $vcu)) {
  throw "missing vcu.exe under target/debug; build vcu-cli first"
}
$ud = Join-Path $env:TEMP ("vcu-270-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $ud | Out-Null
& $vcu --user-dir $ud init --json | Out-Null

$raw = & $vcu --user-dir $ud doctor --json
Write-Host $raw
$doc = $raw | ConvertFrom-Json
$checks = @($doc.data.checks)
if ($checks.Count -lt 1) { throw "doctor checks empty" }

$scope = $null
$backend = $null
$stage = $null
foreach ($c in $checks) {
  if ([string]$c.name -eq "windows_desktop_scope") { $scope = $c }
  if ([string]$c.name -eq "app_backend") { $backend = $c }
  if ([string]$c.name -eq "stage_helper") { $stage = $c }
}
if ($null -eq $scope) { throw "windows_desktop_scope missing" }
if ([string]$scope.status -ne "warn") { throw "windows_desktop_scope expected warn" }
$sdet = [string]$scope.detail
if ($sdet -notlike "*Not a product Windows*") { throw "scope must say not a product Windows session" }
if ($sdet -like "*AXPress*") { throw "scope must not claim AXPress" }
Write-Host "SCOPE_OK"

if ($null -eq $backend) { throw "app_backend missing" }
$bdet = [string]$backend.detail
$bhint = [string]$backend.hint
if ($bdet -notlike "*wm_settext*") { throw "app_backend missing wm_settext" }
if ($bdet -notlike "*bm_click*") { throw "app_backend missing bm_click" }
if ($bdet -like "*AXPress*") { throw "app_backend must not claim AXPress" }
if ($bhint -notlike "*Not product Windows CU*") { throw "app_backend hint must deny product CU" }
Write-Host "BACKEND_OK"

if ($null -eq $stage) { throw "stage_helper missing" }
$tdet = [string]$stage.detail
if ($tdet -notlike "*WinForms*") { throw "stage_helper must name WinForms HUD" }
if ($tdet -like "*omit the helper*") { throw "stage_helper must not omit WinForms" }
Write-Host "STAGE_OK"

Write-Host "CU-D-270 OK"
