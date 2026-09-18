setInterval(() => {
  chrome.runtime.sendMessage({ type: "keepalive" }).catch(() => {});
}, 20000);
