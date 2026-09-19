// VCU content lens. DOM actions are deliberately synthetic: this file never
// moves the operating-system cursor and never claims to create trusted input.

(() => {

const VCU_CURSOR_ID = "vcu-virtual-cursor";
const VCU_CONTENT_VERSION = "0.2.6";
const VCU_CURSOR_HIDE_MS = 520;
const VCU_HALO_MS = 240;
const VCU_LAYOUT_MAX_NODES = 1000;
const VCU_LAYOUT_SCAN_MAX = 5000;
const VCU_INTERACTIVE_SELECTOR = "button,a,input,textarea,select,summary,[role=button],[role=link],[role=checkbox],[role=radio],[role=switch],[role=textbox],[role=tab],[role=menuitem],[role=option],[role=combobox],[contenteditable=true],[tabindex]";
const VCU_ACTION_TYPES = new Set([
  "vcu_extract",
  "vcu_ping_page",
  "vcu_type",
  "vcu_click",
  "vcu_hover",
  "vcu_scroll",
  "vcu_viewport",
  "vcu_click_point",
]);

let cursorHost = null;
let cursorRoot = null;
let cursorHalo = null;
let cursorArrow = null;
let cursorZoom = 1;
const cursorTimers = new Set();

// The previous bridge used a boolean sentinel and an anonymous listener. If
// that bridge is still alive in an already-open page, do not stack another
// listener: the background side will see the missing version and request a
// page refresh. New pages and same-version reinjection stay idempotent.
const vcuExistingSentinel = globalThis.__vcuContent;
const vcuLegacyListener = vcuExistingSentinel === true;
const vcuSameVersion = !!vcuExistingSentinel && vcuExistingSentinel.version === VCU_CONTENT_VERSION;
const vcuState = vcuSameVersion ? vcuExistingSentinel : {
  version: VCU_CONTENT_VERSION,
  listenerInstalled: true,
  document_id: null,
  document_ref: null,
  revision: 0,
  observer: null,
  input_document: null,
  input_listener: null,
  layout_ids: new WeakMap(),
  next_layout_id: 1,
};
if (!vcuLegacyListener && !vcuSameVersion) {
  globalThis.__vcuContent = vcuState;
  if (globalThis.chrome?.runtime?.onMessage?.addListener) {
    chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
      if (!msg || !VCU_ACTION_TYPES.has(msg.type)) return;
      try {
        if (msg.cursor_zoom !== undefined) setCursorZoom(msg.cursor_zoom);
        if (msg.type === "vcu_ping_page") sendResponse(pingPage());
        else if (msg.type === "vcu_type") sendResponse(typeDom(msg.selector, msg.text, msg.dry_run));
        else if (msg.type === "vcu_click") sendResponse(clickDom(msg.selector, msg.dry_run));
        else if (msg.type === "vcu_hover") sendResponse(hoverDom(msg.selector, msg.dry_run));
        else if (msg.type === "vcu_scroll") sendResponse(scrollDom(msg.dy, msg.dry_run));
        else if (msg.type === "vcu_viewport") sendResponse(viewportSnapshot());
        else if (msg.type === "vcu_click_point") sendResponse(clickPoint(msg.x, msg.y, msg.expected_viewport, msg.dry_run));
        else sendResponse(extractDom(msg.selector || "a"));
      } catch (e) {
        sendResponse({ ok: false, error: String(e) });
      }
      return true;
    });
  }
}

// A same-version reinjection reuses the sentinel's document identity,
// revision, and observer. This keeps snapshots stable while avoiding a second
// message listener or a second MutationObserver.
ensureMutationObserver();

function pingPage() {
  return {
    ok: true,
    pong: true,
    version: VCU_CONTENT_VERSION,
    url: location.href,
    title: document.title || "",
    source: "extension_dom",
  };
}

function viewportSnapshot() {
  // Flush mutations that were queued before the request. The cursor is then
  // removed, and a second flush proves that removing our own overlay did not
  // make the page snapshot stale.
  flushMutationRecords();
  const before = currentViewport();
  hideVirtualCursor();
  flushMutationRecords();
  const after = currentViewport();
  vcuState.last_viewport_stable = JSON.stringify(before) === JSON.stringify(after);
  return { ok: true, source: "extension_dom", viewport: after };
}

