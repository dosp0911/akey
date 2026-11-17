// 설정 창: 언어, 잠금 해제 유지 시간, 클립보드 숨김, 로그인 시 자동 실행, 에이전트 전체 접근
import { t, setLang } from "./i18n.js";

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let autostartError = "";
let agentError = "";

function el(tag, attrs = {}, ...children) {
  const node = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (v === undefined || v === null || v === false) continue;
    if (k === "class") node.className = v;
    else if (k === "text") node.textContent = v;
    else if (k.startsWith("on")) node.addEventListener(k.slice(2), v);
    else node.setAttribute(k, v === true ? "" : v);
  }
  for (const c of children.flat()) if (c) node.append(c);
  return node;
}

function select(id, options, value, onchange) {
  const s = el("select", { id, onchange: (e) => onchange(e.target.value) },
    options.map(([v, label]) => el("option", { value: v, text: label, selected: String(v) === String(value) })));
  return s;
}

async function render() {
  const s = await invoke("get_settings");
  setLang(s.lang);
  document.title = t("settingsTitle");

  const general = el("section", { class: "group" },
    el("div", { class: "setting" },
      el("label", { for: "lang", text: t("language") }),
      select("lang", [["ko", "한국어"], ["en", "English"], ["zh", "简体中文"]], s.language, async (v) => {
        await invoke("set_language", { code: v });
      })),
    el("div", { class: "help", text: t("languageHelp") }));

  const security = el("section", { class: "group" },
    el("div", { class: "setting" },
      el("label", { for: "unlock", text: t("stayUnlocked") }),
      select("unlock", s.unlock_choices.map((m) => [m, t("minutes", m)]), s.unlock_minutes, async (v) => {
        await invoke("set_unlock_minutes", { minutes: Number(v) });
      })),
    el("div", { class: "help", text: t("stayUnlockedHelp") }),
    el("div", { class: "setting" },
      el("label", { for: "conceal", text: t("conceal") }),
      el("input", { id: "conceal", type: "checkbox", checked: s.conceal_clipboard,
        onchange: async (e) => { await invoke("set_conceal_clipboard", { on: e.target.checked }); } })),
    el("div", { class: "help", text: t("concealHelp") }),
    el("div", { class: "setting" },
      el("label", { for: "autostart", text: t("autostart") }),
      el("input", { id: "autostart", type: "checkbox", checked: s.autostart,
        onchange: async (e) => {
          try { await invoke("set_autostart", { on: e.target.checked }); autostartError = ""; }
          catch (err) { autostartError = String(err); }
          render();
        } })),
    autostartError ? el("div", { class: "help error", text: autostartError }) : null);

  // 전체 허용: 꺼짐(시크릿별 설정) / 8시간 / 항상. '꺼짐'은 전체 허용만 끄고 시크릿별 허용은 그대로 둔다
  const current = s.all_kind === "ask" ? "off" : s.all_kind;
  const runAgent = async (cmd, args) => {
    try { await invoke(cmd, args); agentError = ""; }
    catch (err) { agentError = String(err); }
    render();
  };
  const agent = el("section", { class: "group" },
    el("div", { class: "setting" },
      el("label", { for: "agent", text: t("agentAccess") }),
      select("agent", [
        ["off", t("allOff")],
        ["timed", s.all_kind === "timed" ? `${t("allowAll8h")} · ${s.all_label}` : t("allowAll8h")],
        ["always", t("allowAllAlways")],
      ], current, (v) => {
        if (v === current) return;
        runAgent("set_all_grant", { hours: v === "timed" ? 8 : v === "always" ? 0 : null });
      })),
    el("div", { class: "help", text: t("agentAccessHelp") }),
    el("div", { class: "setting" },
      el("label", { text: t("individualCount", s.individual_count) }),
      el("button", { class: "btn", text: t("revokeAllButton"),
        disabled: s.individual_count === 0 && s.all_kind === "ask",
        onclick: () => runAgent("revoke_all_grants", {}) })),
    el("div", { class: "help", text: t("revokeAllHelp") }),
    agentError ? el("div", { class: "help error", text: agentError }) : null);

  const children = [general, security, agent];
  if (s.windows_hello) children.push(el("div", { class: "help warning", text: t("helloWarning") }));
  document.getElementById("root").replaceChildren(...children);
  fit();
}

function fit() {
  requestAnimationFrame(() => {
    const h = Math.ceil(document.getElementById("root").getBoundingClientRect().height);
    const { getCurrentWindow } = window.__TAURI__.window;
    const { LogicalSize } = window.__TAURI__.dpi;
    if (h) getCurrentWindow().setSize(new LogicalSize(480, h));
  });
}

listen("akey://changed", render);
render();
