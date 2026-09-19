const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const vm = require("node:vm");

const content = require("../content.js");
const { clickDom, hoverDom, typeDom } = content;
const { hideVirtualCursor, constants } = content.__vcuTest;

class FakeClassList {
  constructor() { this.values = new Set(); }
  add(value) { this.values.add(value); }
  remove(value) { this.values.delete(value); }
  contains(value) { return this.values.has(value); }
}

class FakeEvent {
  constructor(type, init = {}) {
    this.type = type;
    this.bubbles = !!init.bubbles;
    this.cancelable = !!init.cancelable;
    this.isTrusted = false;
  }
}

class FakeMouseEvent extends FakeEvent {
  constructor(type, init = {}) {
    super(type, init);
    this.clientX = init.clientX || 0;
    this.clientY = init.clientY || 0;
    this.button = init.button || 0;
    this.buttons = init.buttons || 0;
    this.detail = init.detail || 0;
  }
}

class FakeInputEvent extends FakeEvent {
  constructor(type, init = {}) {
    super(type, init);
    this.inputType = init.inputType || "";
    this.data = init.data ?? null;
  }
}

class FakeMutationObserver {
  static instances = [];

  constructor(callback) {
    this.callback = callback;
    this.records = [];
    this.disconnected = false;
    FakeMutationObserver.instances.push(this);
  }

  observe(target, options) {
    this.target = target;
    this.options = options;
  }

  takeRecords() {
    const records = this.records;
    this.records = [];
    return records;
  }

  disconnect() { this.disconnected = true; }
  emit(records) { this.callback(records); }
}

class FakeShadowRoot {
  constructor() {
    this.html = "";
    this.nodes = new Map();
    this.firstElementChild = null;
  }

  set innerHTML(value) {
    this.html = value;
    const root = new FakeElement("DIV", { left: 0, top: 0, width: 34, height: 40 });
    const halo = new FakeElement("DIV", { left: 0, top: 0, width: 28, height: 28 });
    const arrow = new FakeElement("SVG", { left: 0, top: 0, width: 30, height: 38 });
    root.classList = new FakeClassList();
    halo.classList = new FakeClassList();
    arrow.classList = new FakeClassList();
    root.style = {};
    halo.style = {};
    arrow.style = {};
    this.nodes.set(".vcu-cursor", root);
    this.nodes.set(".vcu-halo", halo);
    this.nodes.set(".vcu-arrow", arrow);
    this.firstElementChild = root;
  }

  querySelector(selector) { return this.nodes.get(selector) || null; }
}

class FakeElement {
  constructor(tagName, rect, options = {}) {
    this.tagName = tagName.toUpperCase();
    this.rect = { ...rect };
    this.attrs = new Map(Object.entries(options.attrs || {}));
    this.style = {
      display: "block",
      visibility: "visible",
      opacity: "1",
      pointerEvents: "auto",
      ...(options.style || {}),
    };
    this.hidden = !!options.hidden;
    this.disabled = !!options.disabled;
    this.inert = !!options.inert;
    this.readOnly = !!options.readOnly;
    this.type = options.type || "text";
    this.innerText = options.innerText || "";
    this.textContent = options.textContent || "";
    this.value = options.value || "";
    this.isContentEditable = !!options.isContentEditable;
    this.isConnected = true;
    this.focusCalls = 0;
    this.scrollCalls = 0;
    this.dispatches = [];
    this.listeners = new Map();
    this.onScroll = options.onScroll || null;
    this.children = [];
    this.parentNode = null;
    this.classList = new FakeClassList();
    this.offsetWidth = 0;
  }

  getAttribute(name) { return this.attrs.has(name) ? String(this.attrs.get(name)) : null; }
  hasAttribute(name) { return this.attrs.has(name); }
  setAttribute(name, value) { this.attrs.set(name, String(value)); }
  removeAttribute(name) { this.attrs.delete(name); }

