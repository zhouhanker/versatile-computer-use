// VCU extension service worker — Codex-like: auto-pair with local daemon, no CDP Allow dialogs.
const DEFAULT_ENDPOINT = "http://127.0.0.1:17890";
const DEFAULT_GROUP_COLOR = "purple";
const TAB_GROUP_COLORS = new Set([
  "grey", "blue", "red", "yellow", "green", "pink", "purple", "cyan", "orange",
]);

function currentBrowser() {
  try {
    const ua = (globalThis.navigator && globalThis.navigator.userAgent) || "";
    return /Edg\//.test(ua) ? "edge" : "chrome";
  } catch (_) {
    return "chrome";
  }
}
let agentWindowId = null;
let pairingToken = null;
let endpoint = DEFAULT_ENDPOINT;
let polling = false;
let lastError = null;

chrome.runtime.onInstalled.addListener(() => { void bootstrapAndStart();
void ensureOffscreen(); });
chrome.runtime.onStartup.addListener(() => { void bootstrapAndStart(); });
chrome.alarms.create("vcu-keepalive", { periodInMinutes: 1 });
chrome.alarms.onAlarm.addListener((a) => {
  if (a.name === "vcu-keepalive" || a.name === "vcu-quick") void bootstrapAndStart();
});
void bootstrapAndStart();

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  (async () => {
    if (msg.type === "browser_command") {
      const popupUrl = chrome.runtime.getURL("popup.html");
      const allowed = new Set(["list_tabs", "select_tab", "group_tabs", "update_group", "ungroup_tabs"]);
      const command = msg.command || {};
      if (!sender || sender.id !== chrome.runtime.id || sender.url !== popupUrl) {
        sendResponse({ ok: false, error: "browser_command requires the VCU popup sender" });
        return;
      }
      if (!allowed.has(command.method)) {
        sendResponse({ ok: false, error: "browser_command method not allowed" });
        return;
      }
      sendResponse(await handleCommand({ method: command.method, params: command.params || {} }));
      return;
    }
    if (msg.type === "set_config") {
      endpoint = msg.endpoint || endpoint;
      pairingToken = msg.token || pairingToken;
      await chrome.storage.local.set({ endpoint, token: pairingToken });
      await hello();
      await ensureOffscreen();
      startPollLoop();
      const id = await ensureAgentWindow();
      sendResponse({ ok: true, endpoint, hasToken: !!pairingToken, windowId: id });
      return;
    }
    if (msg.type === "status") {
      sendResponse({
        ok: true, endpoint, hasToken: !!pairingToken, agentWindowId, polling, lastError,
        connected: !!pairingToken && polling,
      });
      return;
    }
    if (msg.type === "ensure_agent_window") {
      sendResponse({ ok: true, windowId: await ensureAgentWindow() });
      return;
    }
    if (msg.type === "keepalive") {
      await hello();
      if (!polling) startPollLoop();
      sendResponse({ ok: true });
      return;
    }
    if (msg.type === "rebootstrap") {
      await bootstrapAndStart(true);
      sendResponse({ ok: true, hasToken: !!pairingToken, endpoint, lastError });
      return;
    }
    sendResponse({ ok: false, error: "unknown" });
  })();
  return true;
});

function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

let captureQueue = Promise.resolve();
let lastCaptureAt = 0;
function captureVisiblePng(windowId) {
  const job = captureQueue.then(async () => {
    const perSecond = Number(chrome.tabs.MAX_CAPTURE_VISIBLE_TAB_CALLS_PER_SECOND) || 2;
    const interval = Math.ceil(1000 / perSecond) + 25;
    const wait = Math.max(0, lastCaptureAt + interval - Date.now());
    if (wait) await sleep(wait);
    lastCaptureAt = Date.now();
    try {
      return await chrome.tabs.captureVisibleTab(windowId, { format: "png" });
    } catch (error) {
      // A worker restart may inherit the browser's quota while losing the
      // in-memory clock. Screenshots are read-only, so one bounded retry is safe.
      if (!String(error).includes("MAX_CAPTURE_VISIBLE_TAB_CALLS_PER_SECOND")) throw error;
      await sleep(1000);
      lastCaptureAt = Date.now();
      return chrome.tabs.captureVisibleTab(windowId, { format: "png" });
    }
  });
  captureQueue = job.catch(() => {});
  return job;
}

function withTimeout(promise, ms, timeoutValue) {
  let t;
  const timeout = new Promise((resolve) => {
    t = setTimeout(() => resolve(timeoutValue), ms);
  });
  return Promise.race([promise, timeout]).finally(() => clearTimeout(t));
}

