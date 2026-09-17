// VCU extension service worker
const DEFAULT_ENDPOINT = "http://127.0.0.1:17890";
let agentWindowId = null;
let pairingToken = null;
let endpoint = DEFAULT_ENDPOINT;
let polling = false;

chrome.runtime.onInstalled.addListener(async () => {
  const stored = await chrome.storage.local.get(["endpoint", "token"]);
  endpoint = stored.endpoint || DEFAULT_ENDPOINT;
  pairingToken = stored.token || null;
  if (pairingToken) startPollLoop();
});

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  (async () => {
    if (msg.type === "set_config") {
      endpoint = msg.endpoint || endpoint;
      pairingToken = msg.token || pairingToken;
      await chrome.storage.local.set({ endpoint, token: pairingToken });
      try {
        await fetch(endpoint + "/v1/extension/hello", {
          method: "POST",
          headers: { "X-Vcu-Token": pairingToken, "Content-Type": "application/json" },
          body: "{}",
        });
      } catch (e) {}
      startPollLoop();
      const id = await ensureAgentWindow();
      sendResponse({ ok: true, endpoint, hasToken: !!pairingToken, windowId: id });
      return;
    }
    if (msg.type === "status") {
      sendResponse({ ok: true, endpoint, hasToken: !!pairingToken, agentWindowId, polling });
      return;
    }
    if (msg.type === "ensure_agent_window") {
      sendResponse({ ok: true, windowId: await ensureAgentWindow() });
      return;
    }
    sendResponse({ ok: false, error: "unknown" });
  })();
  return true;
});

async function ensureAgentWindow() {
  if (agentWindowId !== null) {
    try { await chrome.windows.get(agentWindowId); return agentWindowId; } catch { agentWindowId = null; }
  }
  const win = await chrome.windows.create({ url: "about:blank", type: "normal", focused: false, width: 1100, height: 800 });
  agentWindowId = win.id;
  return agentWindowId;
}

async function listTabs() {
  const tabs = await chrome.tabs.query({});
  return tabs.map((t) => ({
    tab_id: String(t.id),
    window_id: String(t.windowId),
    title: t.title || "",
    url: t.url || "",
    agent_owned: t.windowId === agentWindowId,
  }));
}

function startPollLoop() {
  if (polling) return;
  polling = true;
  (async function loop() {
    while (polling && pairingToken) {
      try {
        await fetch(endpoint + "/v1/extension/hello", {
          method: "POST",
          headers: { "X-Vcu-Token": pairingToken, "Content-Type": "application/json" },
          body: "{}",
        });
        const resp = await fetch(endpoint + "/v1/extension/poll?wait_ms=4000", {
          headers: { "X-Vcu-Token": pairingToken },
        });
        const body = await resp.json();
        const cmd = body && body.data;
        if (cmd && cmd.id && cmd.method) {
          const result = await handleCommand(cmd);
          await fetch(endpoint + "/v1/extension/result", {
            method: "POST",
            headers: { "X-Vcu-Token": pairingToken, "Content-Type": "application/json" },
            body: JSON.stringify({ id: cmd.id, result }),
          });
        }
      } catch (e) {
        await new Promise((r) => setTimeout(r, 1000));
      }
    }
    polling = false;
  })();
}

async function handleCommand(cmd) {
  try {
    switch (cmd.method) {
      case "ensure_agent_window":
        return { ok: true, window_id: String(await ensureAgentWindow()) };
      case "list_tabs":
        return { ok: true, tabs: await listTabs() };
      case "navigate": {
        await chrome.tabs.update(Number(cmd.params.tab_id), { url: cmd.params.url, active: false });
        return { ok: true, url: cmd.params.url, os_cursor_used: false, input_path: "extension" };
      }
      case "snapshot": {
        const tabId = Number(cmd.params.tab_id);
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId },
          func: () => {
            const refs = [];
            let i = 0;
            document.querySelectorAll("a,button,input,textarea,select,h1,h2,h3,[role]").forEach((el) => {
              const id = "e" + ++i;
              el.setAttribute("data-vcu-ref", id);
              refs.push({
                ref: id,
                role: el.getAttribute("role") || el.tagName.toLowerCase(),
                name: (el.getAttribute("aria-label") || el.innerText || el.value || "").trim().slice(0, 80),
                selector: el.tagName.toLowerCase() + '[data-vcu-ref="' + id + '"]',
              });
            });
            return {
              a11y_summary: 'document title="' + document.title + '" url="' + location.href + '" nodes=' + refs.length,
              text_excerpt: ((document.body && document.body.innerText) || "").slice(0, 4000),
              dom_refs: refs,
              truncated: false,
              budget_tokens_est: refs.length * 10,
            };
          },
        });
        return Object.assign({ ok: true }, result);
      }
      case "click": {
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId: Number(cmd.params.tab_id) },
          func: (r) => {
            const el = document.querySelector('[data-vcu-ref="' + r + '"]');
            if (!el) return { ok: false, error: "not found" };
            el.click();
            return { ok: true, clicked: r, input_path: "page_dom", os_cursor_used: false };
          },
          args: [cmd.params.ref],
        });
        return result;
      }
      case "type": {
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId: Number(cmd.params.tab_id) },
          func: (text, r) => {
            const el = r ? document.querySelector('[data-vcu-ref="' + r + '"]') : document.activeElement;
            if (!el) return { ok: false };
            el.focus();
            if ("value" in el) el.value = text;
            el.dispatchEvent(new Event("input", { bubbles: true }));
            return { ok: true, typed: text, ref: r || null, input_path: "page_dom", os_cursor_used: false };
          },
          args: [cmd.params.text, cmd.params.ref || null],
        });
        return result;
      }
      case "extract": {
        const [{ result }] = await chrome.scripting.executeScript({
          target: { tabId: Number(cmd.params.tab_id) },
          func: (selector) => ({
            ok: true,
            matches: Array.from(document.querySelectorAll(selector)).slice(0, 50).map((el) => ({
              tag: el.tagName,
              text: (el.innerText || "").slice(0, 200),
              href: el.href || null,
              value: el.value || null,
            })),
          }),
          args: [cmd.params.selector],
        });
        return result;
      }
      default:
        return { ok: false, error: "unknown method " + cmd.method };
    }
  } catch (e) {
    return { ok: false, error: String(e) };
  }
}