  getBoundingClientRect() {
    return {
      ...this.rect,
      right: this.rect.right ?? this.rect.left + this.rect.width,
      bottom: this.rect.bottom ?? this.rect.top + this.rect.height,
    };
  }

  scrollIntoView() {
    this.scrollCalls += 1;
    if (this.onScroll) this.onScroll(this);
  }

  focus() { this.focusCalls += 1; }

  matches(selector) {
    if (selector === ":disabled") return this.disabled;
    return false;
  }

  closest(selector) {
    if (selector === "fieldset[disabled]" && this.disabledAncestor) return this.disabledAncestor;
    if (selector === "[inert]" && this.inertAncestor) return this.inertAncestor;
    return null;
  }

  contains(other) { return other === this || this.children.includes(other); }

  appendChild(child) {
    this.children.push(child);
    child.parentNode = this;
    child.isConnected = true;
    return child;
  }

  addEventListener(type, fn) {
    const list = this.listeners.get(type) || [];
    list.push(fn);
    this.listeners.set(type, list);
  }

  dispatchEvent(event) {
    this.dispatches.push(event);
    for (const fn of this.listeners.get(event.type) || []) fn(event);
    return true;
  }

  remove() { this.removed = true; this.isConnected = false; }
  attachShadow() { this.shadowRoot = new FakeShadowRoot(); return this.shadowRoot; }
}

class FakeDocument {
  constructor(selectorMap, hit) {
    this.selectorMap = selectorMap;
    this.hit = hit;
    this.title = "VCU test page";
    this.created = [];
    this.documentElement = new FakeElement("HTML", { left: 0, top: 0, width: 100, height: 100 });
    this.body = new FakeElement("BODY", { left: 0, top: 0, width: 100, height: 100 });
    this.documentElement.appendChild(this.body);
    this.listeners = new Map();
    this.interactiveNodes = [];
  }

  querySelectorAll(selector) {
    if (selector === "[invalid") throw new Error("SyntaxError");
    if (selector.includes("[contenteditable=true]") && selector.includes("[tabindex]")) return this.interactiveNodes;
    return this.selectorMap[selector] || [];
  }

  elementFromPoint() { return typeof this.hit === "function" ? this.hit() : this.hit; }

  createElement(tag) {
    const element = new FakeElement(tag, { left: 0, top: 0, width: 0, height: 0 });
    this.created.push(element);
    return element;
  }

  getElementById(id) {
    return this.created.find((element) => element.id === id && element.isConnected !== false) || null;
  }

  addEventListener(type, listener) {
    const list = this.listeners.get(type) || [];
    list.push(listener);
    this.listeners.set(type, list);
  }

  removeEventListener(type, listener) {
    this.listeners.set(type, (this.listeners.get(type) || []).filter((entry) => entry !== listener));
  }

  emit(type, target) {
    for (const listener of this.listeners.get(type) || []) listener({ type, target });
  }
}

function installPage(document, withMutationObserver = false) {
  global.document = document;
  global.window = {
    innerWidth: 100,
    innerHeight: 100,
    MouseEvent: FakeMouseEvent,
    getComputedStyle: (element) => element.style,
    scrollBy() {},
  };
  if (withMutationObserver) {
    FakeMutationObserver.instances = [];
    global.MutationObserver = FakeMutationObserver;
    global.window.MutationObserver = FakeMutationObserver;
  }
  global.getComputedStyle = (element) => element.style;
  global.MouseEvent = FakeMouseEvent;
  global.InputEvent = FakeInputEvent;
  global.Event = FakeEvent;
  global.location = { href: "https://example.test/" };
}

function resetPage() {
  hideVirtualCursor();
  delete global.document;
  delete global.window;
  delete global.getComputedStyle;
  delete global.MouseEvent;
  delete global.InputEvent;
  delete global.Event;
  delete global.MutationObserver;
  delete global.location;
}

test.afterEach(resetPage);

