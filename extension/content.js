// VCU content lens — DOM extract without hanging the service worker.
if (globalThis.__vcuContent) {
  // already installed
} else {
  globalThis.__vcuContent = true;
  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    if (!msg || (msg.type !== "vcu_extract" && msg.type !== "vcu_ping_page" && msg.type !== "vcu_type" && msg.type !== "vcu_click" && msg.type !== "vcu_scroll")) return;
    try {
      if (msg.type === "vcu_ping_page") sendResponse(pingPage());
      else if (msg.type === "vcu_type") sendResponse(typeDom(msg.selector, msg.text, msg.dry_run));
      else if (msg.type === "vcu_click") sendResponse(clickDom(msg.selector, msg.dry_run));
      else if (msg.type === "vcu_scroll") sendResponse(scrollDom(msg.dy, msg.dry_run));
      else sendResponse(extractDom(msg.selector || "a"));
    } catch (e) {
      sendResponse({ ok: false, error: String(e) });
    }
    return true;
  });
}

function pingPage() {
  return {
    ok: true,
    pong: true,
    url: location.href,
    title: document.title || "",
    source: "extension_dom",
  };
}

function extractDom(sel) {
  const nl = document.querySelectorAll(sel);
  const matches = [];
  const limit = 200;
  for (let i = 0; i < nl.length && matches.length < limit; i++) {
    const el = nl[i];
    matches.push({
      tag: el.tagName,
      text: (el.innerText || "").slice(0, 160),
      href: el.href || null,
      value: el.value || null,
    });
  }
  return {
    ok: true,
    matches,
    count: matches.length,
    truncated: nl.length > matches.length,
    url: location.href,
    title: document.title || "",
    source: "extension_dom",
  };
}

function typeDom(selector, text, dryRun) {
  let el = null;
  if (selector) {
    try { el = document.querySelector(selector); } catch (e) { return { ok: false, error: String(e) }; }
  }
  if (!el) el = document.activeElement;
  if (!el) return { ok: false, error: "no element" };
  if (dryRun) {
    return { ok: true, typed: false, dry_run: true, tag: el.tagName, source: "extension_dom" };
  }
  el.focus();
  const value = text == null ? "" : String(text);
  if ("value" in el) {
    el.value = value;
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
  } else {
    el.textContent = value;
  }
  return { ok: true, typed: true, text: value.slice(0, 80), tag: el.tagName, source: "extension_dom", os_cursor_used: false };
}

function clickDom(selector, dryRun) {
  let el = null;
  if (selector) {
    try { el = document.querySelector(selector); } catch (e) { return { ok: false, error: String(e) }; }
  }
  if (!el) return { ok: false, error: "not found" };
  const r = el.getBoundingClientRect();
  if (dryRun) {
    return { ok: true, pressed: false, dry_run: true, tag: el.tagName, text: (el.innerText || "").slice(0, 80), source: "extension_dom", os_cursor_used: false };
  }
  el.scrollIntoView({ block: "center", inline: "nearest" });
  el.click();
  return { ok: true, pressed: true, tag: el.tagName, text: (el.innerText || "").slice(0, 80), source: "extension_dom", os_cursor_used: false };
}

function scrollDom(dy, dryRun) {
  const y = Number(dy) || 0;
  if (dryRun) {
    return { ok: true, scrolled: false, dry_run: true, dy: y, source: "extension_dom", os_cursor_used: false };
  }
  window.scrollBy(0, y);
  return { ok: true, scrolled: true, dy: y, source: "extension_dom", os_cursor_used: false };
}
