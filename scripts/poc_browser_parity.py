#!/usr/bin/env python3
"""Controlled USER-browser parity POC.

The default invocation is intentionally read-only: it prints this help and
refuses to create a browser tab.  Real actions require ``--live`` and use only
the local layout-state fixture served by this script.  The script records
created tab IDs and closes exactly those IDs during cleanup.
"""

from __future__ import annotations

import argparse
import json
import sys
import threading
import time
import uuid
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any, Callable
from urllib.parse import urlencode
import subprocess


REPO_ROOT = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = REPO_ROOT / "extension"
FIXTURE_PATH = "/tests/fixtures/layout-state.html"


class PocAbort(RuntimeError):
    """A controlled POC assertion failed."""


class CommandResult:
    def __init__(self, args: list[str], returncode: int, stdout: str, stderr: str, parsed: Any):
        self.args = args
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr
        self.parsed = parsed


class VcuRunner:
    def __init__(self, vcu_path: str):
        self.vcu_path = vcu_path

    def run(self, browser_args: list[str], timeout: float = 20.0) -> CommandResult:
        # Keep the RTK proxy in the child command as required by the project
        # shell policy. A list, rather than shell text, keeps fixture/query
        # arguments literal and avoids command interpolation.
        command = ["rtk", "proxy", self.vcu_path, "browser", *browser_args, "--json"]
        try:
            completed = subprocess.run(
                command,
                cwd=REPO_ROOT,
                capture_output=True,
                text=True,
                timeout=timeout,
                check=False,
            )
            stdout = completed.stdout or ""
            stderr = completed.stderr or ""
            parsed = parse_json(stdout)
            if parsed is None:
                parsed = parse_json(stderr)
            return CommandResult(command, completed.returncode, stdout, stderr, parsed)
        except subprocess.TimeoutExpired as exc:
            stdout = exc.stdout.decode(errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or "")
            stderr = exc.stderr.decode(errors="replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")
            return CommandResult(command, 124, stdout, stderr, parse_json(stdout) or parse_json(stderr))
        except OSError as exc:
            return CommandResult(command, 127, "", str(exc), None)


def parse_json(text: str) -> Any:
    text = (text or "").strip()
    if not text:
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        decoder = json.JSONDecoder()
        for index, char in enumerate(text):
            if char not in "[{":
                continue
            try:
                value, _ = decoder.raw_decode(text[index:])
                return value
            except json.JSONDecodeError:
                continue
    return None


def payload(value: Any) -> dict[str, Any]:
    if not isinstance(value, dict):
        return {}
    data = value.get("data")
    return data if isinstance(data, dict) else value


def top_ok(result: CommandResult) -> bool:
    return isinstance(result.parsed, dict) and result.parsed.get("ok") is True


def error_code(result: CommandResult) -> str:
    value = result.parsed if isinstance(result.parsed, dict) else {}
    error = value.get("error")
    if isinstance(error, dict):
        return str(error.get("code") or error.get("type") or "error")
    if isinstance(error, str):
        return error[:120]
    return "command_failed"


def group_summary(group: Any) -> dict[str, Any] | None:
    if not isinstance(group, dict):
        return None
    return {
        "group_id": str(group.get("group_id")) if group.get("group_id") is not None else None,
        "window_id": str(group.get("window_id")) if group.get("window_id") is not None else None,
        "title": str(group.get("title") or ""),
        "color": str(group.get("color") or ""),
        "collapsed": bool(group.get("collapsed")),
    }


def tab_summary(tab: Any) -> dict[str, Any] | None:
    if not isinstance(tab, dict):
        return None
    group = group_summary(tab.get("group"))
    return {
        "tab_id": str(tab.get("tab_id")) if tab.get("tab_id") is not None else None,
        "window_id": str(tab.get("window_id")) if tab.get("window_id") is not None else None,
        "active": bool(tab.get("active")),
        "focused": bool(tab.get("focused")),
        "group_id": str(tab.get("group_id")) if tab.get("group_id") is not None else None,
        "group": group,
    }