test("repeated content-script execution is versioned and listener-idempotent", () => {
  const source = fs.readFileSync(require.resolve("../content.js"), "utf8");
  const listeners = [];
  const context = {
    chrome: { runtime: { onMessage: { addListener: (fn) => listeners.push(fn) } } },
    document: {
      title: "repeat test",
      querySelectorAll: () => [],
      documentElement: {},
    },
    location: { href: "https://example.test/repeat" },
    console,
    setTimeout,
    clearTimeout,
  };
  context.globalThis = context;
  vm.runInNewContext(source, context, { filename: "content.js" });
  const firstDocumentId = context.__vcuContent.document_id;
  vm.runInNewContext(source, context, { filename: "content.js" });
  assert.equal(listeners.length, 1, "same-version reinjection must not add a second listener");
  assert.equal(context.__vcuContent.document_id, firstDocumentId, "same document keeps its identity");
  let response;
  listeners[0]({ type: "vcu_ping_page" }, null, (value) => { response = value; });
  assert.equal(response.version, "0.2.5");

  const legacyListeners = [];
  const legacyContext = {
    __vcuContent: true,
    chrome: { runtime: { onMessage: { addListener: (fn) => legacyListeners.push(fn) } } },
    document: { documentElement: {} },
    console,
    setTimeout,
    clearTimeout,
  };
  legacyContext.globalThis = legacyContext;
  vm.runInNewContext(source, legacyContext, { filename: "content.js" });
  assert.equal(legacyListeners.length, 0, "legacy pages must refresh instead of stacking listeners");
});

test("viewport metadata is stable across cursor cleanup and tracks page mutations", () => {
  const document = new FakeDocument({}, null);
  installPage(document, true);

  const first = content.viewportSnapshot();
  assert.equal(first.ok, true);
  assert.equal(first.source, "extension_dom");
  assert.match(first.viewport.document_id, /^doc-/);
  assert.equal(first.viewport.width, 100);
  assert.equal(first.viewport.height, 100);
  assert.equal(first.viewport.scroll_x, 0);
  assert.equal(first.viewport.scroll_y, 0);
  assert.equal(first.viewport.device_pixel_ratio, 1);
  assert.equal(first.viewport.visual_scale, 1);
  assert.equal(first.viewport.revision, 0);
  assert.equal(content.__vcuTest.state.last_viewport_stable, true);

  const same = content.viewportSnapshot();
  assert.deepEqual(same.viewport.layout_signature, first.viewport.layout_signature);

  const observer = FakeMutationObserver.instances[0];
  observer.records.push({
    type: "childList",
    target: document.body,
    addedNodes: [new FakeElement("DIV", { left: 0, top: 0, width: 1, height: 1 })],
    removedNodes: [],
  });
  const afterMutation = content.viewportSnapshot();
  assert.equal(afterMutation.viewport.document_id, first.viewport.document_id);
  assert.equal(afterMutation.viewport.revision, 1);

  global.window.scrollY = 14;
  const afterScroll = content.viewportSnapshot();
  assert.equal(afterScroll.viewport.scroll_y, 14);
  assert.equal(afterScroll.viewport.revision, 1, "scroll is represented separately from DOM revision");
});

test("viewport ignores mutations belonging only to the virtual cursor", () => {
  const button = new FakeElement("BUTTON", { left: 20, top: 20, width: 20, height: 20 });
  const document = new FakeDocument({}, button);
  installPage(document, true);
  const before = content.viewportSnapshot();
  const clicked = content.clickPoint(30, 30, before.viewport, false);
  assert.equal(clicked.ok, true);
  const host = document.getElementById(constants.VCU_CURSOR_ID);
  assert.ok(host);
  const observer = FakeMutationObserver.instances[0];
  observer.records.push({
    type: "childList",
    target: document.documentElement,
    addedNodes: [host],
    removedNodes: [],
  });
  observer.records.push({
    type: "childList",
    target: document.documentElement,
    addedNodes: [],
    removedNodes: [host],
  });
  const after = content.viewportSnapshot();
  assert.equal(after.viewport.revision, before.viewport.revision);
  assert.equal(content.__vcuTest.state.last_viewport_stable, true);
});

