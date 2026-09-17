const out = document.getElementById('out');
const endpointEl = document.getElementById('endpoint');
const tokenEl = document.getElementById('token');

chrome.runtime.sendMessage({ type: 'status' }, (st) => {
  if (st?.endpoint) endpointEl.value = st.endpoint;
});

document.getElementById('save').onclick = () => {
  chrome.runtime.sendMessage(
    {
      type: 'set_config',
      endpoint: endpointEl.value.trim(),
      token: tokenEl.value.trim(),
    },
    (res) => {
      chrome.runtime.sendMessage({ type: 'ensure_agent_window' }, (w) => {
        out.textContent = JSON.stringify({ config: res, window: w }, null, 2);
        out.className = w?.ok ? 'ok' : 'bad';
      });
    }
  );
};
