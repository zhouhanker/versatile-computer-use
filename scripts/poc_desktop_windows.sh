#!/usr/bin/env bash
# CU-D-060 live UIA: Notepad list / invoke / capture. Honest SKIP off Windows.
set -euo pipefail
os="$(uname -s 2>/dev/null || echo unknown)"
if [[ "$os" != MINGW* && "$os" != MSYS* && "$os" != CYGWIN* && "$os" != *NT* ]]; then
  echo "SKIP CU-D-060 live UIA: host is $os (need Windows). Scripts exist: uia_tree_script / uia_invoke_script / uia_set_value_script / uia_capture_script."
  exit 0
fi
echo "Windows host: live UIA POC is not automated in this repo yet; run vcu app windows then snapshot win:notepad:<pid>."
exit 1
