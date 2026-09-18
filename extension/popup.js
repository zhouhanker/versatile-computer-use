async function refresh() {
  const st = await chrome.runtime.sendMessage({ type: "status" }).catch(() => null);
  const el = document.getElementById("status");
  if (!st || !st.ok) { el.className = "bad"; el.textContent = "extension SW not ready"; return; }
  el.className = st.hasToken ? "ok" : "bad";
  el.innerHTML = "endpoint: <code>" + st.endpoint + "</code><br/>token: " + (st.hasToken ? "yes" : "no") +
    "<br/>polling: " + st.polling + "<br/>version: " + ((chrome.runtime.getManifest() || {}).version || "?") +
    "<br/>agentWindow: " + (st.agentWindowId || "-") +
    (st.lastError ? "<br/>err: " + st.lastError : "");
}

async function maybeReloadStaleSw() {
  const flag = "vcu_sw_reload_0_1_5";
  const stored = await chrome.storage.local.get([flag]);
  if (stored[flag]) return;
  await chrome.storage.local.set({ [flag]: true });
  chrome.runtime.reload();
}

document.getElementById("boot").onclick = async () => {
  try { await chrome.runtime.sendMessage({ type: "rebootstrap" }); } catch (_) {}
  await refresh();
};
document.getElementById("agent").onclick = async () => {
  try { await chrome.runtime.sendMessage({ type: "ensure_agent_window" }); } catch (_) {}
  await refresh();
};
document.getElementById("reloadsw").onclick = () => chrome.runtime.reload();
void maybeReloadStaleSw();
void refresh();