test("point click rejects stale document, scroll, and revision snapshots", () => {
  const button = new FakeElement("BUTTON", { left: 20, top: 20, width: 20, height: 20 });
  const document = new FakeDocument({}, button);
  installPage(document, true);
  const snapshot = content.viewportSnapshot().viewport;

  const wrongDocument = { ...snapshot, document_id: "doc-other" };
  assert.equal(content.clickPoint(30, 30, wrongDocument, false).error_code, "stale_viewport");

  global.window.scrollX = 4;
  assert.equal(content.clickPoint(30, 30, snapshot, false).error_code, "stale_viewport");
  global.window.scrollX = 0;

  const navigatedDocument = new FakeDocument({}, button);
  installPage(navigatedDocument, true);
  assert.equal(content.clickPoint(30, 30, snapshot, false).error_code, "stale_viewport");

  // Return to the original fake document for the revision assertion below.
  installPage(document, true);
  const freshSnapshot = content.viewportSnapshot().viewport;

  const observer = FakeMutationObserver.instances[0];
  observer.records.push({
    type: "attributes",
    target: document.body,
    addedNodes: [],
    removedNodes: [],
  });
  assert.equal(content.clickPoint(30, 30, freshSnapshot, false).error_code, "stale_viewport");
  assert.equal(content.__vcuTest.state.revision, 1);
});

test("layout signature rejects CSS-only button movement with no DOM mutation", () => {
  const button = new FakeElement("BUTTON", { left: 20, top: 20, width: 20, height: 20 });
  const document = new FakeDocument({}, button);
  document.interactiveNodes = [button];
  installPage(document, true);
  const snapshot = content.viewportSnapshot().viewport;
  assert.equal(snapshot.revision, 0);
  assert.equal(snapshot.layout_signature.nodes[0].left, 20);

  // Simulate a stylesheet/CSS animation move: rect changes, with no
  // attribute/childList MutationObserver record.
  button.rect.left = 48;
  const result = content.clickPoint(30, 30, snapshot, true);
  assert.equal(result.ok, false);
  assert.equal(result.error_code, "stale_viewport");
  assert.equal(result.viewport.revision, 0, "layout drift is carried by the signature, not a fake DOM revision");
  assert.equal(result.viewport.layout_signature.nodes[0].left, 48);
});

test("input and change events invalidate a prior point snapshot without reading values", () => {
  const input = new FakeElement("INPUT", { left: 20, top: 20, width: 30, height: 12 }, { value: "secret" });
  const document = new FakeDocument({}, input);
  document.interactiveNodes = [input];
  installPage(document, true);
  const snapshot = content.viewportSnapshot().viewport;
  const signatureText = JSON.stringify(snapshot.layout_signature);
  assert.doesNotMatch(signatureText, /secret/);

  input.value = "changed-without-attribute-mutation";
  document.emit("input", input);
  const result = content.clickPoint(30, 25, snapshot, true);
  assert.equal(result.ok, false);
  assert.equal(result.error_code, "stale_viewport");
  assert.equal(result.viewport.revision, 1);

  const fresh = content.viewportSnapshot().viewport;
  document.emit("change", input);
  assert.equal(content.clickPoint(30, 25, fresh, true).error_code, "stale_viewport");
  assert.equal(content.__vcuTest.state.revision, 2);
});

