const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "..", "background.js"), "utf8");

function makeHarness(options = {}) {
  const state = {
    tabs: (options.tabs || []).map((tab) => Object.assign({
      status: "complete",
      pinned: false,
      groupId: -1,
      active: false,
    }, tab)),
    groups: (options.groups || []).map((group) => Object.assign({
      title: "",
      color: "purple",
      collapsed: false,
    }, group)),
    focusedWindowId: options.focusedWindowId == null
      ? ((options.tabs || [])[0] && (options.tabs || [])[0].windowId) || 1
      : options.focusedWindowId,
    calls: [],
    listeners: [],
    nextTabId: options.nextTabId || 100,
    nextGroupId: options.nextGroupId || 10,
    storage: Object.assign({}, options.storage),
    sendMessage: options.sendMessage || (() => Promise.resolve({ ok: true, pong: true, version: "0.2.5" })),
    executeScript: options.executeScript || (() => Promise.resolve([{}])),
  };
  if (!state.tabs.some((tab) => tab.active && tab.windowId === state.focusedWindowId)) {
    const first = state.tabs.find((tab) => tab.windowId === state.focusedWindowId);
    if (first) first.active = true;
  }
  const clone = (value) => JSON.parse(JSON.stringify(value));
  const focusedWindow = () => state.focusedWindowId;
  const queryTabs = (query = {}) => {
    let tabs = state.tabs;
    if (query.active) tabs = tabs.filter((tab) => tab.active);
    if (query.lastFocusedWindow) tabs = tabs.filter((tab) => tab.windowId === focusedWindow());
    if (query.windowId != null) tabs = tabs.filter((tab) => tab.windowId === query.windowId);
    return clone(tabs);
  };
  const getTab = (id) => {
    const tab = state.tabs.find((item) => item.id === Number(id));
    if (!tab) throw new Error("tab not found " + id);
    return clone(tab);
  };
  const groupById = (id) => {
    const group = state.groups.find((item) => item.id === Number(id));
    if (!group) throw new Error("group not found " + id);
    return group;
  };
  const chrome = {
    runtime: {
      id: "vcu-extension-id",
      onInstalled: { addListener() {} },
      onStartup: { addListener() {} },
      onMessage: { addListener: (listener) => state.listeners.push(listener) },
      getManifest: () => ({ version: "0.2.5" }),
      getURL: (file) => "chrome-extension://vcu-extension-id/" + file,
      getContexts: async () => [],
      reload() {},
    },
    alarms: { create() {}, onAlarm: { addListener() {} } },
    offscreen: { createDocument: async () => {} },
    storage: {
      local: {
        get: async (keys) => {
          if (!keys) return clone(state.storage);
          const result = {};
          for (const key of keys) if (state.storage[key] !== undefined) result[key] = state.storage[key];
          return result;
        },
        set: async (values) => Object.assign(state.storage, values),
      },
    },
    tabs: {
      query: async (query) => queryTabs(query),
      get: async (id) => getTab(id),
      getZoom: async () => options.zoom || 1,
      remove: async (id) => { getTab(id); state.calls.push({method:'tabs.remove',id}); state.tabs=state.tabs.filter(t=>t.id!==id); },
      sendMessage: async (id, payload) => {
        state.calls.push({ method: "sendMessage", id, payload: clone(payload) });
        return state.sendMessage(id, payload, state);
      },
      create: async (info) => {
        const id = state.nextTabId++;
        const windowId = info.windowId == null ? focusedWindow() : info.windowId;
        const tab = {
          id,
          windowId,
          title: "",
          url: info.url,
          status: "loading",
          pinned: false,
          groupId: -1,
          active: info.active !== false,
        };
        if (tab.active) state.tabs.filter((item) => item.windowId === windowId).forEach((item) => { item.active = false; });
        state.tabs.push(tab);
        return clone(tab);
      },
      update: async (id, changes) => {
        const tab = state.tabs.find((item) => item.id === Number(id));
        if (!tab) throw new Error("tab not found " + id);
        Object.assign(tab, changes);
        if (changes.active) state.tabs.filter((item) => item.windowId === tab.windowId && item.id !== tab.id).forEach((item) => { item.active = false; });
        return clone(tab);
      },
      group: async ({ tabIds, groupId, createProperties }) => {
        let target = groupId;
        if (target == null) {
          target = state.nextGroupId++;
          // Chromium defaults a new group to the current window, and moves
          // supplied tabs there. Do not fake the safer tab-origin default.
          const windowId = createProperties?.windowId ?? state.focusedWindowId;
          state.groups.push({ id: target, windowId, title: "", color: "purple", collapsed: false });
        } else {
          groupById(target);
        }
        for (const id of tabIds) {
          const tab = state.tabs.find((tab) => tab.id === Number(id));
          tab.groupId = target;
          tab.windowId = groupById(target).windowId;
        }
        return target;
      },
      ungroup: async (tabIds) => {
        for (const id of tabIds) {
          const tab = state.tabs.find((item) => item.id === Number(id));
          if (tab) tab.groupId = -1;
        }
        state.groups = state.groups.filter((group) => state.tabs.some((tab) => tab.groupId === group.id));
      },
      captureVisibleTab: async (id) => { state.calls.push({method:"captureVisibleTab", id, at:Date.now()}); return "data:image/png;base64,AA=="; },
    },
    tabGroups: {
      query: async () => clone(state.groups),
      update: async (id, changes) => {
        const group = groupById(id);
        Object.assign(group, changes);
        return clone(group);
      },
    },
    windows: {
      get: async (id) => ({ id }),
      create: async (info) => {
        const id = info.id || 999;
        state.calls.push({method:'windows.create',info:clone(info)});
        const tab = {id:state.nextTabId++,windowId:id,url:info.url,active:true,pinned:false,groupId:-1,status:'complete'};
        state.tabs.push(tab);
        if (info.focused !== false) state.focusedWindowId = id;
        return {id,tabs:[clone(tab)]};
      },
      update: async (id, changes) => {
        state.calls.push({ method: "windows.update", id, changes: clone(changes) });
        if (changes.focused) state.focusedWindowId = id;
        return { id, focused: !!changes.focused };
      },
    },
    scripting: {
      executeScript: async (details) => {
        state.calls.push({ method: "executeScript", details });
        return state.executeScript(details, state);
      },
    },
  };

  const context = {
    chrome,
    console,
    URL,
    AbortController,
    Date,
    Map,
    Set,
    Promise,
    JSON,
    Math,
    String,
    Number,
    Object,
    Array,
    RegExp,
    Error,
    globalThis: null,
    fetch: async () => ({ ok: false, json: async () => ({}) }),
    setTimeout: (fn, ms) => (ms > 1000 ? 0 : setTimeout(fn, ms)),
    clearTimeout,
  };
  context.globalThis = context;
  vm.runInNewContext(source + "\n;globalThis.__vcuTest = { handleCommand, resolveHttpTab, clickViaContent, typeViaContent, openTab, listTabsState };", context, { filename: "background.js" });
  return { state, chrome, api: context.__vcuTest };
}

