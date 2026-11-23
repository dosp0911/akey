// 메뉴바(트레이) 팝오버: 시크릿 목록. 이름·메모·값은 모두 textContent 로만 넣는다 (HTML 로 해석하지 않음).
import { t, setLang } from "./i18n.js";
import { ICONS } from "./icons.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// 문제 확인용: 화면 오류를 앱 기록으로 보낸다
window.addEventListener("error", (e) => invoke("debug_log", { message: `error ${e.message} @${e.filename}:${e.lineno}` }));
window.addEventListener("unhandledrejection", (e) => invoke("debug_log", { message: `rejection ${e.reason}` }));
invoke("debug_log", { message: "app.js loaded" });

const ui = {
  snap: null,
  revealed: new Map(),   // 이름 → 값 (창을 닫으면 비움)
  expanded: new Set(),   // 긴 값을 펼쳐 보는 이름
  editing: null,         // 수정 중인 이름
  draft: { name: "", value: "", note: "" },
  add: { name: "", value: "", note: "" },
  hovered: null,
  menuFor: null,         // 허용 메뉴가 열린 이름
  selecting: false,      // 여러 개 골라 복사하는 중
  selected: new Set(),   // 고른 이름
  query: "",             // 검색어 (이름·메모 부분 일치, 대소문자 무시)
  group: 0,              // 지금 보는 묶음(동그라미)
  renamingGroup: false,  // 묶음 이름 바꾸는 중
  message: "",
  messageIsError: false,
};

// ---- 작은 DOM 도우미 ----
function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === null || v === false) continue;
    if (k === "class") node.className = v;
    else if (k === "text") node.textContent = v;
    else if (k === "icon") node.innerHTML = ICONS[v]; // 고정된 내장 SVG 만
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else node.setAttribute(k, v === true ? "" : v);
  }
  for (const c of children.flat()) if (c) node.append(c);
  return node;
}

// 긴 이름은 밑줄(_) 뒤에서 줄바꿈되게 한다 (글자는 그대로, <wbr> 만 끼움)
function breakableName(name) {
  const parts = name.split("_");
  return parts.flatMap((part, i) => (i < parts.length - 1 ? [part + "_", document.createElement("wbr")] : [part]));
}

function iconButton(icon, label, onclick, extra = "") {
  return el("button", { class: `icon-btn ${extra}`, title: label, "aria-label": label, icon, onclick });
}

function setMessage(text, isError = false) {
  ui.message = text || "";
  ui.messageIsError = isError;
}

// 잠깐 떴다 사라지는 알림 (복사됨 등). 창 크기에 영향을 주지 않게 #root 바깥에 둔다
let toastTimer;
function toast(text) {
  let box = document.getElementById("toast");
  if (!box) {
    box = el("div", { id: "toast", role: "status", "aria-live": "polite" });
    document.body.append(box);
  }
  box.replaceChildren(el("span", { icon: "check" }), el("span", { text }));
  box.classList.remove("show");
  void box.offsetWidth; // 연달아 눌러도 다시 나타나게
  box.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => box.classList.remove("show"), 1400);
}

async function call(cmd, args) {
  try {
    const result = await invoke(cmd, args);
    if (typeof result === "string" && result) setMessage(result);
    return { ok: true, result };
  } catch (err) {
    setMessage(String(err), true);
    return { ok: false };
  }
}

// ---- 데이터 ----
async function refresh() {
  ui.snap = await invoke("snapshot");
  setLang(ui.snap.lang);
  if (!resizing) setFixedSize(Boolean(ui.snap.popover_size));
  for (const h of document.querySelectorAll(".resize-bottom, .resize-corner")) h.title = t("resizeHint");
  if (ui.snap.error) setMessage(ui.snap.error, true);
  // 지워지거나 이름이 바뀐 항목의 보이던 값은 버린다
  const names = new Set(ui.snap.items.map((i) => i.name));
  for (const n of [...ui.revealed.keys()]) if (!names.has(n)) ui.revealed.delete(n);
  for (const n of [...ui.expanded]) if (!ui.revealed.has(n)) ui.expanded.delete(n);
  if (ui.editing && !names.has(ui.editing)) ui.editing = null;
  for (const n of [...ui.selected]) if (!names.has(n)) ui.selected.delete(n);
  render();
}

