#!/usr/bin/env bash
# CU-D-060 live UIA. Honest SKIP off Windows; on Windows runs poc_desktop_windows.ps1.
set -euo pipefail
os="$(uname -s 2>/dev/null || echo unknown)"
if [[ "$os" != MINGW* && "$os" != MSYS* && "$os" != CYGWIN* && "$os" != *NT* ]]; then
  echo "SKIP CU-D-060 live UIA: host is $os (need Windows)."
  exit 0
fi
exec powershell.exe -NoProfile -NonInteractive -File "$(dirname "$0")/poc_desktop_windows.ps1"