test("explicit restricted tabs fail and implicit resolution stays on last-focused USER tab", async () => {
  const { api } = makeHarness({
    focusedWindowId: 1,
    tabs: [
      { id: 1, windowId: 1, url: "https://user.example/", active: true },
      { id: 2, windowId: 2, url: "https://etherscan.io/", active: true },
      { id: 3, windowId: 1, url: "https://agent.example/", active: false },
      { id: 4, windowId: 1, url: "chrome://settings/", active: false },
    ],
  });
  assert.equal(await api.resolveHttpTab(), 1);
  assert.equal(await api.resolveHttpTab(""), 0);
  assert.equal(await api.resolveHttpTab("4"), 0);
  assert.equal(await api.resolveHttpTab("999"), 0);
  assert.equal(await api.resolveHttpTab("3"), 3);
});

test("list tabs exposes native group metadata and select_tab expands and focuses it", async () => {
  const { api, state } = makeHarness({
    focusedWindowId: 1,
    groups: [{ id: 7, windowId: 2, title: "Research", color: "blue", collapsed: true }],
    tabs: [
      { id: 10, windowId: 1, url: "https://one.example/", active: true },
      { id: 11, windowId: 2, url: "https://two.example/", active: false, groupId: 7 },
    ],
  });
  const listed = await api.handleCommand({ method: "list_tabs", params: {} });
  assert.equal(listed.ok, true);
  assert.deepEqual(JSON.parse(JSON.stringify(listed.groups)), [{ group_id: "7", window_id: "2", title: "Research", color: "blue", collapsed: true }]);
  assert.equal(listed.tabs.find((tab) => tab.tab_id === "11").group_id, "7");
  const selected = await api.handleCommand({ method: "select_tab", params: { tab_id: "11" } });
  assert.equal(selected.ok, true);
  assert.equal(selected.tab_id, "11");
  assert.equal(selected.focused, true);
  assert.equal(selected.group.collapsed, false);
  assert.equal(state.groups.find((group) => group.id === 7).collapsed, false);
  assert.equal(state.focusedWindowId, 2);
});

