# VCU Windows installer
#   irm <url>/install.ps1 | iex
#   $env:VCU_BASE_URL = 'file:///C:/path/to/dist'; irm ... | iex
param()
$ErrorActionPreference = 'Stop'
$Version = if ($env:VCU_VERSION) { $env:VCU_VERSION } else { 'latest' }
$Prefix = if ($env:VCU_PREFIX) { $env:VCU_PREFIX } else { Join-Path $env:USERPROFILE '.local' }
$BinDir = if ($env:VCU_BIN_DIR) { $env:VCU_BIN_DIR } else { Join-Path $Prefix 'bin' }
$ShareDir = if ($env:VCU_SHARE_DIR) { $env:VCU_SHARE_DIR } else { Join-Path $Prefix 'share\vcu' }
$BaseUrl = if ($env:VCU_BASE_URL) { $env:VCU_BASE_URL } else { 'https://github.com/zhouhanker/versatile-computer-use/releases/latest/download' }
$Arch = if ([Environment]::Is64BitOperatingSystem) {
  if ($env:PROCESSOR_ARCHITECTURE -match 'ARM') { 'arm64' } else { 'x64' }
} else { throw '32-bit Windows is not supported' }
$Plat = "windows-$Arch"

function Get-ArchiveUrl {
  if ($env:VCU_ARCHIVE_URL) { return $env:VCU_ARCHIVE_URL }
  if ($BaseUrl -like 'file://*') {
    $base = $BaseUrl -replace '^file:///', '' -replace '^file://', ''
    if ($Version -eq 'latest') { return "file:///$base/vcu-latest-$Plat.tar.gz" }
    return "file:///$base/vcu-$Version-$Plat.tar.gz"
  }
  if ($Version -eq 'latest') { return "$BaseUrl/vcu-latest-$Plat.tar.gz" }
  $root = $BaseUrl -replace '/latest/download$','' -replace '/download$',''
  return "$root/download/v$Version/vcu-$Version-$Plat.tar.gz"
}

$tmp = Join-Path $env:TEMP ("vcu-install-" + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
try {
  $url = Get-ArchiveUrl
  Write-Host "Downloading $url"
  $tgz = Join-Path $tmp 'vcu.tgz'
  if ($url -like 'file://*') {
    $path = $url -replace '^file:///', '' -replace '^file://', ''
    Copy-Item $path $tgz
  } else {
    Invoke-WebRequest -Uri $url -OutFile $tgz
  }
  # tar is available on modern Windows 10+
  tar -xzf $tgz -C $tmp
  $stage = Get-ChildItem $tmp -Directory -Filter 'vcu-*' | Select-Object -First 1
  if (-not $stage) { throw 'archive layout missing vcu-* directory' }
  New-Item -ItemType Directory -Force -Path $BinDir, $ShareDir | Out-Null
  Copy-Item (Join-Path $stage.FullName 'bin\vcu.exe') (Join-Path $BinDir 'vcu.exe') -Force
  Copy-Item (Join-Path $stage.FullName 'bin\vcu-daemon.exe') (Join-Path $BinDir 'vcu-daemon.exe') -Force
  Copy-Item (Join-Path $stage.FullName 'bin\vcu-mcp.exe') (Join-Path $BinDir 'vcu-mcp.exe') -Force
  # vcu-stage is macOS-only (AppKit overlay). Windows packages omit it.
  $ext = Join-Path $stage.FullName 'extension'
  if (Test-Path $ext) {
    $destExt = Join-Path $ShareDir 'extension'
    if (Test-Path $destExt) { Remove-Item $destExt -Recurse -Force }
    Copy-Item $ext $destExt -Recurse -Force
  }
  $vcu = Join-Path $BinDir 'vcu.exe'
  & $vcu --version
  & $vcu init --json
  Write-Host "OK: installed to $BinDir"
  Write-Host "Ensure PATH contains $BinDir"
  if (Test-Path $destExt) {
    Write-Host "Extension folder: $destExt"
    Write-Host "Edge: edge://extensions, Developer mode, Load unpacked, select that folder."
    Write-Host "Do not click the debugging consent dialog."
  }
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
