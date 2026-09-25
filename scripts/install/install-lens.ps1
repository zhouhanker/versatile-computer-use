# Copy the VCU browser extension for Edge or Chrome load-unpacked.
# Does not start CDP. Does not restart the browser. Does not move the OS cursor.
# Does not click Edge Allow.
#
# From a checkout:
#   powershell -File scripts/install/install-lens.ps1
#   powershell -File scripts/install/install-lens.ps1 -FromRelease -Open
#
# Without a checkout. Release zip first, then the Windows tarball already on GitHub:
#   irm https://raw.githubusercontent.com/zhouhanker/versatile-computer-use/main/scripts/install/install-lens.ps1 | iex
#
# Pipe overrides: VCU_LENS_DEST, VCU_LENS_SOURCE, VCU_LENS_OPEN=1, VCU_LENS_FROM_RELEASE=1, VCU_BASE_URL
param(
  [string]$Source,
  [string]$Dest,
  [switch]$FromRelease,
  [switch]$Open,
  [string]$BaseUrl
)

$ErrorActionPreference = 'Stop'

if (-not $BaseUrl) {
  if ($env:VCU_BASE_URL) { $BaseUrl = $env:VCU_BASE_URL }
  else { $BaseUrl = 'https://github.com/zhouhanker/versatile-computer-use/releases/latest/download' }
}
if (-not $Dest -and $env:VCU_LENS_DEST) { $Dest = $env:VCU_LENS_DEST }
if (-not $Source -and $env:VCU_LENS_SOURCE) { $Source = $env:VCU_LENS_SOURCE }
if ($env:VCU_LENS_OPEN -eq '1') { $Open = $true }
if ($env:VCU_LENS_FROM_RELEASE -eq '1') { $FromRelease = $true }

$RepoRoot = $null
if ($PSScriptRoot) {
  $RepoRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
}
if (-not $PSScriptRoot -and -not $Source) { $FromRelease = $true }

function Find-LocalExtension {
  $candidates = @()
  if ($Source) { $candidates += $Source }
  if ($RepoRoot) { $candidates += (Join-Path $RepoRoot 'extension') }
  $candidates += (Join-Path $env:USERPROFILE '.local\share\vcu\extension')
  foreach ($dir in $candidates) {
    if ($dir -and (Test-Path -LiteralPath (Join-Path $dir 'manifest.json'))) {
      return (Resolve-Path -LiteralPath $dir).Path
    }
  }
  return $null
}

function Save-Url([string]$Url, [string]$OutFile) {
  if ($Url -like 'file://*') {
    $path = $Url -replace '^file:///', '' -replace '^file://', ''
    Copy-Item -LiteralPath $path -Destination $OutFile -Force
    return
  }
  Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing
}

function Test-ZipFile([string]$Path) {
  if (-not (Test-Path -LiteralPath $Path)) { return $false }
  $fs = [System.IO.File]::OpenRead($Path)
  try {
    $buf = New-Object byte[] 2
    $n = $fs.Read($buf, 0, 2)
    return ($n -eq 2 -and $buf[0] -eq 0x50 -and $buf[1] -eq 0x4B)
  } finally {
    $fs.Dispose()
  }
}

function Get-ReleaseExtension {
  $tmp = Join-Path $env:TEMP ('vcu-lens-' + [guid]::NewGuid().ToString())
  New-Item -ItemType Directory -Force -Path $tmp | Out-Null
  $zip = Join-Path $tmp 'vcu-lens-extension.zip'
  $zipUrl = "$BaseUrl/vcu-lens-extension.zip"
  $gotZip = $false
  try {
    Save-Url $zipUrl $zip
    $gotZip = Test-ZipFile $zip
  } catch {
    $gotZip = $false
  }
  if ($gotZip) {
    $unz = Join-Path $tmp 'zip'
    New-Item -ItemType Directory -Force -Path $unz | Out-Null
    Expand-Archive -LiteralPath $zip -DestinationPath $unz -Force
    $manifest = Join-Path $unz 'manifest.json'
    if (-not (Test-Path -LiteralPath $manifest)) { throw "release zip has no manifest.json: $zipUrl" }
    Write-Host 'LENS_FROM zip'
    return $unz
  }
  $tgz = Join-Path $tmp 'vcu.tgz'
  $url = "$BaseUrl/vcu-latest-windows-x64.tar.gz"
  Write-Host 'LENS_FROM tarball'
  Write-Host "Downloading $url"
  Save-Url $url $tgz
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
  if (-not $src) { $src = Get-ReleaseExtension } else { Write-Host 'LENS_FROM local' }
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