function isRestrictedUrl(url) {
  if (!url) return true;
  return /^(chrome|edge|devtools|chrome-extension|edge-extension|about|data|blob|file):/i.test(url)
    || url.startsWith("https://chrome.google.com/webstore")
    || url.startsWith("https://chromewebstore.google.com")
    || url.startsWith("https://microsoftedge.microsoft.com/addons");
}

async function ensureOffscreen() {
  try {
    if (!chrome.offscreen) return;
    const ctxs = await chrome.runtime.getContexts({ contextTypes: ["OFFSCREEN_DOCUMENT"] });
    if (ctxs && ctxs.length) return;
    await chrome.offscreen.createDocument({
      url: "offscreen.html",
      reasons: ["BLOBS"],
      justification: "Keep VCU extension poll loop alive without UI clicks",
    });
  } catch (e) {
    lastError = "offscreen:" + String(e);
  }
}

async function bootstrapAndStart(force = false) {
  try {
    const stored = await chrome.storage.local.get(["endpoint", "token"]);
    endpoint = stored.endpoint || DEFAULT_ENDPOINT;
    pairingToken = stored.token || null;
    if (!pairingToken || force) {
      const boot = await tryBootstrap();
      if (boot && boot.token) {
        pairingToken = boot.token;
        if (boot.endpoint) endpoint = boot.endpoint;
        await chrome.storage.local.set({ endpoint, token: pairingToken });
      }
    }
    if (pairingToken) {
      await hello();
      await ensureOffscreen();
      startPollLoop();
      lastError = null;
      try { chrome.alarms.create("vcu-quick", { delayInMinutes: 0.5 }); } catch (_) {}
    } else {
      lastError = "no_token_bootstrap_failed";
      setTimeout(() => void bootstrapAndStart(true), 5000);
    }
  } catch (e) {
    lastError = String(e);
    setTimeout(() => void bootstrapAndStart(true), 8000);
  }
}

async function tryBootstrap() {
  try {
    const resp = await fetch(endpoint + "/v1/extension/bootstrap", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ client: "vcu-extension", version: chrome.runtime.getManifest().version }),
    });
    if (!resp.ok) return null;
    const body = await resp.json();
    const data = body && body.data ? body.data : body;
    if (data && data.token) return data;
  } catch (_) {}
  return null;
}

async function hello() {
  if (!pairingToken) return;
  try {
    let hosts = [];
    try {
      const tabs = await chrome.tabs.query({});
      for (const t of tabs) {
        try {
          const h = new URL(t.url || "about:blank").hostname;
          if (h && !hosts.includes(h)) hosts.push(h);
        } catch (_) {}
        if (hosts.length >= 12) break;
      }
    } catch (_) {}
    const likely_user_profile = hosts.some((h) => h && !h.endsWith("msn.com"));
    const browser = currentBrowser();
    await fetchJson(endpoint + "/v1/extension/hello", {
      method: "POST",
      headers: { "X-Vcu-Token": pairingToken, "Content-Type": "application/json" },
      body: JSON.stringify({
        likely_user_profile,
        hosts,
        client_id: chrome.runtime.id,
        browser,
      }),
    }, 5000);
  } catch (e) {
    lastError = "hello_failed:" + String(e);
  }
}

async function ensureAgentWindow() {
  if (agentWindowId === null) {
    const stored = await chrome.storage.local.get(["agentWindowId"]);
    if (stored.agentWindowId != null) agentWindowId = stored.agentWindowId;
  }
  if (agentWindowId !== null) {
    try { await chrome.windows.get(agentWindowId); return agentWindowId; } catch { agentWindowId = null; }
  }
  const win = await chrome.windows.create({ url: "about:blank", type: "normal", focused: false, width: 1100, height: 800 });
  agentWindowId = win.id;
  await chrome.storage.local.set({ agentWindowId });
  return agentWindowId;
}

function groupInfo(group) {
  if (!group || group.id == null) return null;
  return {
    group_id: String(group.id),
    window_id: String(group.windowId),
    title: group.title || "",
    color: group.color || DEFAULT_GROUP_COLOR,
    collapsed: !!group.collapsed,
  };
}

async function queryTabGroups() {
  if (!chrome.tabGroups || typeof chrome.tabGroups.query !== "function") return [];
  const groups = await chrome.tabGroups.query({});
  return (groups || []).map(groupInfo).filter(Boolean);
}