test("select_tab rejects restricted browser pages", async () => {
  const { api } = makeHarness({ tabs: [{ id: 1, windowId: 1, url: "chrome://settings/", active: true }] });
  const result = await api.handleCommand({ method: "select_tab", params: { tab_id: "1" } });
  assert.equal(result.ok, false);
  assert.match(result.error, /unrestricted http/);
});

test("group_tabs rejects cross-window, restricted, and pinned inputs", async () => {
  const { api } = makeHarness({
    tabs: [
      { id: 1, windowId: 1, url: "https://one.example/" },
      { id: 2, windowId: 2, url: "https://two.example/" },
      { id: 3, windowId: 1, url: "chrome://settings/" },
      { id: 4, windowId: 1, url: "https://pinned.example/", pinned: true },
    ],
  });
  const crossWindow = await api.handleCommand({ method: "group_tabs", params: { tab_ids: ["1", "2"], title: "Bad" } });
  const restricted = await api.handleCommand({ method: "group_tabs", params: { tab_ids: ["1", "3"], title: "Bad" } });
  const pinned = await api.handleCommand({ method: "group_tabs", params: { tab_ids: ["1", "4"], title: "Bad" } });
  assert.match(crossWindow.error, /same window/);
  assert.match(restricted.error, /http\(s\)/);
  assert.match(pinned.error, /pinned/);
});

test("native group commands create, update, join, and ungroup tabs", async () => {
  const { api, state } = makeHarness({
    tabs: [
      { id: 1, windowId: 1, url: "https://one.example/", active: true },
      { id: 2, windowId: 1, url: "https://two.example/", active: false },
    ],
  });
  const grouped = await api.handleCommand({ method: "group_tabs", params: { tab_ids: ["1", "2"], title: "Work", color: "red", collapsed: true } });
  assert.equal(grouped.ok, true);
  assert.equal(grouped.group.title, "Work");
  assert.equal(grouped.group.color, "red");
  assert.equal(grouped.group.collapsed, true);
  const updated = await api.handleCommand({ method: "update_group", params: { group_id: grouped.group.group_id, collapsed: false } });
  assert.equal(updated.group.collapsed, false);
  const joined = await api.handleCommand({ method: "open_tab", params: { url: "https://three.example/", group_id: grouped.group.group_id, active: false } });
  assert.equal(joined.group.group_id, grouped.group.group_id);
  const named = await api.handleCommand({ method: "open_tab", params: { url: "https://four.example/", session_name: "Review" } });
  assert.equal(named.group.title, "Review");
  const conflict = await api.handleCommand({ method: "open_tab", params: { url: "https://bad.example/", session_name: "A", group_id: grouped.group.group_id } });
  assert.equal(conflict.ok, false);
  const ungrouped = await api.handleCommand({ method: "ungroup_tabs", params: { tab_ids: ["1", "2"] } });
  assert.equal(ungrouped.ok, true);
  assert.equal(state.tabs.find((tab) => tab.id === 1).groupId, -1);
});

test("mutation action does not replay an existing ok:false response", async () => {
  let actionCalls = 0;
  const { api, state } = makeHarness({
    tabs: [{ id: 1, windowId: 1, url: "https://user.example/", active: true }],
    sendMessage: async (_id, payload) => {
      if (payload.type === "vcu_ping_page") return { ok: true, pong: true, version: "0.2.5" };
      if (payload.type === "vcu_click") {
        actionCalls += 1;
        return { ok: false, error: "page rejected click", detail: "preserve me" };
      }
      return { ok: false, error: "unexpected" };
    },
  });
  const result = await api.clickViaContent(1, "button", false);
  assert.deepEqual(result, { ok: false, error: "page rejected click", detail: "preserve me" });
  assert.equal(actionCalls, 1);
  assert.equal(state.calls.filter((call) => call.method === "executeScript").length, 0);
});

