// Text is inserted with textContent: tab titles and URLs are untrusted page data.
const selected = new Set();
let busy = false;
let visibleTabs = [];
const colors = { grey: '#85858c', blue: '#4c8bf5', red: '#e65a55', yellow: '#d6a62b', green: '#42a86a', pink: '#d65b9a', purple: '#b243bd', cyan: '#23a3b5', orange: '#e28c32' };
const byId = id => document.getElementById(id);
function notice(text, error = false) { byId('notice').textContent = text; byId('notice').classList.toggle('error', error); }
async function command(method, params = {}) {
  const result = await chrome.runtime.sendMessage({ type: 'browser_command', command: { method, params } });
  if (!result?.ok) throw new Error(result?.error || '无法连接浏览器，请重载扩展后重试。');
  return result;
}
function controls() {
  byId('create-group').disabled = busy || !selected.size;
  byId('ungroup').disabled = busy || !selected.size;
  const first = visibleTabs.find(t => selected.has(t.tab_id));
  for (const box of document.querySelectorAll('input[data-tab-id]')) {
    const tab = visibleTabs.find(t => t.tab_id === box.dataset.tabId);
    box.disabled = busy || !tab || !!tab.pinned || tab.controllable === false || !!(first && tab.window_id !== first.window_id);
  }
  byId('selection-label').textContent = selected.size ? `已选 ${selected.size} 个网页 · 同一窗口` : '将同一窗口中的网页整理为任务标签组';
}
async function act(method, params) {
  if (busy) return;
  busy = true; controls();
  try { await command(method, params); selected.clear(); await refresh(); }
  catch (error) { notice(error.message, true); }
  finally { busy = false; controls(); }
}
function tabRow(tab) {
  const row = document.createElement('div'); row.className = 'row' + (tab.focused ? ' active' : '');
  const box = document.createElement('input'); box.type = 'checkbox'; box.checked = selected.has(tab.tab_id);
  box.dataset.tabId = tab.tab_id;
  box.disabled = !!tab.pinned || tab.controllable === false; box.setAttribute('aria-label', '选择 ' + (tab.title || tab.url));
  box.onchange = () => { if (box.checked) selected.add(tab.tab_id); else selected.delete(tab.tab_id); controls(); };
  const button = document.createElement('button'); button.className = 'tab'; button.title = tab.url;
  const title = document.createElement('span'); title.className = 'title'; title.textContent = (tab.focused ? '● ' : '') + (tab.title || tab.url);
  const host = document.createElement('span'); host.className = 'host';
  try { host.textContent = new URL(tab.url).hostname + (tab.pinned ? ' · 已固定' : ''); } catch { host.textContent = tab.url; }
  button.disabled = tab.controllable === false;
  button.append(title, host); button.onclick = () => act('select_tab', { tab_id: tab.tab_id });
  row.append(box, button); return row;
}
async function refresh() {
  try {
    const data = await command('list_tabs');
    const tabs = (data.tabs || []).filter(t => /^https?:\/\//.test(t.url) && !t.agent_owned);
    visibleTabs = tabs;
    const live = new Set(tabs.filter(t => !t.pinned && t.controllable !== false).map(t => t.tab_id));
    for (const id of selected) if (!live.has(id)) selected.delete(id);
    const root = byId('tabs'); root.replaceChildren();
    const groups = data.groups || [];
    const firstSelected = tabs.find(t => selected.has(t.tab_id));
    if (firstSelected) for (const id of selected) {
      if (!tabs.some(t => t.tab_id === id && t.window_id === firstSelected.window_id)) selected.delete(id);
    }
    const windows = [...new Set(tabs.map(t => t.window_id))].sort((a,b) => Number(tabs.some(t=>t.window_id===b && t.focused))-Number(tabs.some(t=>t.window_id===a && t.focused)));
    for (const [index, windowId] of windows.entries()) {
      const windowTabs = tabs.filter(t => t.window_id === windowId);
      const windowSection = document.createElement('section');
      if (windows.length > 1) {
        const heading = document.createElement('h2'); heading.className = 'window-heading';
        const active = windowTabs.find(t => t.active);
        heading.textContent = windowTabs.some(t => t.focused) ? '当前窗口' : `窗口 ${index + 1}${active?.title ? ' · ' + active.title : ''}`;
        windowSection.append(heading);
      }
      const windowGroups = groups.filter(g => windowTabs.some(t => t.group_id === g.group_id));
      for (const group of windowGroups) {
        const members = windowTabs.filter(t => t.group_id === group.group_id);
        const section = document.createElement('section'); section.className = 'group'; section.style.setProperty('--group-color', colors[group.color] || colors.grey);
        const head = document.createElement('div'); head.className = 'group-head';
        const name = document.createElement('span'); name.className = 'group-name'; name.textContent = (group.title || '未命名标签组') + ' · ' + members.length;
        const toggle = document.createElement('button'); toggle.textContent = group.collapsed ? '展开' : '折叠'; toggle.setAttribute('aria-expanded', String(!group.collapsed));
        toggle.onclick = () => act('update_group', { group_id: group.group_id, collapsed: !group.collapsed });
        head.append(name, toggle); section.append(head);
        // Keep members discoverable even when their native group is collapsed.
        for (const tab of members) section.append(tabRow(tab));
        windowSection.append(section);
      }
      const known = new Set(windowGroups.map(g => g.group_id));
      for (const tab of windowTabs.filter(t => !known.has(t.group_id))) windowSection.append(tabRow(tab));
      root.append(windowSection);
    }
    notice(tabs.length ? `${tabs.length} 个网页 · 点击名称切换，勾选后分组` : '打开一个网页后，即可在这里选择和分组。');
    controls();
  } catch (error) { notice(error.message, true); }
  const status = await chrome.runtime.sendMessage({ type: 'status' }).catch(() => null);
  byId('status').textContent = `版本 ${chrome.runtime.getManifest().version} · ${status?.connected ? '已连接' : '未连接'}${status?.lastError ? '\n' + status.lastError : ''}`;
}
byId('refresh').onclick = refresh;
byId('group-form').onsubmit = event => {
  event.preventDefault();
  const title = byId('group-title').value.trim();
  if (title && selected.size) void act('group_tabs', { tab_ids: [...selected], title, color: 'purple' });
};
byId('ungroup').onclick = () => act('ungroup_tabs', { tab_ids: [...selected] });
byId('boot').onclick = async () => { await chrome.runtime.sendMessage({ type: 'rebootstrap' }); await refresh(); };
byId('reloadsw').onclick = () => chrome.runtime.reload();
void refresh();