test("layout signature supports a 400-node page and explicitly blocks visible overflow", () => {
  const nodes = Array.from({ length: 400 }, (_, index) => new FakeElement("BUTTON", {
    left: index % 20,
    top: Math.floor(index / 20),
    width: 1,
    height: 1,
  }));
  const document = new FakeDocument({}, nodes[0]);
  document.interactiveNodes = nodes;
  installPage(document, true);
  const started = process.hrtime.bigint();
  const snapshot = content.viewportSnapshot().viewport;
  const elapsedMs = Number(process.hrtime.bigint() - started) / 1e6;
  assert.equal(snapshot.layout_signature.max_nodes, 1000);
  assert.equal(snapshot.layout_signature.visible_count, 400);
  assert.equal(snapshot.layout_signature.nodes.length, 400);
  assert.equal(snapshot.layout_signature.overflow, false);
  assert.ok(elapsedMs < 1000, `400-node signature should stay bounded (${elapsedMs.toFixed(1)}ms)`);
  assert.equal(content.clickPoint(0.5, 0.5, snapshot, true).ok, true);

  const overflowNodes = Array.from({ length: 1001 }, (_, index) => new FakeElement("BUTTON", {
    left: index % 20,
    top: Math.floor(index / 20),
    width: 1,
    height: 1,
  }));
  const overflowDocument = new FakeDocument({}, overflowNodes[0]);
  overflowDocument.interactiveNodes = overflowNodes;
  installPage(overflowDocument, true);
  const overflowSnapshot = content.viewportSnapshot().viewport;
  assert.equal(overflowSnapshot.layout_signature.overflow, true);
  assert.equal(overflowSnapshot.layout_signature.overflow_reason, "visible_node_cap");
  assert.equal(overflowSnapshot.layout_signature.nodes.length, 1000);
  assert.equal(content.clickPoint(0.5, 0.5, overflowSnapshot, true).error_code, "layout_signature_overflow");

  const scanOverflowNodes = Array.from({ length: 5001 }, () => new FakeElement("BUTTON", {
    left: 0, top: 0, width: 1, height: 1,
  }, { style: { display: "none" } }));
  const scanOverflowDocument = new FakeDocument({}, scanOverflowNodes[0]);
  scanOverflowDocument.interactiveNodes = scanOverflowNodes;
  installPage(scanOverflowDocument, true);
  const scanOverflowSnapshot = content.viewportSnapshot().viewport;
  assert.equal(scanOverflowSnapshot.layout_signature.overflow, true);
  assert.equal(scanOverflowSnapshot.layout_signature.overflow_reason, "scan_cap");
});

test("point click rejects invalid CSS coordinates and does not mutate on dry-run", () => {
  const button = new FakeElement("BUTTON", { left: 20, top: 20, width: 20, height: 20 });
  const document = new FakeDocument({}, button);
  installPage(document, true);
  const snapshot = content.viewportSnapshot().viewport;

  assert.equal(content.clickPoint(Number.NaN, 30, snapshot, false).error_code, "invalid_point");
  assert.equal(content.clickPoint(30, Number.POSITIVE_INFINITY, snapshot, false).error_code, "invalid_point");
  assert.equal(content.clickPoint(100, 30, snapshot, false).error_code, "invalid_point");

  const dryRun = content.clickPoint(30, 30, snapshot, true);
  assert.equal(dryRun.ok, true);
  assert.equal(dryRun.pressed, false);
  assert.equal(dryRun.dry_run, true);
  assert.equal(dryRun.input_path, "dom_point_click");
  assert.equal(dryRun.trusted, false);
  assert.equal(button.dispatches.length, 0);
  assert.equal(document.created.length, 0, "point dry-run must not create a cursor host");
});