function clickPoint(x, y, expectedViewport, dryRun) {
  ensureDocumentState();
  flushMutationRecords();
  const point = finiteCssPoint(x, y);
  if (!point.ok) return point;
  const current = currentViewport();
  if (!sameViewport(current, expectedViewport)) return staleViewport(current);
  if (current.layout_signature?.overflow) {
    return reject("layout signature exceeded its safety budget; narrow the page and recapture", "layout_signature_overflow");
  }
  if (!pointInViewport(point)) return invalidPoint("point is outside the current CSS viewport");
  if (typeof document.elementFromPoint !== "function") return reject("elementFromPoint unavailable");

  const hit = document.elementFromPoint(point.x, point.y);
  if (!hit) return reject("point is not hit-testable");
  const tag = String(hit.tagName || "").toUpperCase();
  if (tag === "IFRAME" || tag === "FRAME" || tag === "CANVAS" || tag === "OBJECT") {
    return reject("point target requires trusted input: " + tag.toLowerCase(), "unsupported_point_target");
  }
  const target = closestInteractiveElement(hit) || hit;
  const capturedHere = expectedViewport.layout_signature?.nodes?.filter(node =>
    point.x >= node.left && point.y >= node.top && point.x < node.left + node.width && point.y < node.top + node.height
  ) || [];
  if (capturedHere.length && !capturedHere.some(node => node.id === layoutNodeId(target))) {
    return staleViewport(current);
  }
  const validation = validateTarget(target);
  if (!validation.ok) return reject(validation.error);
  const rect = rectOf(target);
  const result = {
    ok: true,
    source: "extension_dom",
    pressed: false,
    dry_run: !!dryRun,
    css_point: { x: point.x, y: point.y },
    rectangle: rect,
    input_path: "dom_point_click",
    trusted: false,
    os_cursor_used: false,
    tag: target.tagName,
    target_tag: target.tagName,
    text: String(target.innerText || "").slice(0, 80),
  };
  if (dryRun) return result;

  showVirtualCursor(point, { pulse: true });
  try {
    const dispatched = target.dispatchEvent(makeMouseEvent("click", point));
    result.pressed = true;
    result.default_prevented = dispatched === false;
    return result;
  } catch (e) {
    hideVirtualCursor();
    return reject("could not dispatch point click: " + String(e));
  }
}

function finiteCssPoint(x, y) {
  if (typeof x !== "number" || !Number.isFinite(x) || typeof y !== "number" || !Number.isFinite(y)) {
    return invalidPoint("x and y must be finite CSS numbers");
  }
  return { ok: true, x, y };
}

function invalidPoint(error) {
  return reject(error, "invalid_point");
}

function staleViewport(current) {
  return {
    ok: false,
    error: "stale viewport; recapture screenshot",
    error_code: "stale_viewport",
    stale: true,
    recapture: true,
    source: "extension_dom",
    os_cursor_used: false,
    viewport: current,
  };
}

function sameViewport(current, expected) {
  if (!expected || typeof expected !== "object") return false;
  const fields = [
    "document_id",
    "url",
    "width",
    "height",
    "scroll_x",
    "scroll_y",
    "device_pixel_ratio",
    "visual_scale",
    "visual_offset_x",
    "visual_offset_y",
    "revision",
    "layout_signature",
  ];
  return fields.every((field) => {
    if (!Object.prototype.hasOwnProperty.call(expected, field)) return false;
    if (field === "layout_signature") return sameLayoutSignature(expected[field], current[field]);
    return expected[field] === current[field];
  });
}

function sameLayoutSignature(expected, current) {
  if (!expected || !current) return false;
  const has = (obj,key) => Object.prototype.hasOwnProperty.call(obj,key);
  const equalFields = (a,b,fields) => fields.every(key => has(a,key) && has(b,key) && a[key] === b[key]);
  if (!equalFields(expected,current,["version","max_nodes","scan_max_nodes","visible_count","scanned_count","overflow","overflow_reason"])) return false;
  if (!Array.isArray(expected.nodes) || !Array.isArray(current.nodes) || expected.nodes.length !== current.nodes.length) return false;
  return expected.nodes.every((node,index) => equalFields(node,current.nodes[index],
    ["id","tag","left","top","width","height","visible","disabled","center_hit"]));
}

function closestInteractiveElement(hit) {
  const selector = VCU_INTERACTIVE_SELECTOR;
  try {
    const closest = hit.closest?.(selector);
    if (closest) return closest;
  } catch (_) {}
  let node = hit;
  while (node) {
    const tag = String(node.tagName || "").toUpperCase();
    const role = String(node.getAttribute?.("role") || "").toLowerCase();
    if (["BUTTON", "A", "INPUT", "TEXTAREA", "SELECT", "SUMMARY"].includes(tag) ||
      ["button", "link", "checkbox", "radio", "switch", "textbox", "tab", "menuitem", "option", "combobox"].includes(role) || node.isContentEditable || hasAttr(node,"tabindex")) return node;
    node = node.parentElement || node.parentNode || null;
  }
  return null;
}

function ensureDocumentState() {
  const doc = globalThis.document;
  if (!doc) return;
  if (vcuState.document_ref && vcuState.document_ref !== doc) {
    try { vcuState.observer?.disconnect?.(); } catch (_) {}
    try { vcuState.input_document?.removeEventListener?.("input", vcuState.input_listener, true); } catch (_) {}
    try { vcuState.input_document?.removeEventListener?.("change", vcuState.input_listener, true); } catch (_) {}
    vcuState.observer = null;
    vcuState.input_document = null;
    vcuState.input_listener = null;
    vcuState.revision = 0;
    vcuState.document_id = null;
    vcuState.layout_ids = new WeakMap();
    vcuState.next_layout_id = 1;
  }
  vcuState.document_ref = doc;
  if (!vcuState.document_id) vcuState.document_id = makeDocumentId();
  if (!Number.isFinite(vcuState.revision)) vcuState.revision = 0;
  if (!(vcuState.layout_ids instanceof WeakMap)) vcuState.layout_ids = new WeakMap();
  if (!Number.isInteger(vcuState.next_layout_id) || vcuState.next_layout_id < 1) vcuState.next_layout_id = 1;
  installInputListeners(doc);
}