async function listTabsState() {
  const rawTabs = await chrome.tabs.query({});
  let focusedId = null;
  try {
    const f = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
    if (f && f[0]) focusedId = String(f[0].id);
  } catch (_) {}
  const groups = await queryTabGroups();
  const groupById = new Map(groups.map((g) => [g.group_id, g]));
  const tabs = (rawTabs || []).map((t) => {
    const rawGroupId = t.groupId;
    const numericGroupId = Number(rawGroupId);
    const groupId = rawGroupId != null && Number.isFinite(numericGroupId) && numericGroupId >= 0
      ? String(numericGroupId)
      : null;
    return {
      tab_id: String(t.id),
      window_id: String(t.windowId),
      title: t.title || "",
      url: t.url || "",
      active: !!t.active,
      focused: String(t.id) === focusedId,
      pinned: !!t.pinned,
      controllable: /^https?:/i.test(t.url || "") && !isRestrictedUrl(t.url),
      group_id: groupId,
      group: groupId ? (groupById.get(groupId) || null) : null,
      agent_owned: String(t.windowId) === String(agentWindowId),
      browser: currentBrowser(),
    };
  });
  return { tabs, groups };
}

async function listTabs() {
  return (await listTabsState()).tabs;
}

function tabIdNumber(id) {
  if (id === undefined || id === null || id === "") return null;
  const n = Number(id);
  return Number.isInteger(n) && n >= 0 ? n : null;
}

function isInjectableHttpTab(tab) {
  return !!tab && /^https?:/i.test(tab.url || "") && !isRestrictedUrl(tab.url);
}

function tabActionState(tab) {
  return {
    tab_id: tab ? String(tab.tab_id) : null,
    page_url: tab ? (tab.url || "") : "",
    focused: !!(tab && tab.focused),
  };
}

async function resolveHttpTab(preferredId) {
  const { tabs } = await listTabsState();
  const hasPreferred = preferredId !== undefined && preferredId !== null;
  if (hasPreferred) {
    const id = String(preferredId);
    const hit = tabs.find((t) => t.tab_id === id);
    // An explicit target is authoritative. A missing, restricted, non-http, or
    // otherwise non-injectable target must never silently select another tab.
    return hit && isInjectableHttpTab(hit) ? tabIdNumber(hit.tab_id) : 0;
  }
  // The default is deliberately narrow: only the last-focused USER tab. The
  // active tab in another window, an Etherscan tab, and the first HTTP tab are
  // not valid implicit targets.
  const focused = tabs.find((t) => t.focused && !t.agent_owned && isInjectableHttpTab(t));
  return focused ? tabIdNumber(focused.tab_id) : 0;
}

async function resolveHttpTabState(preferredId) {
  const { tabs } = await listTabsState();
  const hasPreferred = preferredId !== undefined && preferredId !== null && String(preferredId) !== "";
  if (hasPreferred) {
    const hit = tabs.find((t) => t.tab_id === String(preferredId));
    if (!hit) return { mismatch: true };
    if (!isInjectableHttpTab(hit)) return null;
    const id = tabIdNumber(hit.tab_id);
    return id ? { id, tab: hit } : null;
  }
  const focused = tabs.find((t) => t.focused && !t.agent_owned && isInjectableHttpTab(t));
  if (!focused) return null;
  const id = tabIdNumber(focused.tab_id);
  return id ? { id, tab: focused } : null;
}

function wrongBrowserOrMissing(resolved) {
  if (resolved && resolved.mismatch) {
    return { ok: false, error: "wrong_extension_browser", retryable: true };
  }
  if (!resolved) return { ok: false, error: "no injectable http tab" };
  return null;
}

async function fetchJson(url, opts = {}, timeoutMs = 10000) {
  const ctrl = new AbortController();
  const t = setTimeout(() => ctrl.abort(), timeoutMs);
  try {
    const resp = await fetch(url, Object.assign({}, opts, { signal: ctrl.signal }));
    return await resp.json();
  } finally {
    clearTimeout(t);
  }
}

let pollGen = 0;
function startPollLoop() {
  if (polling) return;
  const gen = ++pollGen;
  polling = true;
  (async function loop() {
    try {
      while (gen === pollGen && pairingToken) {
        try {
          const body = await fetchJson(endpoint + "/v1/extension/poll?wait_ms=4000&client_id=" + encodeURIComponent(chrome.runtime.id || ""), {
            headers: { "X-Vcu-Token": pairingToken },
          }, 10000);
          const cmd = body && body.data;
          if (cmd && cmd.id && cmd.method) {
            const result = await withTimeout(
              handleCommand(cmd),
              6000,
              { ok: false, error: "command timeout " + cmd.method }
            );
            await fetchJson(endpoint + "/v1/extension/result", {
              method: "POST",
              headers: { "X-Vcu-Token": pairingToken, "Content-Type": "application/json" },
              body: JSON.stringify({ id: cmd.id, result: result || { ok: false, error: "empty result" } }),
            }, 8000);
          }
        } catch (e) {
          lastError = String(e);
          await new Promise((r) => setTimeout(r, 1200));
        }
      }
    } finally {
      if (gen === pollGen) polling = false;
    }
  })();
}