def tabs_from(result: CommandResult) -> list[dict[str, Any]]:
    raw = payload(result.parsed).get("tabs")
    if not isinstance(raw, list):
        return []
    summaries: list[dict[str, Any]] = []
    for tab in raw:
        summary = tab_summary(tab)
        if summary:
            summaries.append(summary)
    return summaries


def group_from(result: CommandResult, group_id: str) -> dict[str, Any] | None:
    data = payload(result.parsed)
    direct = group_summary(data.get("group"))
    if direct and direct.get("group_id") == str(group_id):
        return direct
    groups = data.get("groups")
    if isinstance(groups, list):
        for group in groups:
            summary = group_summary(group)
            if summary and summary.get("group_id") == str(group_id):
                return summary
    return None


def capture_summary(data: dict[str, Any]) -> dict[str, Any]:
    viewport = data.get("viewport") if isinstance(data.get("viewport"), dict) else {}
    signature = viewport.get("layout_signature") if isinstance(viewport.get("layout_signature"), dict) else {}
    return {
        "capture_id": str(data.get("capture_id") or ""),
        "tab_id": str(data.get("tab_id") or ""),
        "screenshot_width": data.get("screenshot_width"),
        "screenshot_height": data.get("screenshot_height"),
        "document_id": viewport.get("document_id"),
        "viewport_width": viewport.get("width"),
        "viewport_height": viewport.get("height"),
        "revision": viewport.get("revision"),
        "layout_signature": {
            "max_nodes": signature.get("max_nodes"),
            "scan_max_nodes": signature.get("scan_max_nodes"),
            "visible_count": signature.get("visible_count"),
            "scanned_count": signature.get("scanned_count"),
            "overflow": signature.get("overflow"),
            "overflow_reason": signature.get("overflow_reason"),
        },
    }


class QuietHandler(SimpleHTTPRequestHandler):
    """Serve only the checked-in extension directory without log noise."""

    def log_message(self, _format: str, *_args: Any) -> None:
        return


