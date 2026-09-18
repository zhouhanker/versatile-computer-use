// VCU extension service worker — Codex-like: auto-pair with local daemon, no CDP Allow dialogs.
const DEFAULT_ENDPOINT = "http://127.0.0.1:17890";
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

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  (async () => {
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
    await fetchJson(endpoint + "/v1/extension/hello", {
      method: "POST",
      headers: { "X-Vcu-Token": pairingToken, "Content-Type": "application/json" },
      body: JSON.stringify({ likely_user_profile, hosts }),
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

async function listTabs() {
  const tabs = await chrome.tabs.query({});
  let focusedId = null;
  try {
    const f = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
    if (f && f[0]) focusedId = String(f[0].id);
  } catch (_) {}
  return tabs.map((t) => ({
    tab_id: String(t.id),
    window_id: String(t.windowId),
    title: t.title || "",
    url: t.url || "",
    active: !!t.active,
    focused: String(t.id) === focusedId,
    agent_owned: String(t.windowId) === String(agentWindowId),
  }));
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
          const body = await fetchJson(endpoint + "/v1/extension/poll?wait_ms=4000", {
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

async function resolveHttpTab(preferredId) {
  const tabs = await listTabs();
  const id = preferredId ? String(preferredId) : "";
  const http = tabs.filter((t) => (t.url || "").startsWith("http") && !isRestrictedUrl(t.url) && !t.agent_owned);
  if (id) {
    const hit = http.find((t) => t.tab_id === id) || tabs.find((t) => t.tab_id === id);
    if (hit && !isRestrictedUrl(hit.url)) return Number(hit.tab_id);
  }
  const focused = http.find((t) => t.focused) || http.find((t) => t.active);
  if (focused) return Number(focused.tab_id);
  const eth = http.find((t) => /etherscan\.io/i.test(t.url || ""));
  if (eth) return Number(eth.tab_id);
  const first = http[0] || tabs.find((t) => (t.url || "").startsWith("http") && !isRestrictedUrl(t.url));
  return first ? Number(first.tab_id) : 0;
}

async function sendToTab(tabId, payload, ms) {
  try {
    const msg = await withTimeout(chrome.tabs.sendMessage(tabId, payload), ms, null);
    return msg;
  } catch (_) {
    return null;
  }
}

async function extractViaContent(tabId, selector) {
  let msg = await sendToTab(tabId, { type: "vcu_extract", selector }, 400);
  if (msg && msg.ok) return Object.assign({ via: "content_message" }, msg);
  try {
    const injected = await withTimeout(chrome.scripting.executeScript({
      target: { tabId },
      files: ["content.js"],
      injectImmediately: true,
    }), 900, null);
    if (!injected) return { ok: false, error: "inject timeout" };
  } catch (e) {
    return { ok: false, error: "inject content.js: " + String(e) };
  }
  msg = await sendToTab(tabId, { type: "vcu_extract", selector }, 400);
  if (msg && msg.ok) return Object.assign({ via: "content_inject" }, msg);
  return { ok: false, error: "no content script" };
}

async function typeViaContent(tabId, selector, text, dryRun) {
  let msg = await sendToTab(tabId, { type: "vcu_type", selector: selector || null, text: text || "", dry_run: !!dryRun }, 1500);
  if (msg && msg.ok) return Object.assign({ via: "content_message" }, msg);
  try {
    await withTimeout(chrome.scripting.executeScript({
      target: { tabId },
      files: ["content.js"],
      injectImmediately: true,
    }), 2500, null);
  } catch (e) {
    return { ok: false, error: "inject content.js: " + String(e) };
  }
  msg = await sendToTab(tabId, { type: "vcu_type", selector: selector || null, text: text || "", dry_run: !!dryRun }, 1500);
  if (msg && msg.ok) return Object.assign({ via: "content_inject" }, msg);
  return { ok: false, error: "type content timeout" };
}

async function clickViaContent(tabId, selector, dryRun) {
  let msg = await sendToTab(tabId, { type: "vcu_click", selector: selector || null, dry_run: !!dryRun }, 400);
  if (msg && msg.ok) return Object.assign({ via: "content_message" }, msg);
  try {
    const injected = await withTimeout(chrome.scripting.executeScript({
      target: { tabId },
      files: ["content.js"],
      injectImmediately: true,
    }), 900, null);
    if (!injected) return { ok: false, error: "inject timeout" };
  } catch (e) {
    return { ok: false, error: "inject content.js: " + String(e) };
  }
  msg = await sendToTab(tabId, { type: "vcu_click", selector: selector || null, dry_run: !!dryRun }, 400);
  if (msg && msg.ok) return Object.assign({ via: "content_inject" }, msg);
  return { ok: false, error: "no content script" };
}

async function handleCommand(cmd) {
  try {
    switch (cmd.method) {
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
        const url = cmd.params.url || "";
        if (!url || !/^https?:/i.test(url)) return { ok: false, error: "http(s) url required" };
        const tab = await chrome.tabs.create({ url, active: true });
        return { ok: true, tab_id: String(tab.id), url: tab.url || url, os_cursor_used: false };
      }
      case "reload_self":
        setTimeout(() => chrome.runtime.reload(), 50);
        return { ok: true, reloading: true };
      case "list_tabs":
        return { ok: true, tabs: await listTabs() };
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
        const tabs = await listTabs();
        const tabId = await resolveHttpTab(cmd.params.tab_id);
        if (!tabId) return { ok: false, error: "no injectable http tab" };
        let selector = cmd.params.selector || null;
        if (!selector && cmd.params.ref) selector = '[data-vcu-ref="' + cmd.params.ref + '"]';
        if (!selector) return { ok: false, error: "click requires selector or ref" };
        const meta = tabs.find((t) => String(t.tab_id) === String(tabId)) || {};
        const result = await clickViaContent(tabId, selector, !!cmd.params.dry_run);
        return Object.assign({ tab_id: String(tabId), page_url: meta.url || "", focused: !!meta.focused }, result || { ok: false, error: "no result" });
      }
      case "hover": {
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId: Number(cmd.params.tab_id) },
          injectImmediately: true,
          func: (ref) => {
            const el = document.querySelector('[data-vcu-ref="' + ref + '"]');
            if (!el) return { ok: false, error: "not found" };
            const r = el.getBoundingClientRect();
            let c = document.getElementById("vcu-virtual-cursor");
            if (!c) {
              c = document.createElement("div");
              c.id = "vcu-virtual-cursor";
              c.style.cssText = "position:fixed;z-index:2147483647;width:18px;height:18px;margin-left:-9px;margin-top:-9px;border:2px solid #4c8dff;border-radius:50%;background:rgba(76,141,255,.35);pointer-events:none;transition:left .12s linear,top .12s linear";
              document.documentElement.appendChild(c);
            }
            c.style.left = (r.left + r.width / 2) + "px";
            c.style.top = (r.top + r.height / 2) + "px";
            el.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
            el.dispatchEvent(new MouseEvent("mouseenter", { bubbles: true }));
            return { ok: true, ref, input_path: "extension", os_cursor_used: false };
          },
          args: [cmd.params.ref],
        });
        return result;
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
        const tabId = await resolveHttpTab(cmd.params.tab_id);
        if (!tabId) return { ok: false, error: "no injectable http tab" };
        let selector = cmd.params.selector || null;
        if (!selector && cmd.params.ref) selector = '[data-vcu-ref="' + cmd.params.ref + '"]';
        const result = await typeViaContent(tabId, selector, cmd.params.text || "", !!cmd.params.dry_run);
        return Object.assign({ tab_id: String(tabId) }, result || { ok: false, error: "no result" });
      }
      case "extract": {
        const tabs = await listTabs();
        const http = tabs.filter((t) => (t.url || "").startsWith("http") && !isRestrictedUrl(t.url) && !t.agent_owned);
        const prefer = cmd.params.tab_id != null ? String(cmd.params.tab_id) : "";
        const score = (t) => {
          const url = t.url || "";
          if (prefer && String(t.tab_id) === prefer) return 0;
          if (t.focused) return 1;
          if (t.active) return 2;
          if (/etherscan\.io/i.test(url)) return 3;
          return 10;
        };
        const ordered = http.slice().sort((a, b) => score(a) - score(b));
        let last = { ok: false, error: "no injectable http tab" };
        for (const t of ordered.slice(0, 8)) {
          const id = Number(t.tab_id);
          const result = await extractViaContent(id, cmd.params.selector || "a");
          if (result && result.ok) return Object.assign({ tab_id: String(id), page_url: t.url }, result);
          last = Object.assign({ tab_id: String(id), page_url: t.url }, result || { ok: false });
        }
        return last;
      }
      case "scroll": {
        const tabId = await resolveHttpTab(cmd.params.tab_id);
        if (!tabId) return { ok: false, error: "no injectable http tab" };
        const dy = cmd.params.dy || 400;
        let msg = await sendToTab(tabId, { type: "vcu_scroll", dy, dry_run: !!cmd.params.dry_run }, 1500);
        if (msg && msg.ok) return Object.assign({ tab_id: String(tabId), via: "content_message" }, msg);
        try {
          await withTimeout(chrome.scripting.executeScript({
            target: { tabId },
            files: ["content.js"],
            injectImmediately: true,
          }), 2500, null);
        } catch (e) {
          return { ok: false, error: "inject content.js: " + String(e) };
        }
        msg = await sendToTab(tabId, { type: "vcu_scroll", dy, dry_run: !!cmd.params.dry_run }, 1500);
        if (msg && msg.ok) return Object.assign({ tab_id: String(tabId), via: "content_inject" }, msg);
        return { ok: false, error: "scroll content timeout", tab_id: String(tabId) };
      }
      case "wait": {
        const ms = Math.min(Math.max(Number(cmd.params.ms) || 100, 0), 15000);
        await sleep(ms);
        return { ok: true, waited_ms: ms, os_cursor_used: false };
      }
      case "screenshot": {
        const dataUrl = await chrome.tabs.captureVisibleTab(agentWindowId, { format: "png" });
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