test("point click preserves supplied hotspot, activates a button around a span, and reports event coordinates", () => {
  const button = new FakeElement("BUTTON", { left: 10, top: 10, width: 60, height: 30 }, { innerText: "Save" });
  const span = new FakeElement("SPAN", { left: 20, top: 15, width: 20, height: 12 });
  button.appendChild(span);
  span.parentElement = button;
  const document = new FakeDocument({}, span);
  installPage(document, true);
  const snapshot = content.viewportSnapshot().viewport;

  const result = content.clickPoint(25.5, 21.25, snapshot, false);
  assert.equal(result.ok, true);
  assert.equal(result.pressed, true);
  assert.equal(result.css_point.x, 25.5);
  assert.equal(result.css_point.y, 21.25);
  assert.equal(result.input_path, "dom_point_click");
  assert.equal(result.trusted, false);
  assert.equal(result.target_tag, "BUTTON");
  assert.deepEqual(result.rectangle, {
    x: 10, y: 10, left: 10, top: 10, width: 60, height: 30, right: 70, bottom: 40,
  });
  assert.equal(button.dispatches.length, 1);
  assert.equal(button.dispatches[0].clientX, 25.5);
  assert.equal(button.dispatches[0].clientY, 21.25);
  assert.equal(span.dispatches.length, 0);
});

test("point click refuses iframe and canvas targets that need trusted input", () => {
  const iframe = new FakeElement("IFRAME", { left: 5, top: 5, width: 40, height: 40 });
  const document = new FakeDocument({}, iframe);
  installPage(document, true);
  const snapshot = content.viewportSnapshot().viewport;
  const iframeResult = content.clickPoint(20, 20, snapshot, false);
  assert.equal(iframeResult.ok, false);
  assert.equal(iframeResult.error_code, "unsupported_point_target");
  assert.match(iframeResult.error, /trusted input/);

  const canvas = new FakeElement("CANVAS", { left: 5, top: 5, width: 40, height: 40 });
  document.hit = canvas;
  const canvasResult = content.clickPoint(20, 20, snapshot, false);
  assert.equal(canvasResult.ok, false);
  assert.equal(canvasResult.error_code, "unsupported_point_target");
});

test("click rejects ambiguous, hidden, disabled, inert, and occluded selectors", () => {
  const ambiguousA = new FakeElement("BUTTON", { left: 10, top: 10, width: 20, height: 20 });
  const ambiguousB = new FakeElement("BUTTON", { left: 40, top: 10, width: 20, height: 20 });
  const hidden = new FakeElement("BUTTON", { left: 10, top: 10, width: 20, height: 20 }, { style: { display: "none" } });
  const disabled = new FakeElement("BUTTON", { left: 10, top: 10, width: 20, height: 20 }, { disabled: true });
  const inert = new FakeElement("BUTTON", { left: 10, top: 10, width: 20, height: 20 }, { inert: true });
  const covered = new FakeElement("BUTTON", { left: 10, top: 10, width: 20, height: 20 });
  const cover = new FakeElement("DIV", { left: 10, top: 10, width: 20, height: 20 });
  const document = new FakeDocument({
    ".ambiguous": [ambiguousA, ambiguousB],
    "#hidden": [hidden],
    "#disabled": [disabled],
    "#inert": [inert],
    "#covered": [covered],
  }, cover);
  installPage(document);

  assert.match(clickDom(".ambiguous", false).error, /ambiguous/);
  assert.match(clickDom("#hidden", false).error, /hidden/);
  assert.match(clickDom("#disabled", false).error, /disabled/);
  assert.match(clickDom("#inert", false).error, /inert/);
  assert.match(clickDom("#covered", false).error, /occluded/);
  assert.equal(covered.dispatches.length, 0);
});

test("click dry-run reports geometry and scroll need without side effects", () => {
  const button = new FakeElement("BUTTON", { left: 20, top: 140, width: 20, height: 10 });
  const document = new FakeDocument({ "#offscreen": [button] }, button);
  installPage(document);

  const result = clickDom("#offscreen", true);
  assert.equal(result.ok, true);
  assert.equal(result.pressed, false);
  assert.equal(result.dry_run, true);
  assert.equal(result.needs_scroll, true);
  assert.equal(result.css_point.x, 30);
  assert.equal(result.css_point.y, 145);
  assert.equal(button.scrollCalls, 0);
  assert.equal(button.focusCalls, 0);
  assert.equal(button.dispatches.length, 0);
  assert.equal(document.created.length, 0, "dry-run must not create the pointer host");
});

