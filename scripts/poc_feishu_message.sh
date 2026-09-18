#!/usr/bin/env bash
# Honest non-send. Product send is FEISHU-001: Feishu app + host vision + VCU.
# osascript ok and lark-cli IM are false greens (user confirmed 张北北 did not receive).
set -euo pipefail
echo "FEISHU_POC_REFUSED"
echo "reason=lark-cli/osascript is not VCU computer-use"
echo "gate=make poc-feishu-scene  # AX has no 发送; never sends"
echo "product=docs/PLAN.md P2 FEISHU-001 vision click Send + chat screenshot"
echo "wechat_touched=false"
echo "codex_cu_touched=false"
echo "sent=false"
exit 2