test("old content lens is reported stale without stacking an injected listener", async () => {
  const { api, state } = makeHarness({
    tabs: [{ id: 1, windowId: 1, url: "https://user.example/", active: true }],
    sendMessage: async (_id, payload) => payload.type === "vcu_ping_page"
      ? { ok: true, pong: true }
      : { ok: false, error: "unexpected action" },
  });
  const result = await api.clickViaContent(1, "button", false);
  assert.deepEqual(JSON.parse(JSON.stringify(result)), { ok: false, error: "content lens is stale; reload this page" });
  assert.equal(state.calls.filter((call) => call.method === "executeScript").length, 0);
});

test("send timeout injects then sends the mutation once", async () => {
  let actionCalls = 0;
  let pingCalls = 0;
  const { api, state } = makeHarness({
    tabs: [{ id: 1, windowId: 1, url: "https://user.example/", active: true }],
    sendMessage: async (_id, payload) => {
      if (payload.type === "vcu_ping_page") return ++pingCalls > 1 ? { ok: true, pong: true, version: "0.2.5" } : null;
      if (payload.type === "vcu_type") {
        actionCalls += 1;
        return { ok: true, typed: true };
      }
      return null;
    },
    executeScript: async () => [{}],
  });
  const result = await api.typeViaContent(1, "input", "x", false).catch((error) => ({ error: String(error) }));
  // The harness returns no tab only for this focused test, so assert the
  // retry invariant through the direct content helper after tab setup below.
  assert.equal(result.error, undefined);
  assert.equal(actionCalls, 1);
  assert.equal(state.calls.filter((call) => call.method === "executeScript").length, 1);
});

test("scroll preserves an explicit zero delta", async () => {
  const { api, state } = makeHarness({ tabs: [{ id: 1, windowId: 1, url: "https://user.example/", active: true }] });
  const result = await api.handleCommand({ method: "scroll", params: { tab_id: "1", dy: 0, dry_run: true } });
  assert.equal(result.ok, true);
  const scroll = state.calls.find((call) => call.method === "sendMessage" && call.payload.type === "vcu_scroll");
  assert.equal(scroll.payload.dy, 0);
});

test("service-worker recovery restores the persisted agent window", async () => {
  const { api } = makeHarness({ storage: { agentWindowId: 77 } });
  const result = await api.handleCommand({ method: "ensure_agent_window", params: {} });
  assert.deepEqual(JSON.parse(JSON.stringify(result)), { ok: true, window_id: "77" });
});

test("popup browser_command is sender- and method-gated", async () => {
  const harness = makeHarness({ tabs: [{ id: 1, windowId: 1, url: "https://user.example/", active: true }] });
  assert.equal(harness.state.listeners.length, 1);
  const listener = harness.state.listeners[0];
  const invoke = (message, sender) => new Promise((resolve) => {
    listener(message, sender, resolve);
  });
  const rejectedSender = await invoke(
    { type: "browser_command", command: { method: "list_tabs", params: {} } },
    { id: "other", url: "chrome-extension://vcu-extension-id/popup.html" },
  );
  assert.equal(rejectedSender.ok, false);
  const rejectedMethod = await invoke(
    { type: "browser_command", command: { method: "open_tab", params: { url: "https://x.example/" } } },
    { id: "vcu-extension-id", url: "chrome-extension://vcu-extension-id/popup.html" },
  );
  assert.equal(rejectedMethod.ok, false);
  const listed = await invoke(
    { type: "browser_command", command: { method: "list_tabs", params: {} } },
    { id: "vcu-extension-id", url: "chrome-extension://vcu-extension-id/popup.html" },
  );
  assert.equal(listed.ok, true);
});

