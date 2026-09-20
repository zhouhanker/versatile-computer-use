function holdKeepalive() {
  try {
    const port = chrome.runtime.connect({ name: "vcu-keepalive" });
    port.onDisconnect.addListener(() => setTimeout(holdKeepalive, 500));
  } catch (_) {
    setTimeout(holdKeepalive, 1000);
  }
}
holdKeepalive();
setInterval(() => {
  try {
    chrome.runtime.sendMessage({ type: "keepalive" }).catch(() => {});
  } catch (_) {}
}, 15000);
