// 화면 문구 (한국어 / English). Rust 쪽 문구는 akey-core 의 tr() 에 있다.
const STRINGS = {
  noSecrets: ["저장된 시크릿이 없습니다", "No secrets yet"],
  locked: ["잠김", "Locked"],
  unlocked: ["잠금 해제됨", "Unlocked"],
  lockedHelp: ["눌러서 모든 값 보기 (인증 후 {0}분간 잠금 해제)", "Click to show every value (authenticate, then stay unlocked for {0} min)"],
  unlockedHelp: ["눌러서 모든 값을 가리고 다시 잠그기", "Click to hide every value and lock again"],
  agentOff: ["에이전트: 시크릿별", "Agents: per secret"],
  agent8h: ["에이전트: 8시간 전체", "Agents: all for 8h"],
  agentAlways: ["에이전트: 항상 전체", "Agents: all, always"],
  groupN: ["묶음 {0}", "Group {0}"],
  countN: ["{0}개", "{0}"],
  renameGroup: ["눌러서 묶음 이름 바꾸기", "Click to rename this group"],
  allGroups: ["모든 묶음에서 검색", "Searching all groups"],
  emptyGroup: ["이 묶음은 비어 있습니다", "This group is empty"],
  group: ["묶음", "Group"],
  movedTo: ["{0} → {1}", "{0} → {1}"],
  clickToShow: ["눌러서 값 보기", "Click to show"],
  clickToHide: ["눌러서 값 숨기기", "Click to hide"],
  showValue: ["값 보기", "Show value"],
  hideValue: ["값 숨기기", "Hide value"],
  expandValue: ["값 전체 보기", "Show the whole value"],
  collapseValue: ["접기", "Collapse"],
  copy: ["복사", "Copy"],
  edit: ["수정 (이름·값·메모)", "Edit (name, value, note)"],
  remove: ["삭제", "Delete"],
  grantHelp: ["에이전트(CLI)가 인증 없이 읽을 수 있는지", "Whether agents (CLI) can read it without asking"],
  askEveryTime: ["매번 묻기", "Ask every time"],
  allow8h: ["8시간 허용", "Allow for 8 hours"],
  allowAlways: ["항상 허용", "Always allow"],
  name: ["이름", "Name"],
  value: ["값", "Value"],
  note: ["메모", "Note"],
  newValuePlaceholder: ["새 값 (바꾸지 않으려면 비워두세요)", "New value (leave empty to keep)"],
  notePlaceholder: ["예: 개인 OpenAI 키, 2026-12 만료", "e.g. personal OpenAI key, expires 2026-12"],
  editAuthHint: ["이름이나 값을 바꾸면 인증을 묻습니다", "Changing the name or value asks for authentication"],
  cancel: ["취소", "Cancel"],
  save: ["저장", "Save"],
  newSecret: ["새 시크릿", "New secret"],
  noteOptional: ["메모 (선택)", "Note (optional)"],
  add: ["추가", "Add"],
  refresh: ["새로고침", "Refresh"],
  settings: ["설정", "Settings"],
  selectToShare: ["여러 개 골라 복사 (공유용)", "Select several to copy (for sharing)"],
  selectHint: ["복사할 시크릿을 고르세요. .env 형식(NAME=값)으로 복사되며, 복사할 때마다 인증을 묻습니다.", "Choose secrets to copy. They're copied in .env format (NAME=value), and each copy asks for authentication."],
  selectAll: ["전체 선택", "Select all"],
  clearAll: ["선택 해제", "Clear"],
  selectedCount: ["{0}개 선택됨", "{0} selected"],
  copySelected: ["복사", "Copy"],
  copySelectedHelp: ["인증 후 .env 형식으로 클립보드에 복사", "Authenticate, then copy to the clipboard in .env format"],
  searchPlaceholder: ["검색 (이름·메모)", "Search (name, note)"],
  noMatches: ["\"{0}\" 와 맞는 시크릿이 없습니다", "No secrets match \"{0}\""],
  // 설정 창
  settingsTitle: ["Akey 설정", "Akey Settings"],
  language: ["언어", "Language"],
  languageHelp: ["메뉴바(트레이) 화면과 CLI 출력 언어", "Language for the menu bar (tray) window and CLI output"],
  stayUnlocked: ["잠금 해제 유지", "Stay unlocked for"],
  minutes: ["{0}분", "{0} min"],
  stayUnlockedHelp: ["인증 한 번으로 이 시간 동안 값 보기·복사를 다시 묻지 않습니다. 삭제·수정·허용 변경은 항상 묻습니다.", "After one authentication, viewing and copying won't ask again for this long. Delete, edit and access changes always ask."],
  conceal: ["클립보드 기록 앱에서 숨기기", "Hide from clipboard history apps"],
  concealHelp: ["복사한 값을 클립보드 기록 앱(Raycast, Paste, Windows 클립보드 기록 등)이 저장하지 않도록 표시합니다.", "Marks copied values so clipboard history apps (Raycast, Paste, Windows clipboard history and others) don't store them."],
  autostart: ["로그인 시 자동 실행", "Open at login"],
  agentAccess: ["에이전트 전체 접근", "Agent access to all secrets"],
  allowAll8h: ["8시간 전체 허용", "Allow all for 8 hours"],
  allowAllAlways: ["항상 전체 허용", "Always allow all"],
  allOff: ["꺼짐 (시크릿별 설정)", "Off (per-secret settings)"],
  individualCount: ["시크릿별 허용 {0}개", "Per-secret access: {0}"],
  revokeAllButton: ["모든 허용 해제", "Revoke all access"],
  revokeAllHelp: ["전체 허용과 시크릿별 허용을 모두 지워 모든 시크릿을 '매번 묻기'로 되돌립니다.", "Clears all-access and every per-secret access, so every secret asks again."],
  agentAccessHelp: ["켜면 모든 시크릿(앞으로 추가될 것 포함)을 에이전트(CLI)가 인증 없이 읽습니다. 바꿀 때 인증을 묻습니다.", "When on, agents (CLI) can read every secret, including future ones, without authentication. Changing it asks for authentication."],
  helloWarning: ["Windows Hello PIN 을 재설정하거나 TPM 을 초기화하면 '매번 묻기' 시크릿 값은 복구할 수 없습니다. 중요한 키는 원본을 따로 보관하세요.", "If you reset your Windows Hello PIN or clear the TPM, values of 'ask every time' secrets can't be recovered. Keep originals of important keys elsewhere."],
};

let currentLang = "ko";

export function setLang(lang) {
  currentLang = lang === "en" ? "en" : "ko";
  document.documentElement.lang = currentLang;
}

export function t(key, ...args) {
  const pair = STRINGS[key];
  if (!pair) return key;
  let text = currentLang === "en" ? pair[1] : pair[0];
  args.forEach((a, i) => { text = text.replace(`{${i}}`, a); });
  return text;
}