test("viewport screenshot binds the active tab and rejects changes during capture", async () => {
  const viewport = {document_id:"doc-1",url:"https://example.com/",width:100,height:80,revision:1};
  let calls = 0;
  const h = makeHarness({tabs:[{id:1,windowId:4,url:viewport.url,active:true}], sendMessage: async (_id,msg) => {
    if (msg.type === "vcu_ping_page") return {ok:true,version:"0.2.5"};
    if (msg.type === "vcu_viewport") { calls++; return {ok:true,viewport:{...viewport,revision:calls <= 2 ? 1 : calls}}; }
    return {ok:true,pressed:true};
  }});
  const shot = await h.api.handleCommand({method:"capture_tab",params:{tab_id:"1"}});
  assert.equal(shot.ok,true);
  assert.equal(shot.tab_id,"1");
  assert.equal(shot.viewport.document_id,"doc-1");
  const changed = await h.api.handleCommand({method:"capture_tab",params:{tab_id:"1"}});
  assert.equal(changed.ok,false);
  assert.match(changed.error,/changed/);
  const point = await h.api.handleCommand({method:"click_point",params:{tab_id:"1",x:25,y:30,expected_viewport:viewport}});
  assert.equal(point.ok,true);
  const msg = h.state.calls.filter(c=>c.method === "sendMessage").at(-1).payload;
  assert.equal(msg.type,"vcu_click_point");
  assert.equal(msg.x,25);
  assert.equal(msg.expected_viewport.document_id,"doc-1");
});

test("separate USER window can open in background without moving browser focus", async () => {
  const h = makeHarness({tabs:[{id:1,windowId:4,url:"https://example.com/",active:true}],focusedWindowId:4});
  const opened = await h.api.handleCommand({method:"open_tab",params:{url:"https://example.org/",new_window:true,active:false,session_name:"POC"}});
  assert.equal(opened.ok,true);
  assert.equal(opened.focused,false);
  assert.equal(h.state.focusedWindowId,4);
  assert.equal(opened.tabs[0].agent_owned,false);
  assert.equal(opened.group.window_id,"999");
  const count = h.state.calls.length;
  const conflict = await h.api.handleCommand({method:"open_tab",params:{url:"https://example.org/",new_window:true,group_id:opened.group.group_id}});
  assert.equal(conflict.ok,false);
  assert.equal(h.state.calls.length,count,"invalid option combination must not create a window");
});

test("close_tab removes only an explicit valid target and never defaults to the focused page", async () => {
  const h = makeHarness({tabs:[{id:1,windowId:4,url:"https://example.com/",active:true},{id:2,windowId:4,url:"https://example.org/"}]});
  for (const params of [{},{tab_id:""},{tab_id:"999"}]) {
    assert.equal((await h.api.handleCommand({method:"close_tab",params})).ok,false);
  }
  assert.equal(h.state.tabs.length,2);
  const closed = await h.api.handleCommand({method:"close_tab",params:{tab_id:"2"}});
  assert.equal(closed.closed,true);
  assert.equal(closed.tab_id,"2");
  assert.deepEqual(h.state.tabs.map(t=>t.id),[1]);
});

test("drawing zoom travels with the exact tab without changing action coordinates", async () => {
  const h = makeHarness({zoom:.8,tabs:[{id:1,windowId:4,url:"https://example.com/",active:true}]});
  await h.api.handleCommand({method:"click_point",params:{tab_id:"1",x:20,y:30,expected_viewport:{}}});
  const sent = h.state.calls.filter(c=>c.method === "sendMessage").at(-1).payload;
  assert.equal(sent.cursor_zoom,.8);
  assert.equal(sent.x,20);
  assert.equal(sent.y,30);
});

test("screenshot bursts respect the browser capture quota", async () => {
  const viewport={document_id:"doc-rate",url:"https://example.com/",width:100,height:80,revision:0};
  const h=makeHarness({tabs:[{id:1,windowId:4,url:viewport.url,active:true}],sendMessage:async (_id,msg)=>msg.type==='vcu_ping_page'?{ok:true,version:'0.2.5'}:{ok:true,viewport}});
  for(let i=0;i<3;i++) {
    const result=await h.api.handleCommand({method:'capture_tab',params:{tab_id:'1'}});
    assert.equal(result.ok,true,result.error);
  }
  const shots=h.state.calls.filter(c=>c.method==='captureVisibleTab');
  assert.equal(shots.length,3);
  assert.ok(shots[1].at-shots[0].at>=500);
  assert.ok(shots[2].at-shots[1].at>=500);
});