async function toggleReveal(name) {
  if (ui.revealed.has(name)) {
    ui.revealed.delete(name);
    ui.expanded.delete(name);
    render();
    return;
  }
  const { ok, result } = await callQuiet("reveal", { name });
  if (ok) ui.revealed.set(name, result);
  await refresh();
}

// reveal 의 결과(값)는 상태줄에 쓰지 않는다
async function callQuiet(cmd, args) {
  try {
    return { ok: true, result: await invoke(cmd, args) };
  } catch (err) {
    setMessage(String(err), true);
    return { ok: false };
  }
}

// 검색어가 이름이나 메모의 일부와 맞는 항목만 (공백으로 나눈 낱말은 모두 맞아야 함)
function visibleItems() {
  const words = ui.query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  // 검색어가 없으면 지금 묶음만, 있으면 모든 묶음에서 찾는다
  if (!words.length) return ui.snap.items.filter((i) => i.group === ui.group);
  return ui.snap.items.filter((i) => {
    const hay = `${i.name}\n${i.note || ""}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}

function searchBox() {
  let composing = false; // 한글 입력 조합 중에는 다시 그리지 않는다
  const update = (e) => { ui.query = e.target.value; scrollToTop = true; if (!composing) render(); };
  return el("label", { class: "search" },
    el("span", { icon: "search" }),
    el("input", { type: "search", id: "search", placeholder: t("searchPlaceholder"), value: ui.query,
      spellcheck: "false", autocomplete: "off",
      oninput: update,
      oncompositionstart: () => { composing = true; },
      oncompositionend: (e) => { composing = false; update(e); },
      onkeydown: (e) => {
        if (e.key === "Escape" && ui.query) { e.stopPropagation(); ui.query = ""; render(); }
      } }));
}

// ---- 화면 ----
// 에이전트 전체 접근: 꺼짐(시크릿별) / 8시간 / 항상. 켤 때는 인증을 묻는다
function agentSelect(s) {
  const current = s.all_kind === "ask" ? "off" : s.all_kind;
  const options = [
    ["off", t("agentOff")],
    ["timed", s.all_kind === "timed" ? `${t("agent8h")} · ${s.all_label}` : t("agent8h")],
    ["always", t("agentAlways")],
  ];
  const sel = el("select", { class: "agent-select", "aria-label": t("agentAccess"),
    onchange: async (e) => {
      const v = e.target.value;
      if (v === current) return;
      await call("set_all_grant", { hours: v === "timed" ? 8 : v === "always" ? 0 : null });
      await refresh();
    } },
    options.map(([v, label]) => el("option", { value: v, text: label, selected: v === current })));
  return el("label", { class: `agent kind-${s.all_kind}`, title: t("agentAccessHelp") },
    el("span", { icon: "cpu" }), sel);
}

// 잠김 ↔ 풀림 토글: 잠김을 누르면 인증 후 모든 값을 보이고, 풀림을 누르면 모두 가리고 다시 잠근다
async function unlockAll() {
  const { ok, result } = await callQuiet("reveal_all");
  if (ok) for (const [name, value] of Object.entries(result)) ui.revealed.set(name, value);
  await refresh();
}

async function lockAll() {
  await invoke("lock");
  ui.revealed.clear();
  await refresh();
}

function lockToggle(s) {
  if (s.unlocked) {
    return el("button", { class: "badge green", title: t("unlockedHelp"), onclick: lockAll },
      el("span", { icon: "lockOpen" }), el("span", { text: t("unlocked") }));
  }
  return el("button", { class: "badge kind-ask", title: t("lockedHelp", s.unlock_minutes), onclick: unlockAll },
    el("span", { icon: "lock" }), el("span", { text: t("locked") }));
}

function header(s) {
  return el("div", { class: "header" },
    el("span", { class: "logo", icon: "logo" }),
    el("span", { class: "title", text: "Akey" }),
    el("span", { class: "count", text: String(s.items.length) }),
    el("span", { class: "spacer" }),
    agentSelect(s),
    lockToggle(s));
}

// ---- 묶음(동그라미) ----
function groupLabel(i) {
  const name = ui.snap.group_names[i];
  return name || t("groupN", i + 1);
}

function groupDots() {
  const counts = new Array(ui.snap.group_names.length).fill(0);
  for (const it of ui.snap.items) counts[it.group] += 1;
  const searching = ui.query.trim() !== "";
  const dots = counts.map((n, i) => el("button", {
    class: `dot g${i} ${i === ui.group && !searching ? "current" : ""} ${n ? "filled" : ""}`,
    title: `${groupLabel(i)} · ${t("countN", n)}`, "aria-label": groupLabel(i),
    "aria-pressed": i === ui.group ? "true" : "false",
    onclick: () => {
      scrollToTop = true;
      ui.group = i; ui.query = ""; ui.renamingGroup = false; ui.editing = null; ui.menuFor = null;
      render();
    },
  }));
  let label;
  if (ui.renamingGroup) {
    const done = async (save, value) => {
      ui.renamingGroup = false;
      if (save) await call("set_group_name", { group: ui.group, label: value });
      await refresh();
    };
    label = el("input", { type: "text", class: "group-name-input", id: "group-name", maxlength: "24",
      value: ui.snap.group_names[ui.group], placeholder: t("groupN", ui.group + 1),
      onkeydown: (e) => {
        if (e.key === "Enter") done(true, e.target.value);
        if (e.key === "Escape") { e.stopPropagation(); done(false); }
      },
      onblur: (e) => { if (ui.renamingGroup) done(true, e.target.value); } });
  } else {
    label = el("button", { class: `group-name g${ui.group}`, title: t("renameGroup"),
      text: searching ? t("allGroups") : groupLabel(ui.group),
      disabled: searching,
      onclick: () => { ui.renamingGroup = true; render(); requestAnimationFrame(() => {
        const box = document.getElementById("group-name"); if (box) { box.focus(); box.select(); } }); } });
  }
  return el("div", { class: "groups" }, el("div", { class: "dots" }, dots), label);
}

// 수정 패널의 묶음 고르기: 누르면 바로 옮긴다
function groupPicker(item) {
  return el("div", { class: "dots small" }, ui.snap.group_names.map((_, i) => el("button", {
    class: `dot g${i} ${i === item.group ? "current filled" : ""}`, title: groupLabel(i), "aria-label": groupLabel(i),
    onclick: async () => {
      if (i === item.group) return;
      const { ok } = await call("set_group", { name: item.name, group: i });
      if (ok) { ui.editing = null; setMessage(t("movedTo", item.name, groupLabel(i))); }
      await refresh();
    },
  })));
}

function grantMenu(item) {
  const choose = (hours) => async () => {
    ui.menuFor = null;
    await call("set_grant", { name: item.name, hours });
    await refresh();
  };
  return el("div", { class: "menu", role: "menu" },
    el("button", { role: "menuitem", text: t("askEveryTime"), onclick: choose(null) }),
    el("button", { role: "menuitem", text: t("allow8h"), onclick: choose(8) }),
    el("button", { role: "menuitem", text: t("allowAlways"), onclick: choose(0) }));
}

function editPanel(item) {
  const d = ui.draft;
  const save = async () => {
    const { ok } = await call("save_edit", { original: item.name, name: d.name, value: d.value, note: d.note });
    if (ok) { ui.editing = null; d.value = ""; }
    await refresh();
  };
  const nameInput = el("input", { type: "text", class: "mono", value: d.name, placeholder: "NAME", spellcheck: "false",
    oninput: (e) => { d.name = e.target.value; saveBtn.disabled = !d.name.trim(); } });
  const valueInput = secretInput(d.value, t("newValuePlaceholder"), (e) => { d.value = e.target.value; });
  const noteInput = el("input", { type: "text", value: d.note, placeholder: t("notePlaceholder"),
    oninput: (e) => { d.note = e.target.value; }, onkeydown: (e) => { if (e.key === "Enter") save(); } });
  const saveBtn = el("button", { class: "btn primary", text: t("save"), onclick: save, disabled: !d.name.trim() });
  return el("div", { class: "edit" },
    el("div", { class: "line" }, el("span", { text: t("name") }), nameInput),
    el("div", { class: "line" }, el("span", { text: t("value") }), valueInput),
    el("div", { class: "line" }, el("span", { text: t("note") }), noteInput),
    el("div", { class: "line" }, el("span", { text: t("group") }), groupPicker(item)),
    el("div", { class: "line" },
      el("span", { class: "hint", text: t("editAuthHint"), style: "width:auto" }),
      el("span", { class: "spacer" }),
      el("button", { class: "btn", text: t("cancel"), onclick: () => { ui.editing = null; d.value = ""; render(); } }),
      saveBtn));
}

// 고르기 모드의 줄: 줄 아무 곳이나 누르면 체크가 바뀐다 (값·버튼은 숨김)
function selectRow(item) {
  const checked = ui.selected.has(item.name);
  return el("label", { class: `row selectable ${checked ? "selected" : ""}` },
    el("div", { class: "row-main" },
      el("input", { type: "checkbox", checked, "aria-label": item.name,
        onchange: (e) => { e.target.checked ? ui.selected.add(item.name) : ui.selected.delete(item.name); render(); } }),
      el("span", { class: `bar kind-${item.grant_kind}` }),
      el("div", { class: "ident" },
        el("span", { class: "name", title: item.name }, ...breakableName(item.name)),
        item.note ? el("span", { class: "note", text: item.note, title: item.note }) : null),
      el("span", { class: "spacer" }),
      el("span", { class: `badge kind-${item.grant_kind}`, text: item.grant_label })));
}

// JSON 이면 들여쓰기해서 보여 준다 (복사는 항상 원래 값 그대로)
function prettyValue(value) {
  const v = value.trim();
  if (v.startsWith("{") || v.startsWith("[")) {
    try { return { text: JSON.stringify(JSON.parse(v), null, 2), json: true }; } catch (_) {}
  }
  return { text: value, json: false };
}

function fullValue(value) {
  const { text, json } = prettyValue(value);
  return el("div", { class: "full-value" },
    json ? el("span", { class: "kind", text: "JSON" }) : null,
    el("pre", { text }));
}

// 값 입력칸: 여러 줄(JSON, PEM 등)도 그대로 받는다. 글자는 가려서 보인다
function secretInput(value, placeholder, oninput) {
  const box = el("textarea", { class: "secret", rows: "1", placeholder, spellcheck: "false",
    autocomplete: "off", autocorrect: "off", autocapitalize: "off",
    oninput: (e) => { grow(e.target); oninput(e); } });
  box.value = value;
  requestAnimationFrame(() => grow(box));
  return box;
}

function grow(box) {
  box.style.height = "auto";
  box.style.height = Math.min(box.scrollHeight + 2, 120) + "px";
  fit();
}

function row(item) {
  if (ui.selecting) return selectRow(item);
  const revealed = ui.revealed.has(item.name);
  const value = revealed ? ui.revealed.get(item.name) : "••••••••";
  // 보이는 값은 길이와 상관없이 모두 펼쳐 볼 수 있다 (좁은 칸에서 잘리므로)
  const expanded = revealed && ui.expanded.has(item.name);
  const badge = el("button", {
    class: `badge kind-${item.grant_kind}`, title: t("grantHelp"),
    "aria-haspopup": "menu", "aria-expanded": ui.menuFor === item.name ? "true" : "false",
    text: item.grant_label,
    onclick: (e) => { e.stopPropagation(); ui.menuFor = ui.menuFor === item.name ? null : item.name; render(); },
  });
  const anchor = el("div", { class: "menu-anchor" }, badge, ui.menuFor === item.name ? grantMenu(item) : null);

  const main = el("div", { class: "row-main" },
    el("span", { class: `bar kind-${item.grant_kind}` }),
    ui.query.trim() ? el("span", { class: `mini-dot g${item.group}`, title: groupLabel(item.group) }) : null,
    el("div", { class: "ident" },
      el("span", { class: "name", title: item.name }, ...breakableName(item.name)),
      item.note ? el("span", { class: "note", text: item.note, title: item.note }) : null),
    el("button", { class: "value", text: revealed ? value.replace(/\s*\n\s*/g, " ") : value, title: revealed ? t("clickToHide") : t("clickToShow"),
      onclick: () => toggleReveal(item.name) }),
    anchor,
    el("div", { class: "actions" },
      revealed ? iconButton(expanded ? "collapse" : "expand", expanded ? t("collapseValue") : t("expandValue"), () => {
        expanded ? ui.expanded.delete(item.name) : ui.expanded.add(item.name);
        render();
      }, expanded ? "active" : "") : null,
      iconButton(revealed ? "eyeSlash" : "eye", revealed ? t("hideValue") : t("showValue"),
        () => toggleReveal(item.name), `eye ${revealed ? "active" : ""}`),
      iconButton("copy", t("copy"), async () => {
        const { ok, result } = await callQuiet("copy", { name: item.name });
        if (ok) { setMessage(""); toast(result); }
        await refresh();
      }),
      iconButton("pencil", t("edit"), () => {
        if (ui.editing === item.name) { ui.editing = null; ui.draft.value = ""; }
        else { ui.editing = item.name; ui.draft = { name: item.name, value: "", note: item.note || "" }; }
        render();
      }, ui.editing === item.name ? "active" : ""),
      iconButton("trash", t("remove"), async () => {
        const { ok } = await call("remove", { name: item.name });
        if (ok) ui.revealed.delete(item.name);
        await refresh();
      }, "danger")));

  return el("div", { class: `row ${revealed ? "revealed" : ""}` },
    main, expanded ? fullValue(value) : null, ui.editing === item.name ? editPanel(item) : null);
}

function addCard() {
  const a = ui.add;
  const submit = async () => {
    const { ok } = await call("add", { name: a.name.trim(), value: a.value, note: a.note, group: ui.group });
    if (ok) { a.name = ""; a.value = ""; a.note = ""; }
    await refresh();
  };
  const update = () => { addBtn.disabled = !(a.name.trim() && a.value); };
  const addBtn = el("button", { class: "btn primary", text: t("add"), onclick: submit, disabled: !(a.name.trim() && a.value) });
  return el("div", { class: "add" },
    el("div", { class: "label" }, el("span", { icon: "plus" }), el("span", { text: t("newSecret") }),
      el("span", { class: `mini-dot g${ui.group}` }), el("span", { class: "in-group", text: groupLabel(ui.group) })),
    el("div", { class: "line" },
      el("input", { type: "text", class: "mono name", placeholder: "NAME", value: a.name, spellcheck: "false",
        oninput: (e) => { a.name = e.target.value; update(); } }),
      secretInput(a.value, t("value"), (e) => { a.value = e.target.value; update(); })),
    el("div", { class: "line" },
      el("input", { type: "text", placeholder: t("noteOptional"), value: a.note,
        oninput: (e) => { a.note = e.target.value; },
        onkeydown: (e) => { if (e.key === "Enter" && !addBtn.disabled) submit(); } }),
      addBtn));
}

function exitSelecting() {
  ui.selecting = false;
  ui.selected.clear();
}

// 고르기 모드의 바닥글: 전체 선택 / N개 선택됨 / 취소 / 복사(매번 인증)
function selectionBar() {
  const names = visibleItems().map((i) => i.name);
  const all = names.length > 0 && names.every((n) => ui.selected.has(n));
  const copy = async () => {
    // 검색으로 가려진 것도 고른 것은 포함한다 (목록 순서대로)
    const chosen = ui.snap.items.map((i) => i.name).filter((n) => ui.selected.has(n));
    const { ok, result } = await callQuiet("copy_many", { names: chosen });
    if (ok) { setMessage(""); toast(result); exitSelecting(); }
    await refresh();
  };
  return el("div", { class: "footer selection-bar" },
    el("button", { class: "btn", text: all ? t("clearAll") : t("selectAll"),
      onclick: () => { names.forEach((n) => (all ? ui.selected.delete(n) : ui.selected.add(n))); render(); } }),
    el("span", { class: "count", text: t("selectedCount", ui.selected.size) }),
    el("span", { class: "spacer" }),
    el("button", { class: "btn", text: t("cancel"), onclick: () => { exitSelecting(); render(); } }),
    el("button", { class: "btn primary", text: t("copySelected"), title: t("copySelectedHelp"),
      disabled: ui.selected.size === 0, onclick: copy }));
}

function footer() {
  if (ui.selecting) return selectionBar();
  return el("div", { class: "footer" },
    el("span", { class: `message ${ui.messageIsError ? "error" : ""}`, text: ui.message }),
    el("span", { class: "spacer" }),
    iconButton("checklist", t("selectToShare"), () => {
      ui.selecting = true; ui.editing = null; ui.menuFor = null; ui.revealed.clear(); render();
    }),
    iconButton("refresh", t("refresh"), refresh),
    iconButton("gear", t("settings"), () => invoke("open_settings")));
}

function render() {
  const s = ui.snap;
  if (!s) return;
  // 입력 중인 칸의 포커스와 커서를 다시 그린 뒤에도 유지
  const active = document.activeElement;
  const fields = () => [...document.querySelectorAll("input, textarea")];
  const focusKey = active && (active.tagName === "INPUT" || active.tagName === "TEXTAREA") ? fields().indexOf(active) : -1;
  const caret = focusKey >= 0 ? active.selectionStart : null;
  // 다시 그려도 목록의 스크롤 위치는 그대로
  const oldList = document.querySelector("#root > .list");
  listScroll = oldList && !scrollToTop ? oldList.scrollTop : 0;
  scrollToTop = false;

  const items = visibleItems();
  const emptyText = ui.query.trim() ? t("noMatches", ui.query.trim()) : s.items.length ? t("emptyGroup") : t("noSecrets");
  const list = items.length
    ? el("div", { class: "list" }, items.map(row))
    : el("div", { class: "list" }, el("div", { class: "empty", text: emptyText }));
  const search = s.items.length ? searchBox() : null;
  const hint = ui.selecting ? el("div", { class: "select-hint", text: t("selectHint") }) : null;
  const parts = ui.selecting
    ? [header(s), groupDots(), hint, search, list, footer()]
    : [header(s), groupDots(), search, list, addCard(), footer()];
  document.getElementById("root").replaceChildren(...parts);

  if (focusKey >= 0) {
    const input = fields()[focusKey];
    if (input) { input.focus(); try { input.setSelectionRange(caret, caret); } catch (_) {} }
  }
  fit();
}

let lastHeight = 0;
let listScroll = 0;
let scrollToTop = false; // 묶음이나 검색어가 바뀌면 맨 위부터
// 내용에 맞출 때의 최대 높이: 이보다 길면 목록만 스크롤되고 새 시크릿 칸과 아래 버튼은 그대로 보인다
const MAX_FIT_HEIGHT = 900;

function restoreListScroll() {
  const list = document.querySelector("#root > .list");
  if (list && listScroll) list.scrollTop = listScroll;
}

function fit() {
  // 크기를 직접 정해 두었으면 그 크기 그대로 (목록이 안에서 스크롤된다). 창이 아직 그 크기가 아니면 맞춘다
  if (document.documentElement.classList.contains("fixed-size")) {
    const size = ui.snap && ui.snap.popover_size;
    if (!resizing && size && (Math.abs(window.innerWidth - size.width) > 1 || Math.abs(window.innerHeight - size.height) > 1)) {
      invoke("fit_popover", { height: size.height });
    }
    restoreListScroll();
    return;
  }
  requestAnimationFrame(() => {
    const root = document.getElementById("root");
    const list = root.querySelector(".list");
    let h = Math.ceil(root.getBoundingClientRect().height);
    const max = Math.min(MAX_FIT_HEIGHT, Math.max(320, window.screen.availHeight - 16));
    if (list && h > max) {
      // 넘치는 만큼 목록 높이만 줄인다
      list.style.maxHeight = `${Math.max(80, list.getBoundingClientRect().height - (h - max))}px`;
      list.classList.add("capped");
      h = Math.ceil(root.getBoundingClientRect().height);
    }
    restoreListScroll();
    if (h && Math.abs(h - lastHeight) > 1) {
      lastHeight = h;
      invoke("fit_popover", { height: h });
    }
  });
}

// ---- 마우스로 크기 조절 ----
// 오른쪽 아래 모서리(너비·높이)와 아래 가장자리(높이)를 끌면 크기가 바뀌고, 놓으면 설정에 저장돼 다음에도 유지된다.
// 모서리를 두 번 누르면 다시 내용에 맞춘다.
let resizing = false;
function setFixedSize(on) {
  document.documentElement.classList.toggle("fixed-size", on);
  if (!on) { lastHeight = 0; fit(); }
}

function resizeHandle(kind) {
  const handle = el("div", { class: `resize-${kind}`, title: t("resizeHint"), "aria-hidden": "true" });
  let start = null, frame = 0, next = null;
  const send = (done) => invoke("resize_popover", { width: next.width, height: next.height, done });
  handle.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    resizing = true;
    setFixedSize(true);
    // 창의 위쪽·왼쪽은 그대로라 창 안 좌표(client)로 계산한다 (screenX 는 여러 모니터에서 어긋남)
    start = { x: e.clientX, y: e.clientY, width: window.innerWidth, height: window.innerHeight };
  });
  handle.addEventListener("pointermove", (e) => {
    if (!start) return;
    next = {
      width: kind === "corner" ? start.width + (e.clientX - start.x) : start.width,
      height: start.height + (e.clientY - start.y),
    };
    if (!frame) frame = requestAnimationFrame(() => { frame = 0; send(false); });
  });
  const end = async () => {
    if (!start) return;
    start = null;
    if (frame) { cancelAnimationFrame(frame); frame = 0; }
    if (next) { await send(true); next = null; }
    resizing = false;
    await refresh();
  };
  handle.addEventListener("pointerup", end);
  handle.addEventListener("pointercancel", end);
  if (kind === "corner") {
    handle.addEventListener("dblclick", async () => {
      await invoke("reset_popover_size");
      await refresh();
    });
  }
  return handle;
}
document.body.append(resizeHandle("bottom"), resizeHandle("corner"));

// 메뉴 바깥을 누르면 닫기, Esc 로 팝오버 닫기
document.addEventListener("click", () => { if (ui.menuFor) { ui.menuFor = null; render(); } });
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    if (ui.menuFor) { ui.menuFor = null; render(); }
    else if (ui.selecting) { exitSelecting(); render(); }
    else invoke("hide_popover");
  }
});

listen("akey://changed", refresh);
listen("akey://hidden", () => {
  // 창을 닫으면 보이던 값과 수정 중인 값은 지운다
  ui.revealed.clear();
  ui.expanded.clear();
  ui.editing = null;
  ui.draft = { name: "", value: "", note: "" };
  ui.menuFor = null;
  ui.query = "";
  ui.renamingGroup = false;
  scrollToTop = true;
  exitSelecting();
  render();
});

// ⌘F / Ctrl+F 또는 / 로 검색칸에 바로 가기
document.addEventListener("keydown", (e) => {
  const box = document.getElementById("search");
  if (!box) return;
  const typing = document.activeElement && document.activeElement.tagName === "INPUT";
  if (((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "f") || (e.key === "/" && !typing)) {
    e.preventDefault();
    box.focus();
    box.select();
  }
});

refresh();
