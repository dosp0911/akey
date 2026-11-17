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
  if (ui.snap.error) setMessage(ui.snap.error, true);
  // 지워지거나 이름이 바뀐 항목의 보이던 값은 버린다
  const names = new Set(ui.snap.items.map((i) => i.name));
  for (const n of [...ui.revealed.keys()]) if (!names.has(n)) ui.revealed.delete(n);
  if (ui.editing && !names.has(ui.editing)) ui.editing = null;
  for (const n of [...ui.selected]) if (!names.has(n)) ui.selected.delete(n);
  render();
}

async function toggleReveal(name) {
  if (ui.revealed.has(name)) {
    ui.revealed.delete(name);
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
  const update = (e) => { ui.query = e.target.value; if (!composing) render(); };
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
  const valueInput = el("input", { type: "password", value: d.value, placeholder: t("newValuePlaceholder"),
    oninput: (e) => { d.value = e.target.value; } });
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

function row(item) {
  if (ui.selecting) return selectRow(item);
  const revealed = ui.revealed.has(item.name);
  const value = revealed ? ui.revealed.get(item.name) : "••••••••";
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
    el("button", { class: "value", text: value, title: revealed ? t("clickToHide") : t("clickToShow"),
      onclick: () => toggleReveal(item.name) }),
    anchor,
    el("div", { class: "actions" },
      iconButton(revealed ? "eyeSlash" : "eye", revealed ? t("hideValue") : t("showValue"),
        () => toggleReveal(item.name), `eye ${revealed ? "active" : ""}`),
      iconButton("copy", t("copy"), async () => { await call("copy", { name: item.name }); await refresh(); }),
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
    main, ui.editing === item.name ? editPanel(item) : null);
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
      el("input", { type: "password", placeholder: t("value"), value: a.value,
        oninput: (e) => { a.value = e.target.value; update(); } })),
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
    const { ok } = await call("copy_many", { names: chosen });
    if (ok) exitSelecting();
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
  const focusKey = active && active.tagName === "INPUT" ? [...document.querySelectorAll("input")].indexOf(active) : -1;
  const caret = focusKey >= 0 ? active.selectionStart : null;

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
    const input = document.querySelectorAll("input")[focusKey];
    if (input) { input.focus(); try { input.setSelectionRange(caret, caret); } catch (_) {} }
  }
  fit();
}

let lastHeight = 0;
function fit() {
  requestAnimationFrame(() => {
    const h = Math.ceil(document.getElementById("root").getBoundingClientRect().height);
    if (h && Math.abs(h - lastHeight) > 1) {
      lastHeight = h;
      invoke("fit_popover", { height: h });
    }
  });
}

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
  ui.editing = null;
  ui.draft = { name: "", value: "", note: "" };
  ui.menuFor = null;
  ui.query = "";
  ui.renamingGroup = false;
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