function makeDocumentId() {
  try {
    if (typeof globalThis.crypto?.randomUUID === "function") return "doc-" + globalThis.crypto.randomUUID();
  } catch (_) {}
  return "doc-" + Date.now().toString(36) + "-" + Math.random().toString(36).slice(2, 12);
}

function currentViewport() {
  ensureDocumentState();
  const doc = globalThis.document || {};
  const root = doc.documentElement || {};
  const win = globalThis.window || {};
  const visual = win.visualViewport || {};
  return {
    document_id: vcuState.document_id,
    url: String(globalThis.location?.href || ""),
    width: finiteNumber(win.innerWidth, finiteNumber(root.clientWidth, 0)),
    height: finiteNumber(win.innerHeight, finiteNumber(root.clientHeight, 0)),
    scroll_x: finiteNumber(win.scrollX, finiteNumber(win.pageXOffset, 0)),
    scroll_y: finiteNumber(win.scrollY, finiteNumber(win.pageYOffset, 0)),
    device_pixel_ratio: finiteNumber(win.devicePixelRatio, 1),
    visual_scale: finiteNumber(visual.scale, 1),
    visual_offset_x: finiteNumber(visual.offsetLeft, 0),
    visual_offset_y: finiteNumber(visual.offsetTop, 0),
    revision: vcuState.revision,
    layout_signature: layoutSignature(),
  };
}

function finiteNumber(value, fallback) {
  const n = Number(value);
  return Number.isFinite(n) ? n : fallback;
}

function installInputListeners(doc) {
  if (!doc || vcuState.input_document === doc) return;
  if (vcuState.input_document && vcuState.input_listener) {
    try { vcuState.input_document.removeEventListener?.("input", vcuState.input_listener, true); } catch (_) {}
    try { vcuState.input_document.removeEventListener?.("change", vcuState.input_listener, true); } catch (_) {}
  }
  const listener = (event) => {
    if (isCursorNode(event?.target)) return;
    vcuState.revision += 1;
  };
  try {
    doc.addEventListener?.("input", listener, true);
    doc.addEventListener?.("change", listener, true);
  } catch (_) {}
  vcuState.input_document = doc;
  vcuState.input_listener = listener;
}

function layoutSignature() {
  // This deliberately records no labels, hrefs, text, or control values.
  // Passwords and hidden page data therefore never enter the screenshot
  // binding; only visible target identity, geometry, visibility, and disabled
  // state participate.
  // Inspect at most the safety scan budget. An overflow marker is safer than
  // silently pretending that a partial signature is complete.
  const nodes = [];
  let visibleCount = 0;
  let overflow = false;
  let overflow_reason = null;
  let candidates;
  try {
    candidates = document.querySelectorAll(VCU_INTERACTIVE_SELECTOR);
  } catch (_) {
    return {
      version: 1,
      max_nodes: VCU_LAYOUT_MAX_NODES,
      scan_max_nodes: VCU_LAYOUT_SCAN_MAX,
      visible_count: 0,
      scanned_count: 0,
      overflow: true,
      overflow_reason: "query_failed",
      nodes: [],
    };
  }
  const candidateCount = candidates?.length || 0;
  const scanCount = Math.min(candidateCount, VCU_LAYOUT_SCAN_MAX);
  if (candidateCount > VCU_LAYOUT_SCAN_MAX) {
    overflow = true;
    overflow_reason = "scan_cap";
  }
  for (let index = 0; index < scanCount; index += 1) {
    const el = candidates[index];
    const rect = rectOf(el);
    if (!isVisibleInteractive(el, rect)) continue;
    visibleCount += 1;
    if (visibleCount > VCU_LAYOUT_MAX_NODES) {
      overflow = true;
      overflow_reason = "visible_node_cap";
      break;
    }
    nodes.push({
      id: layoutNodeId(el),
      tag: String(el.tagName || "").toUpperCase(),
      left: numberForSignature(rect.left),
      top: numberForSignature(rect.top),
      width: numberForSignature(rect.width),
      height: numberForSignature(rect.height),
      visible: true,
      disabled: isDisabled(el),
      center_hit: visibleCenterHit(rect),
    });
  }
  return {
    version: 1,
    max_nodes: VCU_LAYOUT_MAX_NODES,
    scan_max_nodes: VCU_LAYOUT_SCAN_MAX,
    visible_count: overflow_reason === "visible_node_cap" ? VCU_LAYOUT_MAX_NODES + 1 : visibleCount,
    scanned_count: scanCount,
    overflow,
    overflow_reason,
    nodes,
  };
}

function visibleCenterHit(rect) {
  const view = viewport();
  const x = (Math.max(0, rect.left) + Math.min(view.width, rect.right)) / 2;
  const y = (Math.max(0, rect.top) + Math.min(view.height, rect.bottom)) / 2;
  try {
    const hit = document.elementFromPoint(x, y);
    return hit ? layoutNodeId(closestInteractiveElement(hit) || hit) : null;
  } catch (_) { return null; }
}