test("click scrolls, remeasures, hit-tests, and reports an untrusted DOM click", async () => {
  const button = new FakeElement("BUTTON", { left: 20, top: 140, width: 20, height: 10 }, {
    innerText: "Continue",
    onScroll: (element) => { element.rect = { left: 20, top: 30, width: 20, height: 10 }; },
  });
  const document = new FakeDocument({ "#continue": [button] }, button);
  installPage(document);

  const result = clickDom("#continue", false);
  assert.equal(result.ok, true);
  assert.equal(result.pressed, true);
  assert.equal(result.source, "extension_dom");
  assert.equal(result.input_path, "dom_click");
  assert.equal(result.trusted, false);
  assert.equal(result.os_cursor_used, false);
  assert.deepEqual(result.css_point, { x: 30, y: 35 });
  assert.deepEqual(result.rectangle, {
    x: 20, y: 30, left: 20, top: 30, width: 20, height: 10, right: 40, bottom: 40,
  });
  assert.equal(button.scrollCalls, 1);
  assert.equal(button.dispatches.length, 1);
  assert.equal(button.dispatches[0].type, "click");
  assert.equal(button.dispatches[0].clientX, 30);
  assert.equal(button.dispatches[0].clientY, 35);
  const host = document.getElementById(constants.VCU_CURSOR_ID);
  assert.ok(host, "click should render the virtual pointer");
  assert.match(host.style.cssText, /pointer-events:none/);
  assert.match(host.shadowRoot.html, /vcu-arrow/);

  await new Promise((resolve) => setTimeout(resolve, constants.VCU_CURSOR_HIDE_MS + 30));
  assert.equal(document.getElementById(constants.VCU_CURSOR_ID), null, "cursor and timers should clean up");
});

