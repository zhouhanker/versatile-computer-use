# Copy the VCU browser extension for Edge or Chrome load-unpacked.
# Does not start CDP. Does not restart the browser. Does not move the OS cursor.
param(
  [string]$Source,
  [string]$Dest,
  [switch]$FromRelease,
  [switch]$Open,
  [string]$BaseUrl = 'https://github.com/zhouhanker/versatile-computer-use/releases/latest/download'
)
$ErrorActionPreference = 'Stop'
$RepoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
function Find-LocalExtension {
  $candidates = @()
  if ($Source) { $candidates += $Source }
  $candidates += @(
    (Join-Path $RepoRoot 'extension'),
    (Join-Path $env:USERPROFILE '.local\share\vcu\extension')
  )
  foreach ($dir in $candidates) {
    if ($dir -and (Test-Path -LiteralPath (Join-Path $dir 'manifest.json'))) {
      return (Resolve-Path -LiteralPath $dir).Path
    }
  }
  return $null
}

function Get-ReleaseExtension {
  $tmp = Join-Path $env:TEMP ('vcu-lens-' + [guid]::NewGuid().ToString())
  New-Item -ItemType Directory -Force -Path $tmp | Out-Null
  $tgz = Join-Path $tmp 'vcu.tgz'
  $url = "$BaseUrl/vcu-latest-windows-x64.tar.gz"
  Write-Host "Downloading $url"
  Invoke-WebRequest -Uri $url -OutFile $tgz
  tar -xzf $tgz -C $tmp
  $hit = Get-ChildItem -Path $tmp -Recurse -Filter manifest.json |
    Where-Object { $_.Directory.Name -eq 'extension' } |
    Select-Object -First 1
  if (-not $hit) { throw 'release archive has no extension/manifest.json' }
  return $hit.Directory.FullName
}

if (-not $Dest) { $Dest = Join-Path $env:USERPROFILE '.vcu\lens-extension' }
if ($FromRelease) {
  $src = Get-ReleaseExtension
} else {
  $src = Find-LocalExtension
  if (-not $src) { $src = Get-ReleaseExtension }
}
if (Test-Path -LiteralPath $Dest) { Remove-Item -LiteralPath $Dest -Recurse -Force }
New-Item -ItemType Directory -Force -Path $Dest | Out-Null
Copy-Item -Path (Join-Path $src '*') -Destination $Dest -Recurse -Force
$copied = Join-Path $Dest 'manifest.json'
if (-not (Test-Path -LiteralPath $copied)) { throw "copy failed: $copied" }
Write-Host "LENS_OK $Dest"
Write-Host 'Edge: open edge://extensions, enable Developer mode, Load unpacked, select this folder.'
Write-Host 'Do not click the debugging consent dialog. Do not use CDP.'
if ($Open) {
  $explorer = Join-Path $env:SystemRoot 'explorer.exe'
  & $explorer $Dest
}