class Poc:
    def __init__(self, args: argparse.Namespace):
        self.args = args
        self.runner = VcuRunner(args.vcu)
        self.nonce = uuid.uuid4().hex[:10]
        self.server: ThreadingHTTPServer | None = None
        self.server_thread: threading.Thread | None = None
        self.base_url = ""
        self.fixture_urls: dict[str, str] = {}
        self.created_tabs: list[dict[str, str]] = []
        self.baseline_focused_tab: str | None = None
        self.steps: list[dict[str, Any]] = []
        self.cleanup: list[dict[str, Any]] = []
        self.report: dict[str, Any] = {
            "ok": False,
            "mode": "live",
            "nonce": self.nonce,
            "fixture": "extension/tests/fixtures/layout-state.html",
            "nodes": args.nodes,
            "steps": self.steps,
            "cleanup": self.cleanup,
        }

    def start_server(self) -> None:
        if not FIXTURE_ROOT.is_dir():
            raise PocAbort(f"fixture root missing: {FIXTURE_ROOT}")

        def handler(*handler_args: Any, **handler_kwargs: Any) -> QuietHandler:
            return QuietHandler(*handler_args, directory=str(FIXTURE_ROOT), **handler_kwargs)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), handler)
        self.server.daemon_threads = True
        port = self.server.server_address[1]
        self.base_url = f"http://127.0.0.1:{port}"
        self.fixture_urls = {
            role: f"{self.base_url}{FIXTURE_PATH}?{urlencode({'nodes': self.args.nodes, 'nonce': self.nonce, 'window': role})}"
            for role in ("A", "B", "A2")
        }
        self.server_thread = threading.Thread(target=self.server.serve_forever, name="vcu-poc-http", daemon=True)
        self.server_thread.start()

    def stop_server(self) -> None:
        if self.server is None:
            return
        self.server.shutdown()
        self.server.server_close()
        if self.server_thread is not None:
            self.server_thread.join(timeout=2)
        self.server = None

    def command_label(self, browser_args: list[str]) -> list[str]:
        return ["rtk", "proxy", self.args.vcu, "browser", *browser_args, "--json"]

    def record(self, name: str, browser_args: list[str], started: float, passed: bool, detail: dict[str, Any]) -> None:
        self.steps.append({
            "name": name,
            "pass": bool(passed),
            "latency_ms": round((time.monotonic() - started) * 1000, 2),
            "command": self.command_label(browser_args),
            "detail": detail,
        })

    def assertion(self, name: str, browser_args: list[str], passed: bool, detail: dict[str, Any]) -> None:
        """Record a derived check against the preceding bounded command."""
        self.steps.append({
            "name": name,
            "pass": bool(passed),
            "latency_ms": 0.0,
            "command": self.command_label(browser_args),
            "detail": {"derived_from_previous_command": True, **detail},
        })
        if not passed:
            raise PocAbort(f"{name} failed")

    def run_step(
        self,
        name: str,
        browser_args: list[str],
        validator: Callable[[CommandResult], tuple[bool, dict[str, Any]]],
        timeout: float = 20.0,
    ) -> CommandResult:
        started = time.monotonic()
        result = self.runner.run(browser_args, timeout=timeout)
        try:
            passed, detail = validator(result)
        except Exception as exc:  # Validators are part of the POC boundary.
            passed, detail = False, {"reason": "validator_exception", "type": type(exc).__name__}
        detail = dict(detail)
        detail.setdefault("returncode", result.returncode)
        if not passed:
            detail.setdefault("error_code", error_code(result))
        self.record(name, browser_args, started, passed, detail)
        if not passed:
            raise PocAbort(f"{name} failed")
        return result

    @staticmethod
    def expect_ok(result: CommandResult) -> tuple[bool, dict[str, Any]]:
        if top_ok(result):
            return True, {"response": "ok"}
        return False, {"response": "not_ok"}

    @staticmethod
    def expect_error(predicate: Callable[[str], bool], label: str) -> Callable[[CommandResult], tuple[bool, dict[str, Any]]]:
        def validate(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            text = json.dumps(result.parsed, ensure_ascii=False) if result.parsed is not None else ""
            passed = isinstance(result.parsed, dict) and result.parsed.get("ok") is False and predicate(text.lower())
            return passed, {"expected_error": label, "response": "rejected" if passed else "unexpected"}
        return validate

    def list_tabs(self, name: str, own_only: bool = True) -> tuple[CommandResult, list[dict[str, Any]]]:
        holder: dict[str, Any] = {}

        def validate(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            tabs = tabs_from(result)
            holder["tabs"] = tabs
            own = [tab for tab in tabs if tab.get("tab_id") in {item["tab_id"] for item in self.created_tabs}]
            return True, {"own_tabs": own if own_only else tabs}

        result = self.run_step(name, ["tabs"], validate)
        return result, holder.get("tabs", [])

    def tab_by_id(self, tabs: list[dict[str, Any]], tab_id: str) -> dict[str, Any] | None:
        return next((tab for tab in tabs if tab.get("tab_id") == str(tab_id)), None)

    def check_baseline_focus(self, name: str) -> None:
        _, tabs = self.list_tabs(name)
        focused = [tab.get("tab_id") for tab in tabs if tab.get("focused")]
        if self.baseline_focused_tab not in focused:
            # list_tabs only reports own tabs in its detail, so query again in
            # this small local check without writing unrelated tab metadata.
            all_result = self.runner.run(["tabs"], timeout=15)
            all_tabs = tabs_from(all_result)
            baseline_focused = any(tab.get("tab_id") == self.baseline_focused_tab and tab.get("focused") for tab in all_tabs)
            if baseline_focused:
                return
            raise PocAbort(f"{name} baseline focus changed")

    def parse_open_into(self, holder: dict[str, Any], role: str, result: CommandResult) -> tuple[bool, dict[str, Any]]:
        if not top_ok(result):
            return False, {"response": "not_ok"}
        tabs = payload(result.parsed).get("tabs")
        if not isinstance(tabs, list) or len(tabs) != 1:
            return False, {"response": "unexpected_tab_count"}
        tab = tab_summary(tabs[0])
        if not tab or not tab.get("tab_id") or not tab.get("window_id"):
            return False, {"response": "tab_identity_missing"}
        # Register the exact tab as soon as the successful open response
        # reveals its identity. Later validation (background focus/group) may
        # fail, but finally cleanup must still know this tab.
        if not any(item["tab_id"] == tab["tab_id"] for item in self.created_tabs):
            self.created_tabs.append({"tab_id": tab["tab_id"], "window_id": tab["window_id"], "role": role})
        group = group_summary(payload(result.parsed).get("group")) or tab.get("group")
        if not group or not group.get("group_id"):
            return False, {"response": "group_identity_missing"}
        if tab.get("focused"):
            return False, {"response": "background_open_focused"}
        holder.update({"tab": tab, "group": group, "role": role})
        return True, {"tab": tab, "group": group}

    def open_poc_tab(self, role: str, group_title: str) -> dict[str, Any]:
        holder: dict[str, Any] = {}
        if role in ("A", "B"):
            args = [
                "open", "--new-window", "--background", "--session-name", group_title,
                "--url", self.fixture_urls[role],
            ]
            validator = lambda result: self.parse_open_into(holder, role, result)
        else:
            args = ["open", "--group", group_title, "--background", "--url", self.fixture_urls[role]]
            validator = lambda result: self.parse_open_into(holder, role, result)
        self.run_step(f"open_{role}", args, validator)
        tab = holder["tab"]
        group = holder["group"]
        return {"tab": tab, "group": group}

    def wait_fixture(self, role: str, tab_id: str) -> None:
        started = time.monotonic()
        deadline = started + 5.0
        attempts = 0
        last: CommandResult | None = None
        while time.monotonic() < deadline:
            attempts += 1
            remaining = max(0.05, deadline - time.monotonic())
            last = self.runner.run(["extract", "--selector", "#moving", "--tab", tab_id], timeout=remaining)
            if top_ok(last):
                matches = payload(last.parsed).get("matches")
                if isinstance(matches, list) and matches:
                    self.record(
                        f"fixture_ready_{role}",
                        ["extract", "--selector", "#moving", "--tab", tab_id],
                        started,
                        True,
                        {"attempts": attempts, "source": payload(last.parsed).get("source", "")},
                    )
                    return
            if time.monotonic() >= deadline:
                break
            time.sleep(min(0.25, max(0.0, deadline - time.monotonic())))
        detail = {"attempts": attempts, "response": "fixture_not_ready"}
        if last is not None:
            detail["error_code"] = error_code(last)
        self.record(
            f"fixture_ready_{role}",
            ["extract", "--selector", "#moving", "--tab", tab_id],
            started,
            False,
            detail,
        )
        raise PocAbort(f"fixture {role} did not become ready within 5 seconds")

    def capture(self, role: str, tab_id: str, label: str) -> dict[str, Any]:
        holder: dict[str, Any] = {}

        def validate(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            data = payload(result.parsed)
            summary = capture_summary(data)
            viewport = data.get("viewport") if isinstance(data.get("viewport"), dict) else {}
            signature = viewport.get("layout_signature") if isinstance(viewport.get("layout_signature"), dict) else {}
            capture_id = summary["capture_id"]
            if not capture_id or summary["tab_id"] != str(tab_id):
                return False, {"response": "capture_identity_missing"}
            if not isinstance(summary["screenshot_width"], (int, float)) or not isinstance(summary["screenshot_height"], (int, float)):
                return False, {"response": "screenshot_dimensions_missing"}
            if signature.get("overflow") is not False:
                return False, {"response": "layout_signature_overflow", "layout_signature": signature}
            if signature.get("visible_count", 0) < self.args.nodes:
                return False, {"response": "layout_signature_count_too_low", "layout_signature": signature}
            holder.update({"data": data, "summary": summary, "viewport": viewport})
            return True, {"capture": summary}

        self.run_step(f"{label}_screenshot", ["screenshot", "--tab", tab_id], validate)
        return holder["data"]

    def stable_pixel_dry_run(self, tab_id: str, capture: dict[str, Any]) -> None:
        # This is a no-side-effect protocol check, not a substitute for a
        # host viewing the PNG before a real pixel click.
        target = self.run_step("stable_target_geometry", ["click", "--selector", "#moving", "--tab", tab_id, "--dry-run"], self.expect_ok)
        point = payload(target.parsed).get("extension", {}).get("css_point", {})
        viewport = capture["viewport"]
        px = float(point["x"]) * float(capture["screenshot_width"]) / float(viewport["width"])
        py = float(point["y"]) * float(capture["screenshot_height"]) / float(viewport["height"])
        def valid(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            data = payload(result.parsed)
            passed = top_ok(result) and data.get("pressed") is False and data.get("input_path") == "dom_point_click"
            return passed, {"response": "stable_capture_accepted" if passed else "stable_capture_rejected", "pressed":data.get("pressed")}
        self.run_step("stable_capture_allows_point_dry_run", ["click", "--space", "viewport", "--capture", str(capture["capture_id"]), "--tab", tab_id, "--pixel-x", str(px), "--pixel-y", str(py), "--dry-run"], valid)

    def selector_click(self, tab_id: str, selector: str = "#move-css", name: str = "selector_click_move_css") -> None:
        def validate(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            data = payload(result.parsed)
            return data.get("pressed") is True and data.get("source") == "extension_dom", {
                "pressed": data.get("pressed"),
                "source": data.get("source"),
            }

        self.run_step(
            name,
            ["click", "--selector", selector, "--tab", tab_id],
            validate,
        )

    def stale_pixel_dry_run(self, tab_id: str, capture_id: str, label: str) -> None:
        validator = self.expect_error(lambda text: "stale_viewport" in text or "stale viewport" in text, "stale_viewport")
        self.run_step(
            label,
            [
                "click", "--space", "viewport", "--capture", capture_id,
                "--tab", tab_id, "--pixel-x", "1", "--pixel-y", "1", "--dry-run",
            ],
            validator,
        )

    def type_local_input(self, tab_id: str) -> str:
        text = f"VCU_POC_INPUT_{self.nonce}"

        def validate(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            data = payload(result.parsed)
            return data.get("typed") is True and data.get("source") == "extension_dom", {
                "typed": data.get("typed"),
                "source": data.get("source"),
            }

        self.run_step(
            "type_plain_input",
            ["type", "--selector", "#plain-input", "--tab", tab_id, "--text", text],
            validate,
        )
        return text

    def group_unchanged(self, tabs: list[dict[str, Any]], expected: dict[str, dict[str, Any]]) -> tuple[bool, dict[str, Any]]:
        changed: list[str] = []
        for tab_id, expected_group in expected.items():
            tab = self.tab_by_id(tabs, tab_id)
            if not tab or tab.get("group_id") != expected_group.get("group_id") or tab.get("group") != expected_group:
                changed.append(tab_id)
        return not changed, {"unchanged": not changed, "changed_tab_ids": changed}

    def run(self) -> None:
        self.start_server()
        self.report["fixture_url"] = self.fixture_urls["A"]

        _, baseline_tabs = self.list_tabs("baseline_tabs")
        focused = [tab.get("tab_id") for tab in baseline_tabs if tab.get("focused")]
        # list_tabs() intentionally only places own tabs in its step detail,
        # but before any creation all tabs are non-own, so inspect the command
        # once more without retaining its URLs.
        if not focused:
            baseline_result = self.runner.run(["tabs"], timeout=15)
            baseline_all = tabs_from(baseline_result)
            focused = [tab.get("tab_id") for tab in baseline_all if tab.get("focused")]
        if len(focused) != 1:
            raise PocAbort("could not identify exactly one baseline focused tab")
        self.baseline_focused_tab = focused[0]
        self.steps[-1]["detail"]["baseline_focused_tab"] = self.baseline_focused_tab

        opened_a = self.open_poc_tab("A", f"VCU POC {self.nonce} A")
        tab_a = opened_a["tab"]["tab_id"]
        group_a = opened_a["group"]["group_id"]
        self.wait_fixture("A", tab_a)
        self.check_baseline_focus("focus_after_A")

        opened_b = self.open_poc_tab("B", f"VCU POC {self.nonce} B")
        tab_b = opened_b["tab"]["tab_id"]
        group_b = opened_b["group"]["group_id"]
        self.wait_fixture("B", tab_b)
        self.check_baseline_focus("focus_after_B")
        self.assertion(
            "new_windows_and_groups_isolated",
            ["tabs"],
            opened_a["tab"]["window_id"] != opened_b["tab"]["window_id"] and group_a != group_b,
            {
                "window_A": opened_a["tab"]["window_id"],
                "window_B": opened_b["tab"]["window_id"],
                "group_A": group_a,
                "group_B": group_b,
            },
        )

        _, tabs_before_cross = self.list_tabs("tabs_before_cross_window_group")
        expected_groups = {
            tab_a: self.tab_by_id(tabs_before_cross, tab_a).get("group") if self.tab_by_id(tabs_before_cross, tab_a) else opened_a["group"],
            tab_b: self.tab_by_id(tabs_before_cross, tab_b).get("group") if self.tab_by_id(tabs_before_cross, tab_b) else opened_b["group"],
        }
        self.run_step(
            "cross_window_group_rejected",
            ["group", "--tabs", f"{tab_a},{tab_b}", "--title", f"VCU POC CROSS {self.nonce}"],
            self.expect_error(lambda text: "same window" in text or "window" in text, "cross_window_group"),
        )
        _, tabs_after_cross = self.list_tabs("groups_unchanged_after_cross_window_rejection")
        unchanged, unchanged_detail = self.group_unchanged(tabs_after_cross, expected_groups)
        self.assertion("original_groups_unchanged", ["tabs"], unchanged, unchanged_detail)

        opened_a2 = self.open_poc_tab("A2", group_a)
        tab_a2 = opened_a2["tab"]["tab_id"]
        self.assertion(
            "same_window_group_open",
            ["tabs"],
            opened_a2["tab"]["window_id"] == opened_a["tab"]["window_id"] and opened_a2["group"]["group_id"] == group_a,
            {"tab_A2": tab_a2, "window_A2": opened_a2["tab"]["window_id"], "group_A2": opened_a2["group"]["group_id"]},
        )
        self.check_baseline_focus("focus_after_A2")

        def collapse_ok(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            group = group_summary(payload(result.parsed).get("group"))
            return bool(group and group.get("group_id") == group_a and group.get("collapsed") is True), {"group": group}

        self.run_step("collapse_group_A", ["group-update", "--group", group_a, "--collapsed", "true"], collapse_ok)

        def select_ok(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            tab = self.tab_by_id(tabs_from(result), tab_a)
            return bool(tab and tab.get("focused") and tab.get("group", {}).get("collapsed") is False), {"tab": tab}

        self.run_step("select_A_expands_group", ["select", "--tab", tab_a], select_ok)
        _, selected_tabs = self.list_tabs("selected_A_state")
        selected_a = self.tab_by_id(selected_tabs, tab_a)
        self.assertion(
            "selected_A_focused_and_expanded",
            ["tabs"],
            bool(selected_a and selected_a.get("focused") and not selected_a.get("group", {}).get("collapsed")),
            {"tab": selected_a},
        )

        capture_cover = self.capture("A", tab_a, "capture_before_css_overlay")
        self.stable_pixel_dry_run(tab_a, capture_cover)
        self.selector_click(tab_a, "#cover-css", "selector_click_show_css_overlay")
        self.stale_pixel_dry_run(tab_a, str(capture_cover["capture_id"]), "old_capture_after_css_overlay_rejected")
        self.selector_click(tab_a, "#uncover-css", "selector_click_hide_css_overlay")

        capture_one = self.capture("A", tab_a, "capture_before_css_move")
        capture_one_id = str(capture_one.get("capture_id"))
        self.selector_click(tab_a)
        self.stale_pixel_dry_run(tab_a, capture_one_id, "old_capture_after_css_move_rejected")

        capture_two = self.capture("A", tab_a, "capture_before_input_event")
        capture_two_id = str(capture_two.get("capture_id"))
        self.type_local_input(tab_a)
        self.stale_pixel_dry_run(tab_a, capture_two_id, "old_capture_after_input_rejected")

        def count_zero(result: CommandResult) -> tuple[bool, dict[str, Any]]:
            if not top_ok(result):
                return False, {"response": "not_ok"}
            matches = payload(result.parsed).get("matches")
            text = " ".join(str(item.get("text") or "") for item in matches if isinstance(item, dict)) if isinstance(matches, list) else ""
            return text.strip() == "clicks=0", {"counter_text": text[:32]}

        self.run_step("counter_remains_zero", ["extract", "--selector", "#count", "--tab", tab_a], count_zero)
        self.report["created_tabs"] = list(self.created_tabs)
        self.report["captures"] = [capture_summary(capture_cover), capture_summary(capture_one), capture_summary(capture_two)]
        self.report["baseline_focused_tab"] = self.baseline_focused_tab
        self.report["group_checks"] = {
            "window_A": opened_a["tab"]["window_id"],
            "window_B": opened_b["tab"]["window_id"],
            "group_A": group_a,
            "group_B": group_b,
            "tab_A2": tab_a2,
        }
        self.report["ok"] = True

    def cleanup_tabs(self) -> None:
        if not self.created_tabs:
            return
        # Restore the user's focus only when the currently focused tab is one
        # of this script's own tabs. A manual user switch is left alone.
        try:
            current = self.runner.run(["tabs"], timeout=15)
            current_tabs = tabs_from(current)
            own_ids = {item["tab_id"] for item in self.created_tabs}
            own_focused = any(tab.get("focused") and tab.get("tab_id") in own_ids for tab in current_tabs)
            baseline_exists = any(tab.get("tab_id") == self.baseline_focused_tab for tab in current_tabs)
            if own_focused and baseline_exists and self.baseline_focused_tab:
                started = time.monotonic()
                restore = self.runner.run(["select", "--tab", self.baseline_focused_tab], timeout=15)
                self.cleanup.append({
                    "action": "restore_focus",
                    "tab_id": self.baseline_focused_tab,
                    "pass": top_ok(restore),
                    "returncode": restore.returncode,
                    "latency_ms": round((time.monotonic() - started) * 1000, 2),
                    "command": self.command_label(["select", "--tab", self.baseline_focused_tab]),
                })
        except Exception:
            self.cleanup.append({"action": "restore_focus", "pass": False, "reason": "query_failed"})

        for created in reversed(self.created_tabs):
            tab_id = created["tab_id"]
            started = time.monotonic()
            listing = self.runner.run(["tabs"], timeout=15)
            if not top_ok(listing):
                self.cleanup.append({
                    "action": "close_tab",
                    "tab_id": tab_id,
                    "window_id": created["window_id"],
                    "pass": False,
                    "status": "tabs_query_failed",
                    "latency_ms": round((time.monotonic() - started) * 1000, 2),
                    "command": self.command_label(["tabs"]),
                })
                continue
            raw_tabs = payload(listing.parsed).get("tabs")
            raw_tab = None
            if isinstance(raw_tabs, list):
                raw_tab = next((tab for tab in raw_tabs if isinstance(tab, dict) and str(tab.get("tab_id")) == tab_id), None)
            if raw_tab is None:
                self.cleanup.append({
                    "action": "close_tab",
                    "tab_id": tab_id,
                    "window_id": created["window_id"],
                    "pass": True,
                    "status": "already_closed",
                    "latency_ms": round((time.monotonic() - started) * 1000, 2),
                    "command": self.command_label(["tabs"]),
                })
                continue

            current_window = str(raw_tab.get("window_id"))
            current_url = str(raw_tab.get("url") or "")
            local_fixture = (
                current_url.startswith(self.base_url + FIXTURE_PATH)
                and f"nonce={self.nonce}" in current_url
            )
            if current_window != created["window_id"] or not local_fixture:
                # The user may have navigated or moved this tab. Preserve it;
                # this script must never close a tab after that handoff.
                self.cleanup.append({
                    "action": "close_tab",
                    "tab_id": tab_id,
                    "window_id": created["window_id"],
                    "pass": True,
                    "status": "preserved_user_change",
                    "latency_ms": round((time.monotonic() - started) * 1000, 2),
                    "command": self.command_label(["tabs"]),
                })
                continue

            result = self.runner.run(["close", "--tab", tab_id], timeout=15)
            passed = top_ok(result)
            status = "closed" if passed else "close_failed"
            if not passed:
                # A concurrent user close is considered already closed; do not
                # replay the close mutation.
                check = self.runner.run(["tabs"], timeout=15)
                check_tabs = payload(check.parsed).get("tabs") if top_ok(check) else None
                still_exists = isinstance(check_tabs, list) and any(
                    isinstance(tab, dict) and str(tab.get("tab_id")) == tab_id for tab in check_tabs
                )
                if not still_exists:
                    passed = True
                    status = "already_closed"
            self.cleanup.append({
                "action": "close_tab",
                "tab_id": tab_id,
                "window_id": created["window_id"],
                "pass": passed,
                "status": status,
                "returncode": result.returncode,
                "latency_ms": round((time.monotonic() - started) * 1000, 2),
                "command": self.command_label(["close", "--tab", tab_id]),
            })


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Run the controlled USER-browser multi-window/layout parity POC.",
        epilog="Real browser actions are refused unless --live is supplied. Only the local layout-state fixture is used.",
    )
    parser.add_argument("--live", action="store_true", help="explicitly enable local USER-browser actions")
    parser.add_argument("--vcu", default="target/debug/vcu", help="path to the vcu CLI (default: target/debug/vcu)")
    parser.add_argument("--output", default=".local/browser-parity/multi-window-poc.json", help="local JSON report path")
    parser.add_argument("--nodes", type=int, default=400, help="local fixture interactive node count (default: 400)")
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    if not args.live:
        print(parser.format_help())
        print("Refusing real browser actions. Re-run with --live only for the controlled local fixture.", file=sys.stderr)
        return 2
    if args.nodes < 1 or args.nodes > 1600:
        print("--nodes must be between 1 and 1600 for layout-state.html", file=sys.stderr)
        return 2

    poc = Poc(args)
    try:
        poc.run()
    except PocAbort as exc:
        poc.report["ok"] = False
        poc.report["failure"] = str(exc)
    except Exception as exc:
        poc.report["ok"] = False
        poc.report["failure"] = type(exc).__name__
    finally:
        try:
            poc.cleanup_tabs()
        finally:
            poc.stop_server()
            poc.report.setdefault("created_tabs", list(poc.created_tabs))
            poc.report["cleanup_ok"] = all(item.get("pass") is True for item in poc.cleanup)
            if not poc.report["cleanup_ok"]:
                poc.report["ok"] = False
            poc.report["finished_at"] = time.time()
            output = Path(args.output)
            output.parent.mkdir(parents=True, exist_ok=True)
            output.write_text(json.dumps(poc.report, ensure_ascii=False, indent=2) + "\n")

    print(json.dumps({"ok": poc.report.get("ok") is True, "output": str(Path(args.output)), "steps": len(poc.steps)}, ensure_ascii=False))
    return 0 if poc.report.get("ok") is True else 1


if __name__ == "__main__":
    raise SystemExit(main())