test("hover uses the same isolated, pointer-transparent arrow and halo", () => {
  const button = new FakeElement("BUTTON", { left: 25, top: 35, width: 20, height: 10 });
  const document = new FakeDocument({ "#hover": [button] }, button);
  installPage(document);

  const result = hoverDom("#hover", false);
  assert.equal(result.ok, true);
  assert.equal(result.hovered, true);
  assert.equal(result.input_path, "dom_hover");
  assert.equal(button.dispatches.length, 3);
  assert.deepEqual([button.dispatches[0].clientX, button.dispatches[0].clientY], [35, 40]);
  const host = document.getElementById(constants.VCU_CURSOR_ID);
  assert.ok(host);
  assert.match(host.style.cssText, /pointer-events:none/);
  assert.match(host.shadowRoot.html, /fill: #555c65/);
  assert.match(host.shadowRoot.html, /vcu-halo/);
});

test("type requires its selector and never falls back to activeElement", () => {
  const active = new FakeElement("INPUT", { left: 10, top: 10, width: 20, height: 10 }, { value: "old" });
  const document = new FakeDocument({}, active);
  document.activeElement = active;
  installPage(document);
  const result = typeDom(undefined, "new value", false);
  assert.equal(result.ok, false);
  assert.match(result.error, /selector required/);
  assert.equal(active.value, "old");
  assert.equal(active.focusCalls, 0);
});

test("type only edits writable controls and uses the native value setter", () => {
  class NativeInput extends FakeElement {
    constructor(rect) {
      super("INPUT", rect);
      this._nativeValue = "";
      delete this.value;
    }
    get value() { return this._nativeValue; }
    set value(value) { this._nativeValue = String(value); }
  }
  class ReactInput extends NativeInput {
    constructor(rect) {
      super(rect);
      this.ownSetterCalls = 0;
      Object.defineProperty(this, "value", {
        configurable: true,
        get: () => this._nativeValue,
        set: (value) => { this.ownSetterCalls += 1; this._trackedValue = value; },
      });
    }
  }
  const input = new ReactInput({ left: 15, top: 20, width: 30, height: 12 });
  const readonly = new FakeElement("INPUT", { left: 15, top: 20, width: 30, height: 12 }, { readOnly: true });
  const disabled = new FakeElement("INPUT", { left: 15, top: 20, width: 30, height: 12 }, { disabled: true });
  const body = new FakeElement("BODY", { left: 0, top: 0, width: 100, height: 100 });
  const document = new FakeDocument({ "#input": [input], "#readonly": [readonly], "#disabled": [disabled], "body": [body] }, input);
  document.defaultView = {
    HTMLInputElement: NativeInput,
    HTMLTextAreaElement: FakeElement,
  };
  input.ownerDocument = document;
  document.body = body;
  installPage(document);

  assert.match(typeDom("#readonly", "x", false).error, /readonly/);
  assert.match(typeDom("#disabled", "x", false).error, /disabled/);
  assert.match(typeDom("body", "should not replace body", false).error, /editable/);

  const result = typeDom("#input", "hello", false);
  assert.equal(result.ok, true);
  assert.equal(result.typed, true);
  assert.equal(result.input_path, "dom_type");
  assert.equal(result.trusted, false);
  assert.equal(input.value, "hello");
  assert.equal(input.ownSetterCalls, 0, "framework own setter must not be used");
  assert.deepEqual(input.dispatches.map((event) => event.type), ["input", "change"]);
  assert.equal(input.focusCalls, 1);
  assert.equal(body.textContent, "");
});

test("cursor drawing compensates webpage zoom while preserving its target hotspot", () => {
  const button = new FakeElement("BUTTON", {left:20,top:30,width:20,height:10});
  const document = new FakeDocument({"#zoom-target":[button]},button);
  installPage(document,true);
  for (const [zoom,scale] of [[.8,1.25],[1,1],[2,.5]]) {
    content.__vcuTest.setCursorZoom(zoom);
    const result = hoverDom("#zoom-target",false);
    assert.deepEqual(result.css_point,{x:30,y:35});
    const root=document.getElementById(constants.VCU_CURSOR_ID).shadowRoot.querySelector('.vcu-cursor');
    assert.equal(root.style.left,'30px');
    assert.equal(root.style.top,'35px');
    assert.equal(root.style.transform,`scale(${scale})`);
  }
  content.__vcuTest.setCursorZoom(1);
});

test("CSS-only overlay and stacking changes invalidate a screenshot target", () => {
  const button = new FakeElement("BUTTON",{left:10,top:10,width:50,height:30});
  const overlay = new FakeElement("DIV",{left:10,top:10,width:50,height:30});
  const doc = new FakeDocument({"#button":[button]},button);
  doc.interactiveNodes=[button];
  installPage(doc,true);
  const snapshot=content.viewportSnapshot().viewport;
  doc.hit=overlay;
  const rejected=content.clickPoint(20,20,snapshot,false);
  assert.equal(rejected.ok,false);
  assert.equal(rejected.error_code,"stale_viewport");
  assert.equal(button.dispatches.length,0);
  assert.equal(overlay.dispatches.length,0);
});

test("stable layout remains valid after Rust JSON object-key ordering roundtrip", () => {
  const button=new FakeElement("BUTTON",{left:10,top:10,width:50,height:30});
  const doc=new FakeDocument({"#button":[button]},button);doc.interactiveNodes=[button];installPage(doc,true);
  const snapshot=content.viewportSnapshot().viewport;
  const sortKeys=value=>Array.isArray(value)?value.map(sortKeys):value && typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,sortKeys(value[key])])):value;
  const stored=JSON.parse(JSON.stringify(sortKeys(snapshot)));
  const checked=content.clickPoint(20,20,stored,true);
  assert.equal(checked.ok,true,checked.error);
  assert.equal(checked.pressed,false);
  const clicked=content.clickPoint(20,20,stored,false);
  assert.equal(clicked.ok,true,clicked.error);
  assert.equal(clicked.pressed,true);
  assert.equal(button.dispatches.length,1);
});