async function waitTabComplete(tabId, timeoutMs) {
  const deadline = Date.now() + (timeoutMs || 12000);
  while (Date.now() < deadline) {
    try {
      const t = await chrome.tabs.get(tabId);
      if (t && t.status === "complete") return t;
    } catch (_) {
      return null;
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  try { return await chrome.tabs.get(tabId); } catch (_) { return null; }
}

async function sendToTab(tabId, payload, ms) {
  try {
    const msg = await withTimeout(chrome.tabs.sendMessage(tabId, payload), ms, null);
    return msg;
  } catch (_) {
    return null;
  }
}

async function injectContent(tabId, timeoutMs) {
  try {
    const injected = await withTimeout(chrome.scripting.executeScript({
      target: { tabId },
      files: ["content.js"],
      injectImmediately: true,
    }), timeoutMs, null);
    if (!injected) return { ok: false, error: "inject timeout" };
    return { ok: true };
  } catch (e) {
    return { ok: false, error: "inject content.js: " + String(e) };
  }
}

async function ensureContent(tabId, injectTimeoutMs, pingTimeoutMs) {
  const expectedVersion = chrome.runtime.getManifest().version;
  let ping = await sendToTab(tabId, { type: "vcu_ping_page" }, pingTimeoutMs);
  if (ping) {
    if (ping.ok && ping.version === expectedVersion) return { ok: true, via: "content_message" };
    if (ping.ok) return { ok: false, error: "content lens is stale; reload this page" };
    return ping;
  }

  // A failed/timeout ping has no page mutation attached to it, so it is safe
  // to install the listener. The action itself is sent only once below.
  const injected = await injectContent(tabId, injectTimeoutMs);
  if (!injected.ok) return injected;
  ping = await sendToTab(tabId, { type: "vcu_ping_page" }, pingTimeoutMs);
  if (ping && ping.ok && ping.version === expectedVersion) return { ok: true, via: "content_inject" };
  if (ping && ping.ok) return { ok: false, error: "content lens is stale; reload this page" };
  if (ping && ping.ok === false) return ping;
  return { ok: false, error: "no content script" };
}

async function actionViaContent(tabId, payload, actionName, actionTimeoutMs, injectTimeoutMs, pingTimeoutMs) {
  const ready = await ensureContent(tabId, injectTimeoutMs, pingTimeoutMs);
  if (!ready.ok) return ready;
  // Native CU pointers keep their apparent size when the webpage zooms.
  // Scale only our drawing; event and hit-test coordinates remain CSS px.
  if (typeof chrome.tabs.getZoom === "function") {
    payload.cursor_zoom = await chrome.tabs.getZoom(tabId);
  }
  const msg = await sendToTab(tabId, payload, actionTimeoutMs);
  // An explicit response is authoritative. In particular, do not replay an
  // action after the page reports ok:false: it may already have side effects.
  if (msg && msg.ok) return Object.assign({ via: ready.via }, msg);
  if (msg && msg.ok === false) return msg;
  return { ok: false, error: actionName + " content timeout" };
}

async function extractViaContent(tabId, selector) {
  const ready = await ensureContent(tabId, 900, 400);
  if (!ready.ok) return ready;
  const msg = await sendToTab(tabId, { type: "vcu_extract", selector }, 400);
  if (msg && msg.ok) return Object.assign({ via: ready.via }, msg);
  if (msg && msg.ok === false) return msg;
  return { ok: false, error: "extract content timeout" };
}

async function typeViaContent(tabId, selector, text, dryRun) {
  return actionViaContent(
    tabId,
    { type: "vcu_type", selector: selector || null, text: text || "", dry_run: !!dryRun },
    "type",
    1500,
    2500,
    1500,
  );
}

async function clickViaContent(tabId, selector, dryRun) {
  return actionViaContent(
    tabId,
    { type: "vcu_click", selector: selector || null, dry_run: !!dryRun },
    "click",
    1500,
    900,
    1000,
  );
}

async function scrollViaContent(tabId, dy, dryRun) {
  return actionViaContent(
    tabId,
    { type: "vcu_scroll", dy, dry_run: !!dryRun },
    "scroll",
    1500,
    2500,
    1500,
  );
}

function groupIdNumber(groupId) {
  if (groupId === undefined || groupId === null || groupId === "") return null;
  const n = Number(groupId);
  return Number.isInteger(n) && n >= 0 ? n : null;
}

async function findGroup(groupId) {
  const id = String(groupId);
  const groups = await queryTabGroups();
  return groups.find((g) => g.group_id === id) || null;
}

function normalizeTabIds(rawIds) {
  if (!Array.isArray(rawIds) || rawIds.length === 0) {
    return { ok: false, error: "tab_ids must be a non-empty array" };
  }
  const ids = [];
  const seen = new Set();
  for (const raw of rawIds) {
    const id = tabIdNumber(raw);
    if (id === null) return { ok: false, error: "invalid tab_id " + String(raw) };
    if (seen.has(String(id))) return { ok: false, error: "duplicate tab_id " + String(raw) };
    seen.add(String(id));
    ids.push(id);
  }
  return { ok: true, ids };
}

function validateGroupColor(color) {
  if (color == null || color === "") return DEFAULT_GROUP_COLOR;
  const value = String(color).toLowerCase();
  return TAB_GROUP_COLORS.has(value) ? value : null;
}

async function freshTabsForIds(ids) {
  const { tabs, groups } = await listTabsState();
  const wanted = new Set(ids.map((id) => String(id)));
  return {
    tabs: tabs.filter((t) => wanted.has(t.tab_id)),
    groups,
  };
}

function tabsResult(state, extra = {}) {
  return Object.assign({
    ok: true,
    tabs: state.tabs,
    groups: state.groups,
    source: "extension_tabs",
    os_cursor_used: false,
  }, extra);
}

async function groupTabs(params) {
  const parsed = normalizeTabIds(params.tab_ids);
  if (!parsed.ok) return parsed;
  const title = params.title == null ? "" : String(params.title).trim();
  if (!title) return { ok: false, error: "group title required" };
  const color = validateGroupColor(params.color);
  if (!color) return { ok: false, error: "invalid tab group color" };

  const rawTabs = [];
  try {
    for (const id of parsed.ids) rawTabs.push(await chrome.tabs.get(id));
  } catch (e) {
    return { ok: false, error: "tabs.get: " + String(e) };
  }
  const windowId = rawTabs[0] && String(rawTabs[0].windowId);
  if (rawTabs.some((t) => String(t.windowId) !== windowId)) {
    return { ok: false, error: "all tabs must be in the same window" };
  }
  const invalid = rawTabs.find((t) => !/^https?:/i.test(t.url || "") || isRestrictedUrl(t.url));
  if (invalid) return { ok: false, error: "tab_ids must contain only unrestricted http(s) tabs" };
  const pinned = rawTabs.find((t) => t.pinned);
  if (pinned) return { ok: false, error: "pinned tabs cannot be grouped" };

  let groupId;
  try {
    groupId = await chrome.tabs.group({ tabIds: parsed.ids, createProperties: { windowId: Number(windowId) } });
    await chrome.tabGroups.update(groupId, {
      title,
      color,
      collapsed: params.collapsed == null ? false : !!params.collapsed,
    });
  } catch (e) {
    return { ok: false, error: "group tabs: " + String(e) };
  }
  const state = await freshTabsForIds(parsed.ids);
  const group = state.groups.find((g) => g.group_id === String(groupId)) || null;
  return tabsResult(state, { group });
}

async function updateGroup(params) {
  const id = groupIdNumber(params.group_id);
  if (id === null) return { ok: false, error: "invalid group_id" };
  const patch = {};
  if (params.title !== undefined) patch.title = String(params.title);
  if (params.color !== undefined) {
    const color = validateGroupColor(params.color);
    if (!color) return { ok: false, error: "invalid tab group color" };
    patch.color = color;
  }
  if (params.collapsed !== undefined) patch.collapsed = !!params.collapsed;
  if (!Object.keys(patch).length) return { ok: false, error: "group update requires title, color, or collapsed" };
  try {
    await chrome.tabGroups.update(id, patch);
  } catch (e) {
    return { ok: false, error: "tabGroups.update: " + String(e) };
  }
  const group = await findGroup(id);
  return { ok: true, group, source: "extension_tabs", os_cursor_used: false };
}

async function ungroupTabs(params) {
  const parsed = normalizeTabIds(params.tab_ids);
  if (!parsed.ok) return parsed;
  try {
    await chrome.tabs.ungroup(parsed.ids);
  } catch (e) {
    return { ok: false, error: "ungroup tabs: " + String(e) };
  }
  const state = await freshTabsForIds(parsed.ids);
  return tabsResult(state);
}

async function selectTab(params) {
  const id = tabIdNumber(params.tab_id);
  if (id === null) return { ok: false, error: "invalid tab_id" };
  let tab;
  try {
    tab = await chrome.tabs.get(id);
  } catch (e) {
    return { ok: false, error: "tabs.get: " + String(e) };
  }
  if (!isInjectableHttpTab({ url: tab.url || "" })) {
    return { ok: false, error: "select_tab requires an unrestricted http(s) tab" };
  }
  const groupId = Number(tab.groupId);
  try {
    if (Number.isInteger(groupId) && groupId >= 0) {
      const group = await findGroup(groupId);
      if (group && group.collapsed) await chrome.tabGroups.update(groupId, { collapsed: false });
    }
    await chrome.windows.update(tab.windowId, { focused: true });
    await chrome.tabs.update(id, { active: true });
  } catch (e) {
    return { ok: false, error: "select tab: " + String(e) };
  }
  const state = await freshTabsForIds([id]);
  const selected = state.tabs[0] || null;
  const selectedGroup = selected && selected.group_id
    ? state.groups.find((g) => g.group_id === selected.group_id) || selected.group || null
    : null;
  return tabsResult(state, Object.assign(tabActionState(selected), { group: selectedGroup }));
}

function isUserWindowId(id) {
  return id != null && String(id) !== String(agentWindowId);
}

async function userWindowIdForNewTab() {
  try {
    const focused = await chrome.windows.getLastFocused({ windowTypes: ["normal"] });
    if (focused && isUserWindowId(focused.id)) return focused.id;
  } catch (_) {}
  try {
    const wins = await chrome.windows.getAll({ windowTypes: ["normal"] });
    const hit = (wins || []).find((w) => w && w.focused && isUserWindowId(w.id))
      || (wins || []).find((w) => w && isUserWindowId(w.id));
    if (hit) return hit.id;
  } catch (_) {}
  try {
    const tabs = await chrome.tabs.query({});
    const hit = (tabs || []).find((t) => t && t.focused && isUserWindowId(t.windowId))
      || (tabs || []).find((t) => t && t.active && isUserWindowId(t.windowId))
      || (tabs || []).find((t) => t && isUserWindowId(t.windowId));
    if (hit) return hit.windowId;
  } catch (_) {}
  return null;
}

async function openTab(params) {
  const url = params.url || "";
  if (!url || !/^https?:/i.test(url)) return { ok: false, error: "http(s) url required" };
  const hasSessionName = Object.prototype.hasOwnProperty.call(params, "session_name");
  const hasGroupId = Object.prototype.hasOwnProperty.call(params, "group_id");
  if (hasSessionName && hasGroupId) return { ok: false, error: "session_name and group_id are mutually exclusive" };
  const sessionName = hasSessionName && params.session_name != null ? String(params.session_name).trim() : "";
  if (hasSessionName && !sessionName) return { ok: false, error: "session_name must be nonempty" };
  const active = params.active === undefined ? true : !!params.active;
  const newWindow = params.new_window === true;
  if (params.new_window !== undefined && typeof params.new_window !== "boolean") return { ok: false, error: "new_window must be boolean" };
  if (newWindow && hasGroupId) return { ok: false, error: "new_window and group_id are mutually exclusive" };
  let targetGroup = null;
  let targetGroupId = null;
  if (hasGroupId) {
    if (params.group_id == null || String(params.group_id).trim() === "") return { ok: false, error: "invalid group_id" };
    targetGroupId = groupIdNumber(params.group_id);
    if (targetGroupId === null) return { ok: false, error: "invalid group_id" };
    targetGroup = await findGroup(targetGroupId);
    if (!targetGroup) return { ok: false, error: "group not found" };
  }
  const createInfo = { url, active };
  if (targetGroup) createInfo.windowId = Number(targetGroup.window_id);
  let tab;
  try {
    if (newWindow) {
      const win = await chrome.windows.create({ url, type: "normal", focused: active });
      tab = win.tabs?.[0] || (await chrome.tabs.query({ windowId: win.id }))[0];
      if (!tab) return { ok: false, error: "new browser window has no tab", window_id: String(win.id) };
    } else {
      if (createInfo.windowId == null) {
        const windowId = await userWindowIdForNewTab();
        if (windowId == null) return { ok: false, error: "no existing USER window; pass new_window to open one" };
        createInfo.windowId = windowId;
      }
      tab = await chrome.tabs.create(createInfo);
    }
    if (sessionName) {
      const createdGroupId = await chrome.tabs.group({ tabIds: [tab.id], createProperties: { windowId: tab.windowId } });
      await chrome.tabGroups.update(createdGroupId, {
        title: sessionName,
        color: DEFAULT_GROUP_COLOR,
        collapsed: false,
      });
    } else if (targetGroupId !== null) {
      await chrome.tabs.group({ tabIds: [tab.id], groupId: targetGroupId });
      if (active && targetGroup.collapsed) await chrome.tabGroups.update(targetGroupId, { collapsed: false });
    }
    if (active) await chrome.windows.update(tab.windowId, { focused: true });
  } catch (e) {
    return { ok: false, error: "open tab: " + String(e) };
  }
  const state = await freshTabsForIds([tab.id]);
  const opened = state.tabs[0] || null;
  const group = opened && opened.group_id
    ? state.groups.find((g) => g.group_id === opened.group_id) || opened.group || null
    : null;
  return tabsResult(state, Object.assign(tabActionState(opened), {
    group,
    url: opened ? (opened.url || url) : url,
  }));
}

async function handleCommand(cmd) {
  try {
    switch (cmd.method) {
      case "capture_tab": {
        const resolved = await resolveHttpTabState(cmd.params?.tab_id);
        if (resolved && resolved.mismatch) return { ok: false, error: "wrong_extension_browser", retryable: true };
        if (!resolved || !resolved.tab.active) return { ok: false, error: "select the target tab before taking a viewport screenshot" };
        const ready = await ensureContent(resolved.id, 1500, 500);
        if (!ready.ok) return ready;
        const before = await sendToTab(resolved.id, { type: "vcu_viewport", keep_cursor: true }, 700);
        if (!before?.ok || !before.viewport) return { ok: false, error: "viewport metadata unavailable; refresh the page" };
        const dataUrl = await captureVisiblePng(Number(resolved.tab.window_id));
        const after = await sendToTab(resolved.id, { type: "vcu_viewport", keep_cursor: true }, 700);
        const current = await chrome.tabs.get(resolved.id);
        if (!after?.ok || !current.active || current.url !== before.viewport.url || JSON.stringify(before.viewport) !== JSON.stringify(after.viewport)) {
          return { ok: false, error: "page changed during screenshot; capture again" };
        }
        if (!dataUrl?.startsWith("data:image/png;base64,")) return { ok: false, error: "browser did not return a PNG" };
        return { ok: true, ...tabActionState(resolved.tab), viewport: after.viewport, png_base64: dataUrl.split(",")[1], source: "extension_viewport", os_cursor_used: false };
      }
      case "click_point": {
        const resolved = await resolveHttpTabState(cmd.params?.tab_id);
        // The screenshot pins an explicit tab/document. A user switching to
        // another tab must neither retarget the action nor force focus back.
        if (resolved && resolved.mismatch) return { ok: false, error: "wrong_extension_browser", retryable: true };
        if (!resolved) return { ok: false, error: "screenshot target tab is no longer available; capture again" };
        const result = await actionViaContent(resolved.id, {
          type: "vcu_click_point", x: cmd.params.x, y: cmd.params.y,
          expected_viewport: cmd.params.expected_viewport, dry_run: !!cmd.params.dry_run,
        }, "point click", 1500, 1500, 500);
        return Object.assign(tabActionState(resolved.tab), result);
      }
      case "ensure_agent_window":
        return { ok: true, window_id: String(await ensureAgentWindow()) };
      case "ping":
        return {
          ok: true,
          pong: true,
          ts: Date.now(),
          version: chrome.runtime.getManifest().version,
        };
      case "open_tab": {
        return openTab(cmd.params || {});
      }
      case "close_tab": {
        const id = tabIdNumber(cmd.params?.tab_id);
        if (id === null) return { ok: false, error: "an explicit tab_id is required to close a tab" };
        const tab = await chrome.tabs.get(id);
        await chrome.tabs.remove(id);
        const remaining = await chrome.tabs.query({});
        if (remaining.some(t => t.id === id)) return { ok: false, error: "tab did not close", tab_id: String(id) };
        return { ok: true, closed: true, tab_id: String(id), window_id: String(tab.windowId), source: "extension_tabs", os_cursor_used: false };
      }
      case "reload_self":
        polling = false;
        pollGen += 1;
        setTimeout(() => chrome.runtime.reload(), 50);
        return { ok: true, reloading: true, version: chrome.runtime.getManifest().version };
      case "list_tabs": {
        const state = await listTabsState();
        return tabsResult(state);
      }
      case "select_tab":
        return selectTab(cmd.params || {});
      case "group_tabs":
        return groupTabs(cmd.params || {});
      case "update_group":
        return updateGroup(cmd.params || {});
      case "ungroup_tabs":
        return ungroupTabs(cmd.params || {});
      case "navigate": {
        const tabId = Number(cmd.params.tab_id);
        try {
          await chrome.tabs.update(tabId, { url: cmd.params.url, active: false });
        } catch (e) {
          return { ok: false, error: "tabs.update:" + String(e) };
        }
        const tab = await waitTabComplete(tabId, 8000);
        await sleep(200);
        return { ok: true, url: cmd.params.url, status: tab && tab.status, os_cursor_used: false, input_path: "extension" };
      }
      case "snapshot": {
        const tabId = Number(cmd.params.tab_id);
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId },
          injectImmediately: true,
          func: () => {
            const refs = [];
            let i = 0;
            document.querySelectorAll("a,button,input,textarea,select,h1,h2,h3,h4,li,label,summary,[role]").forEach((el) => {
              if (refs.length >= 120) return;
              const id = "e" + ++i;
              try { el.setAttribute("data-vcu-ref", id); } catch (_) {}
              refs.push({
                ref: id,
                role: el.getAttribute("role") || el.tagName.toLowerCase(),
                name: (el.getAttribute("aria-label") || el.innerText || el.value || "").trim().slice(0, 80),
                selector: el.tagName.toLowerCase() + '[data-vcu-ref="' + id + '"]',
              });
            });
            return {
              a11y_summary: 'document title="' + document.title + '" url="' + location.href + '" nodes=' + refs.length,
              text_excerpt: ((document.body && document.body.innerText) || "").slice(0, 8000),
              dom_refs: refs,
              truncated: false,
              budget_tokens_est: refs.length * 10,
            };
          },
        });
        return Object.assign({ ok: true }, result);
      }
      case "click": {
        const params = cmd.params || {};
        const resolved = await resolveHttpTabState(params.tab_id);
        const mismatch = wrongBrowserOrMissing(resolved);
        if (mismatch) return mismatch;
        const tabId = resolved.id;
        let selector = params.selector || null;
        if (!selector && params.ref) selector = '[data-vcu-ref="' + params.ref + '"]';
        if (!selector) return { ok: false, error: "click requires selector or ref" };
        const result = await clickViaContent(tabId, selector, !!params.dry_run);
        return Object.assign(tabActionState(resolved.tab), result || { ok: false, error: "no result" });
      }
      case "hover": {
        const params = cmd.params || {};
        const resolved = await resolveHttpTabState(params.tab_id);
        const mismatch = wrongBrowserOrMissing(resolved);
        if (mismatch) return mismatch;
        let selector = params.selector || null;
        if (!selector && params.ref) selector = '[data-vcu-ref="' + params.ref + '"]';
        if (!selector) return { ok: false, error: "hover requires selector or ref" };
        const result = await actionViaContent(
          resolved.id,
          {
            type: "vcu_hover",
            selector,
            dry_run: !!params.dry_run,
          },
          "hover",
          400,
          900,
          400,
        );
        return Object.assign(tabActionState(resolved.tab), result || { ok: false, error: "no result" });
      }
      case "keypress": {
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId: Number(cmd.params.tab_id) },
          injectImmediately: true,
          func: (key) => {
            const el = document.activeElement || document.body;
            el.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
            el.dispatchEvent(new KeyboardEvent("keyup", { key, bubbles: true }));
            return { ok: true, key, input_path: "extension", os_cursor_used: false };
          },
          args: [cmd.params.key || "Enter"],
        });
        return result;
      }
      case "type": {
        const params = cmd.params || {};
        const resolved = await resolveHttpTabState(params.tab_id);
        const mismatch = wrongBrowserOrMissing(resolved);
        if (mismatch) return mismatch;
        const tabId = resolved.id;
        let selector = params.selector || null;
        if (!selector && params.ref) selector = '[data-vcu-ref="' + params.ref + '"]';
        const result = await typeViaContent(tabId, selector, params.text || "", !!params.dry_run);
        return Object.assign(tabActionState(resolved.tab), result || { ok: false, error: "no result" });
      }
      case "extract": {
        const params = cmd.params || {};
        const resolved = await resolveHttpTabState(params.tab_id);
        const mismatch = wrongBrowserOrMissing(resolved);
        if (mismatch) return mismatch;
        const result = await extractViaContent(resolved.id, params.selector || "a");
        return Object.assign(tabActionState(resolved.tab), result || { ok: false, error: "no result" });
      }
      case "scroll": {
        const params = cmd.params || {};
        const resolved = await resolveHttpTabState(params.tab_id);
        const mismatch = wrongBrowserOrMissing(resolved);
        if (mismatch) return mismatch;
        const tabId = resolved.id;
        const dy = params.dy == null ? 400 : params.dy;
        const result = await scrollViaContent(tabId, dy, !!params.dry_run);
        return Object.assign(tabActionState(resolved.tab), result || { ok: false, error: "no result" });
      }
      case "wait": {
        const ms = Math.min(Math.max(Number(cmd.params.ms) || 100, 0), 15000);
        await sleep(ms);
        return { ok: true, waited_ms: ms, os_cursor_used: false };
      }
      case "screenshot": {
        const dataUrl = await captureVisiblePng(agentWindowId);
        const png_base64 = (dataUrl || "").split(",")[1] || "";
        return { ok: true, png_base64, data_url: dataUrl };
      }
      default:
        return { ok: false, error: "unknown method " + cmd.method };
    }
  } catch (e) {
    return { ok: false, error: String(e) };
  }
}