function isVisibleInteractive(el, rect = rectOf(el)) {
  if (!el || el.isConnected === false || isHidden(el)) return false;
  const view = viewport();
  return rect.width > 0 && rect.height > 0 && rect.right > 0 && rect.bottom > 0 && rect.left < view.width && rect.top < view.height;
}

function layoutNodeId(el) {
  let id = vcuState.layout_ids.get(el);
  if (!id) {
    id = "i" + vcuState.next_layout_id++;
    vcuState.layout_ids.set(el, id);
  }
  return id;
}

function numberForSignature(value) {
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

function ensureMutationObserver() {
  ensureDocumentState();
  const doc = globalThis.document;
  if (!doc?.documentElement || vcuState.observer) return;
  const Observer = globalThis.MutationObserver || globalThis.window?.MutationObserver;
  if (typeof Observer !== "function") return;
  try {
    const observer = new Observer((records) => recordMutations(records));
    observer.observe(doc.documentElement, {
      subtree: true,
      childList: true,
      attributes: true,
      characterData: true,
    });
    vcuState.observer = observer;
  } catch (_) {}
}

function flushMutationRecords() {
  ensureMutationObserver();
  const observer = vcuState.observer;
  if (!observer || typeof observer.takeRecords !== "function") return 0;
  let records = [];
  try { records = observer.takeRecords() || []; } catch (_) { return 0; }
  return recordMutations(records);
}

function recordMutations(records) {
  let changed = 0;
  for (const record of records || []) {
    if (!cursorOnlyMutation(record)) changed += 1;
  }
  if (changed) vcuState.revision += changed;
  return changed;
}

function cursorOnlyMutation(record) {
  if (!record) return true;
  if (isCursorNode(record.target)) return true;
  const nodes = [];
  for (const node of record.addedNodes || []) nodes.push(node);
  for (const node of record.removedNodes || []) nodes.push(node);
  return nodes.length > 0 && nodes.every((node) => isCursorNode(node));
}

function isCursorNode(node) {
  if (!node) return false;
  if (node === cursorHost || node.id === VCU_CURSOR_ID) return true;
  try {
    if (cursorHost?.contains?.(node)) return true;
    if (cursorHost?.shadowRoot && node.getRootNode?.() === cursorHost.shadowRoot) return true;
  } catch (_) {}
  return false;
}

function extractDom(sel) {
  let nl;
  try {
    nl = document.querySelectorAll(sel);
  } catch (e) {
    return { ok: false, error: String(e), source: "extension_dom" };
  }
  const matches = [];
  const limit = 200;
  for (let i = 0; i < nl.length && matches.length < limit; i++) {
    const el = nl[i];
    matches.push({
      tag: el.tagName,
      text: String(el.innerText || "").slice(0, 160),
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
  const resolved = resolveUniqueSelector(selector);
  if (!resolved.ok) return resolved;
  if (isDocumentContainer(resolved.element)) return reject("document container is not an editable text element");
  const editable = editableKind(resolved.element);
  if (!editable) return reject("target is not an editable text element");

  let validation = validateTarget(resolved.element, { editable: true });
  if (!validation.ok) return reject(validation.error);
  let rect = rectOf(resolved.element);
  let point = pointOf(rect);
  let needsScroll = needsScrollFor(rect, point);

  // A dry-run is read-only. In particular, do not focus, scroll, add the
  // cursor host, set a value, or dispatch an event here.
  if (dryRun) {
    return actionResult("type", resolved.element, point, rect, {
      ok: true,
      typed: false,
      dry_run: true,
      needs_scroll: needsScroll,
      ready: !needsScroll && pointInViewport(point),
    });
  }

  if (needsScroll || !pointInViewport(point)) {
    const scrolled = scrollTarget(resolved.element);
    if (!scrolled.ok) return reject(scrolled.error);
    validation = validateTarget(resolved.element, { editable: true });
    if (!validation.ok) return reject(validation.error);
    rect = rectOf(resolved.element);
    point = pointOf(rect);
    needsScroll = needsScrollFor(rect, point);
  }
  if (!pointInViewport(point)) return reject("target point is outside the viewport after scrolling");

  try {
    if (typeof resolved.element.focus === "function") {
      try { resolved.element.focus({ preventScroll: true }); } catch (_) { resolved.element.focus(); }
    }
  } catch (e) {
    return reject("could not focus target: " + String(e));
  }

  // Focus handlers can change layout or put another element above the field.
  validation = validateTarget(resolved.element, { editable: true });
  if (!validation.ok) return reject(validation.error);
  rect = rectOf(resolved.element);
  point = pointOf(rect);
  if (!pointInViewport(point)) return reject("target point left the viewport");
  const hit = verifyHit(resolved.element, point);
  if (!hit.ok) return reject(hit.error);

  showVirtualCursor(point, { pulse: false });
  const value = text == null ? "" : String(text);
  try {
    if (editable === "contenteditable") {
      // The selector was resolved against the editable node, so this cannot
      // accidentally replace document.body or another page container.
      resolved.element.textContent = value;
    } else {
      setNativeValue(resolved.element, value);
    }
    dispatchInput(resolved.element, value);
    dispatchChange(resolved.element);
  } catch (e) {
    hideVirtualCursor();
    return reject("could not set target value: " + String(e));
  }
  return actionResult("type", resolved.element, point, rect, {
    ok: true,
    typed: true,
    dry_run: false,
    text: value.slice(0, 80),
    needs_scroll: false,
  });
}

function clickDom(selector, dryRun) {
  const resolved = resolveUniqueSelector(selector);
  if (!resolved.ok) return resolved;
  let validation = validateTarget(resolved.element);
  if (!validation.ok) return reject(validation.error);

  let rect = rectOf(resolved.element);
  let point = pointOf(rect);
  let needsScroll = needsScrollFor(rect, point);

  // Dry-run returns geometry and the scroll requirement only. It must not
  // mutate the page or claim that a click happened.
  if (dryRun) {
    if (!needsScroll && pointInViewport(point)) {
      const hit = verifyHit(resolved.element, point);
      if (!hit.ok) return reject(hit.error);
    }
    return actionResult("click", resolved.element, point, rect, {
      ok: true,
      pressed: false,
      dry_run: true,
      needs_scroll: needsScroll || !pointInViewport(point),
      ready: !needsScroll && pointInViewport(point),
    });
  }

  if (needsScroll || !pointInViewport(point)) {
    const scrolled = scrollTarget(resolved.element);
    if (!scrolled.ok) return reject(scrolled.error);
    validation = validateTarget(resolved.element);
    if (!validation.ok) return reject(validation.error);
    rect = rectOf(resolved.element);
    point = pointOf(rect);
    needsScroll = needsScrollFor(rect, point);
  }
  if (!pointInViewport(point)) return reject("target point is outside the viewport after scrolling");

  // Always re-measure and hit-test after scroll. A page can move or cover the
  // target while scrollIntoView runs.
  const hit = verifyHit(resolved.element, point);
  if (!hit.ok) return reject(hit.error);

  showVirtualCursor(point, { pulse: true });
  try {
    const event = makeMouseEvent("click", point);
    resolved.element.dispatchEvent(event);
  } catch (e) {
    hideVirtualCursor();
    return reject("could not dispatch click: " + String(e));
  }
  return actionResult("click", resolved.element, point, rect, {
    ok: true,
    pressed: true,
    dry_run: false,
    needs_scroll: false,
    trusted: false,
  });
}

function hoverDom(selector, dryRun) {
  const resolved = resolveUniqueSelector(selector);
  if (!resolved.ok) return resolved;
  let validation = validateTarget(resolved.element);
  if (!validation.ok) return reject(validation.error);
  let rect = rectOf(resolved.element);
  let point = pointOf(rect);
  let needsScroll = needsScrollFor(rect, point);

  if (dryRun) {
    if (!needsScroll && pointInViewport(point)) {
      const hit = verifyHit(resolved.element, point);
      if (!hit.ok) return reject(hit.error);
    }
    return actionResult("hover", resolved.element, point, rect, {
      ok: true,
      hovered: false,
      dry_run: true,
      needs_scroll: needsScroll || !pointInViewport(point),
      ready: !needsScroll && pointInViewport(point),
    });
  }

  if (needsScroll || !pointInViewport(point)) {
    const scrolled = scrollTarget(resolved.element);
    if (!scrolled.ok) return reject(scrolled.error);
    validation = validateTarget(resolved.element);
    if (!validation.ok) return reject(validation.error);
    rect = rectOf(resolved.element);
    point = pointOf(rect);
  }
  if (!pointInViewport(point)) return reject("target point is outside the viewport after scrolling");
  const hit = verifyHit(resolved.element, point);
  if (!hit.ok) return reject(hit.error);

  showVirtualCursor(point, { pulse: false });
  try {
    for (const type of ["mousemove", "mouseover", "mouseenter"]) {
      resolved.element.dispatchEvent(makeMouseEvent(type, point));
    }
  } catch (e) {
    hideVirtualCursor();
    return reject("could not dispatch hover: " + String(e));
  }
  return actionResult("hover", resolved.element, point, rect, {
    ok: true,
    hovered: true,
    dry_run: false,
    needs_scroll: false,
    trusted: false,
  });
}

function scrollDom(dy, dryRun) {
  const y = Number(dy) || 0;
  if (dryRun) {
    return { ok: true, scrolled: false, dry_run: true, dy: y, source: "extension_dom", os_cursor_used: false };
  }
  if (typeof window?.scrollBy !== "function") return reject("window.scrollBy unavailable");
  window.scrollBy(0, y);
  return { ok: true, scrolled: true, dy: y, source: "extension_dom", os_cursor_used: false };
}

function resolveUniqueSelector(selector) {
  if (typeof selector !== "string" || !selector.trim()) return reject("selector required");
  let nodes;
  try {
    nodes = Array.from(document.querySelectorAll(selector));
  } catch (e) {
    return reject("invalid selector: " + String(e));
  }
  if (nodes.length === 0) return reject("selector not found");
  if (nodes.length !== 1) return reject("selector is ambiguous (matched " + nodes.length + " elements)");
  return { ok: true, element: nodes[0] };
}

function validateTarget(el, options = {}) {
  if (!el) return { ok: false, error: "target not found" };
  if (el.isConnected === false) return { ok: false, error: "target is detached" };
  if (isHidden(el)) return { ok: false, error: "target is hidden" };
  if (isDisabled(el)) return { ok: false, error: "target is disabled" };
  if (isInert(el)) return { ok: false, error: "target is inert" };
  if (options.editable && (isDocumentContainer(el) || !editableKind(el))) return { ok: false, error: "target is not editable" };
  if (options.editable && isReadonly(el)) return { ok: false, error: "target is readonly" };
  const rect = rectOf(el);
  if (!(rect.width > 0 && rect.height > 0)) return { ok: false, error: "target has no visible bounds" };
  return { ok: true };
}

function isHidden(el) {
  if (el.hidden === true || hasAttr(el, "hidden") || attr(el, "aria-hidden") === "true") return true;
  try {
    if (typeof el.closest === "function" && el.closest("[hidden], [aria-hidden=\"true\"]")) return true;
  } catch (_) {}
  const style = computedStyle(el);
  if (!style) return false;
  if (String(style.display || "").toLowerCase() === "none") return true;
  if (["hidden", "collapse"].includes(String(style.visibility || "").toLowerCase())) return true;
  const opacityValue = style.opacity;
  if (opacityValue == null || opacityValue === "") return false;
  const opacity = Number(opacityValue);
  return Number.isFinite(opacity) && opacity <= 0;
}

function isDisabled(el) {
  if (el.disabled === true || hasAttr(el, "disabled") || attr(el, "aria-disabled") === "true") return true;
  try {
    if (typeof el.matches === "function" && el.matches(":disabled")) return true;
  } catch (_) {}
  try {
    if (typeof el.closest === "function" && el.closest("fieldset[disabled]")) return true;
  } catch (_) {}
  return false;
}

function isInert(el) {
  if (el.inert === true || hasAttr(el, "inert")) return true;
  try {
    if (typeof el.closest === "function" && el.closest("[inert]")) return true;
  } catch (_) {}
  return false;
}

function isReadonly(el) {
  return el.readOnly === true || hasAttr(el, "readonly") || attr(el, "aria-readonly") === "true";
}

function isDocumentContainer(el) {
  const tag = String(el?.tagName || "").toUpperCase();
  return el === globalThis.document?.body || el === globalThis.document?.documentElement || tag === "BODY" || tag === "HTML";
}

function editableKind(el) {
  const tag = String(el.tagName || "").toUpperCase();
  const contentEditable = el.isContentEditable === true ||
    (attr(el, "contenteditable") != null && attr(el, "contenteditable") !== "false");
  if (contentEditable) return "contenteditable";
  if (tag === "TEXTAREA") return "textarea";
  if (tag !== "INPUT") return null;
  const type = String(el.type || attr(el, "type") || "text").toLowerCase();
  if (["hidden", "button", "checkbox", "color", "file", "image", "radio", "range", "reset", "submit"].includes(type)) return null;
  return "input";
}

function computedStyle(el) {
  try {
    const getStyle = globalThis.getComputedStyle || globalThis.window?.getComputedStyle;
    return typeof getStyle === "function" ? getStyle(el) : (el.style || null);
  } catch (_) {
    return el.style || null;
  }
}

function rectOf(el) {
  const r = typeof el.getBoundingClientRect === "function" ? el.getBoundingClientRect() : {};
  const left = Number(r.left ?? r.x ?? 0);
  const top = Number(r.top ?? r.y ?? 0);
  const width = Number(r.width ?? ((r.right ?? left) - left));
  const height = Number(r.height ?? ((r.bottom ?? top) - top));
  return {
    x: left,
    y: top,
    left,
    top,
    width,
    height,
    right: Number(r.right ?? left + width),
    bottom: Number(r.bottom ?? top + height),
  };
}

function pointOf(rect) {
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

function viewport() {
  const d = globalThis.document?.documentElement || {};
  const w = globalThis.window || {};
  return {
    width: Number(w.innerWidth || d.clientWidth || 0),
    height: Number(w.innerHeight || d.clientHeight || 0),
  };
}

function pointInViewport(point) {
  const v = viewport();
  return point.x >= 0 && point.y >= 0 && point.x < v.width && point.y < v.height;
}

function needsScrollFor(rect, point) {
  const v = viewport();
  return rect.left < 0 || rect.top < 0 || rect.right > v.width || rect.bottom > v.height || !pointInViewport(point);
}

function scrollTarget(el) {
  if (typeof el.scrollIntoView !== "function") return reject("target cannot be scrolled into view");
  try {
    el.scrollIntoView({ block: "center", inline: "center", behavior: "instant" });
  } catch (_) {
    try { el.scrollIntoView({ block: "center", inline: "center" }); }
    catch (e) { return reject("could not scroll target: " + String(e)); }
  }
  return { ok: true };
}

function verifyHit(el, point) {
  const style = computedStyle(el);
  if (style && String(style.pointerEvents || "").toLowerCase() === "none") {
    return reject("target has pointer-events:none");
  }
  if (!pointInViewport(point)) return reject("target point is outside the viewport");
  if (typeof document.elementFromPoint !== "function") return reject("elementFromPoint unavailable");
  const hit = document.elementFromPoint(point.x, point.y);
  if (!hit) return reject("target point is not hit-testable");
  if (hit === el || (typeof el.contains === "function" && el.contains(hit))) return { ok: true, hit };
  return reject("target is occluded by " + String(hit.tagName || "another element").toLowerCase());
}

function actionResult(kind, el, point, rect, extra) {
  const result = Object.assign({
    source: "extension_dom",
    css_point: { x: point.x, y: point.y },
    rectangle: rect,
    tag: el.tagName,
    os_cursor_used: false,
    trusted: false,
    input_path: kind === "click" ? "dom_click" : kind === "type" ? "dom_type" : "dom_hover",
  }, extra || {});
  if (kind === "click") result.text = String(el.innerText || "").slice(0, 80);
  return result;
}

function reject(error, errorCode) {
  const result = { ok: false, error: String(error), source: "extension_dom", os_cursor_used: false };
  if (errorCode) result.error_code = errorCode;
  return result;
}

function makeMouseEvent(type, point) {
  const Ctor = globalThis.MouseEvent || globalThis.window?.MouseEvent;
  if (typeof Ctor !== "function") {
    const event = new Event(type, { bubbles: true, cancelable: true });
    try {
      Object.defineProperties(event, {
        clientX: { value: point.x },
        clientY: { value: point.y },
        button: { value: 0 },
        buttons: { value: type === "mouseup" ? 0 : 1 },
      });
    } catch (_) {}
    return event;
  }
  return new Ctor(type, {
    bubbles: true,
    cancelable: true,
    view: globalThis.window || null,
    clientX: point.x,
    clientY: point.y,
    button: 0,
    buttons: type === "mouseup" ? 0 : 1,
    detail: type === "click" ? 1 : 0,
  });
}

function setNativeValue(el, value) {
  const tag = String(el.tagName || "").toUpperCase();
  const ownerWindow = el.ownerDocument?.defaultView || globalThis.window || globalThis;
  const nativeConstructor = tag === "INPUT"
    ? ownerWindow.HTMLInputElement
    : tag === "TEXTAREA"
      ? ownerWindow.HTMLTextAreaElement
      : null;
  let descriptor = nativeConstructor?.prototype
    ? Object.getOwnPropertyDescriptor(nativeConstructor.prototype, "value")
    : null;

  // A page framework can install an own `value` descriptor (React's value
  // tracker is a common example). Calling that setter would update only the
  // framework's tracker and may leave the browser control unchanged. Prefer
  // the native setter from the element's own realm, then walk prototypes as
  // a fallback for test doubles and custom controls.
  let proto = Object.getPrototypeOf(el);
  while (!descriptor && proto) {
    descriptor = Object.getOwnPropertyDescriptor(proto, "value");
    proto = Object.getPrototypeOf(proto);
  }
  if (descriptor && typeof descriptor.set === "function") descriptor.set.call(el, value);
  else el.value = value;
}

function dispatchInput(el, value) {
  const Ctor = globalThis.InputEvent || globalThis.Event;
  const init = { bubbles: true, cancelable: true, inputType: "insertText", data: value };
  el.dispatchEvent(new Ctor("input", init));
}

function dispatchChange(el) {
  const Ctor = globalThis.Event;
  el.dispatchEvent(new Ctor("change", { bubbles: true }));
}

function showVirtualCursor(point, options = {}) {
  const cursor = ensureVirtualCursor();
  if (!cursor) return;
  clearCursorTimers();
  cursorRoot.style.left = point.x + "px";
  cursorRoot.style.top = point.y + "px";
  cursorRoot.style.transform = `scale(${1 / cursorZoom})`;
  cursorRoot.setAttribute("data-css-point", point.x + "," + point.y);
  cursorHalo.classList.remove("pulse");
  if (options.pulse) {
    // Force a fresh animation when two clicks happen in quick succession.
    void cursorHalo.offsetWidth;
    cursorHalo.classList.add("pulse");
    scheduleCursor(() => cursorHalo?.classList.remove("pulse"), VCU_HALO_MS);
  }
  scheduleCursor(() => hideVirtualCursor(), VCU_CURSOR_HIDE_MS);
}

function setCursorZoom(zoom) {
  const n = Number(zoom);
  cursorZoom = Number.isFinite(n) && n > 0 ? n : 1;
}

function ensureVirtualCursor() {
  const d = globalThis.document;
  if (!d?.documentElement) return null;
  if (cursorHost && cursorHost.isConnected !== false && cursorRoot && cursorHalo && cursorArrow) {
    return { host: cursorHost, root: cursorRoot, halo: cursorHalo, arrow: cursorArrow };
  }
  clearCursorTimers();
  const previous = d.getElementById?.(VCU_CURSOR_ID);
  if (previous && typeof previous.remove === "function") previous.remove();
  const host = d.createElement("div");
  host.id = VCU_CURSOR_ID;
  host.setAttribute("aria-hidden", "true");
  host.style.cssText = [
    "position:fixed !important",
    "display:block !important",
    "visibility:visible !important",
    "opacity:1 !important",
    "left:0 !important",
    "top:0 !important",
    "width:0 !important",
    "height:0 !important",
    "margin:0 !important",
    "padding:0 !important",
    "border:0 !important",
    "z-index:2147483647 !important",
    "pointer-events:none !important",
    "user-select:none !important",
    "isolation:isolate !important",
    "overflow:visible !important",
    "background:transparent !important",
    "transform:none !important",
  ].join(";");
  const shadow = typeof host.attachShadow === "function" ? host.attachShadow({ mode: "open" }) : host;
  shadow.innerHTML = `
    <style>
      .vcu-cursor {
        position: fixed; left: 0; top: 0; width: 20px; height: 22px;
        margin: 0; padding: 0; pointer-events: none; overflow: visible;
        transform: none; transform-origin: 0 0; isolation: isolate;
      }
      /* Same-background PARITY-004: native Codex CU crop + public fog metrics
         (~66px circular haze, compact dart). No hard ring, no long stem. */
      .vcu-halo {
        position: absolute; left: -24px; top: -22px; width: 66px; height: 66px;
        pointer-events: none; border: 0; border-radius: 50%;
        background: radial-gradient(circle at 48% 44%, rgba(148,168,188,.50) 0%, rgba(170,184,200,.26) 36%, rgba(206,212,222,.11) 60%, transparent 78%);
        filter: blur(6px); opacity: .98;
      }
      .vcu-halo::after {
        content: ""; position: absolute; inset: 16px;
        border-radius: 50%;
        background: radial-gradient(circle, rgba(180,192,206,.16), transparent 72%);
      }
      .vcu-halo.pulse { animation: vcu-halo-pulse 240ms ease-out both; }
      .vcu-arrow {
        position: absolute; left: 0; top: 0; width: 20px; height: 22px;
        overflow: visible; pointer-events: none; display: block;
        filter: drop-shadow(0 1px 1px rgba(44,56,70,.14));
      }
      .vcu-arrow polygon { fill: #5a6068; fill-opacity: .96; stroke: #fff; stroke-opacity: .92; stroke-width: 1.5;
        stroke-linejoin: round; }
      @keyframes vcu-halo-pulse {
        0% { opacity: .95; transform: scale(.92); }
        55% { opacity: 1; transform: scale(1.12); }
        100% { opacity: .86; transform: scale(1); }
      }
      @media (prefers-reduced-motion: reduce) {
        .vcu-halo.pulse { animation: none; opacity: .72; }
      }
    </style>
    <div class="vcu-cursor">
      <div class="vcu-halo"></div>
      <svg class="vcu-arrow" viewBox="0 0 20 22" aria-hidden="true" focusable="false">
        <polygon points="0,0 18,10 10,12 4,20"></polygon>
      </svg>
    </div>`;
  d.documentElement.appendChild(host);
  cursorHost = host;
  cursorRoot = shadow.querySelector?.(".vcu-cursor") || shadow.firstElementChild;
  cursorHalo = shadow.querySelector?.(".vcu-halo") || null;
  cursorArrow = shadow.querySelector?.(".vcu-arrow") || null;
  if (!cursorRoot || !cursorHalo || !cursorArrow) {
    hideVirtualCursor();
    return null;
  }
  return { host: cursorHost, root: cursorRoot, halo: cursorHalo, arrow: cursorArrow };
}

function scheduleCursor(fn, ms) {
  const id = setTimeout(() => {
    cursorTimers.delete(id);
    try { fn(); } catch (_) {}
  }, ms);
  cursorTimers.add(id);
}

function clearCursorTimers() {
  for (const id of cursorTimers) clearTimeout(id);
  cursorTimers.clear();
}

function hideVirtualCursor() {
  clearCursorTimers();
  if (cursorHost && typeof cursorHost.remove === "function") cursorHost.remove();
  else {
    const previous = globalThis.document?.getElementById?.(VCU_CURSOR_ID);
    if (previous && typeof previous.remove === "function") previous.remove();
  }
  cursorHost = null;
  cursorRoot = null;
  cursorHalo = null;
  cursorArrow = null;
}

function attr(el, name) {
  try { return el.getAttribute?.(name); } catch (_) { return null; }
}

function hasAttr(el, name) {
  try { return el.hasAttribute?.(name) === true; } catch (_) { return false; }
}

// Small CommonJS surface for the repository's node:test coverage. The browser
// extension does not define `module`, so this is inert in production.
if (typeof module !== "undefined" && module.exports) {
  module.exports = {
    clickDom,
    hoverDom,
    typeDom,
    scrollDom,
    extractDom,
    pingPage,
    viewportSnapshot,
    clickPoint,
    __vcuTest: {
      ensureVirtualCursor,
      hideVirtualCursor,
      clearCursorTimers,
      flushMutationRecords,
      recordMutations,
      currentViewport,
      setCursorZoom,
      sameViewport,
      state: vcuState,
      constants: {
        VCU_CURSOR_ID,
        VCU_CURSOR_HIDE_MS,
        VCU_HALO_MS,
        VCU_LAYOUT_MAX_NODES,
        VCU_LAYOUT_SCAN_MAX,
      },
    },
  };
}
})();
