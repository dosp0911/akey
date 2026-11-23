// 화면 문구 [한국어, English, 简体中文]. Rust 쪽 문구는 akey-core 의 tr() 에 있다.
const STRINGS = {
  noSecrets: ["저장된 시크릿이 없습니다", "No secrets yet", "还没有密钥"],
  locked: ["잠김", "Locked", "已锁定"],
  unlocked: ["잠금 해제됨", "Unlocked", "已解锁"],
  lockedHelp: ["눌러서 모든 값 보기 (인증 후 {0}분간 잠금 해제)", "Click to show every value (authenticate, then stay unlocked for {0} min)", "点击显示所有值（验证后 {0} 分钟内保持解锁）"],
  unlockedHelp: ["눌러서 모든 값을 가리고 다시 잠그기", "Click to hide every value and lock again", "点击隐藏所有值并重新锁定"],
  agentOff: ["에이전트: 시크릿별", "Agents: per secret", "助手：按密钥"],
  agent8h: ["에이전트: 8시간 전체", "Agents: all for 8h", "助手：8 小时全部"],
  agentAlways: ["에이전트: 항상 전체", "Agents: all, always", "助手：始终全部"],
  resizeHint: ["드래그해서 크기 조절 · 두 번 클릭하면 내용에 맞춤", "Drag to resize · double-click to fit the content", "拖动以调整大小 · 双击恢复为适应内容"],
  groupN: ["묶음 {0}", "Group {0}", "分组 {0}"],
  countN: ["{0}개", "{0}", "{0} 个"],
  renameGroup: ["눌러서 묶음 이름 바꾸기", "Click to rename this group", "点击重命名此分组"],
  allGroups: ["모든 묶음에서 검색", "Searching all groups", "在所有分组中搜索"],
  emptyGroup: ["이 묶음은 비어 있습니다", "This group is empty", "此分组为空"],
  group: ["묶음", "Group", "分组"],
  movedTo: ["{0} → {1}", "{0} → {1}", "{0} → {1}"],
  clickToShow: ["눌러서 값 보기", "Click to show", "点击显示"],
  clickToHide: ["눌러서 값 숨기기", "Click to hide", "点击隐藏"],
  showValue: ["값 보기", "Show value", "显示值"],
  hideValue: ["값 숨기기", "Hide value", "隐藏值"],
  expandValue: ["값 전체 보기", "Show the whole value", "显示完整的值"],
  collapseValue: ["접기", "Collapse", "收起"],
  copy: ["복사", "Copy", "复制"],
  edit: ["수정 (이름·값·메모)", "Edit (name, value, note)", "编辑（名称、值、备注）"],
  remove: ["삭제", "Delete", "删除"],
  grantHelp: ["에이전트(CLI)가 인증 없이 읽을 수 있는지", "Whether agents (CLI) can read it without asking", "助手（命令行）是否可以无需验证读取"],
  askEveryTime: ["매번 묻기", "Ask every time", "每次询问"],
  allow8h: ["8시간 허용", "Allow for 8 hours", "允许 8 小时"],
  allowAlways: ["항상 허용", "Always allow", "始终允许"],
  name: ["이름", "Name", "名称"],
  value: ["값", "Value", "值"],
  note: ["메모", "Note", "备注"],
  newValuePlaceholder: ["새 값 (바꾸지 않으려면 비워두세요)", "New value (leave empty to keep)", "新值（留空则保持不变）"],
  notePlaceholder: ["예: 개인 OpenAI 키, 2026-12 만료", "e.g. personal OpenAI key, expires 2026-12", "例如：个人 OpenAI 密钥，2026-12 到期"],
  editAuthHint: ["이름이나 값을 바꾸면 인증을 묻습니다", "Changing the name or value asks for authentication", "修改名称或值时需要验证"],
  cancel: ["취소", "Cancel", "取消"],
  save: ["저장", "Save", "保存"],
  newSecret: ["새 시크릿", "New secret", "新密钥"],
  noteOptional: ["메모 (선택)", "Note (optional)", "备注（可选）"],
  add: ["추가", "Add", "添加"],
  refresh: ["새로고침", "Refresh", "刷新"],
  settings: ["설정", "Settings", "设置"],
  selectToShare: ["여러 개 골라 복사 (공유용)", "Select several to copy (for sharing)", "选择多个复制（用于分享）"],
  selectHint: ["복사할 시크릿을 고르세요. .env 형식(NAME=값)으로 복사되며, 복사할 때마다 인증을 묻습니다.", "Choose secrets to copy. They're copied in .env format (NAME=value), and each copy asks for authentication.", "选择要复制的密钥。将以 .env 格式（NAME=值）复制，每次复制都需要验证。"],
  selectAll: ["전체 선택", "Select all", "全选"],
  clearAll: ["선택 해제", "Clear", "取消选择"],
  selectedCount: ["{0}개 선택됨", "{0} selected", "已选择 {0} 个"],
  copySelected: ["복사", "Copy", "复制"],
  copySelectedHelp: ["인증 후 .env 형식으로 클립보드에 복사", "Authenticate, then copy to the clipboard in .env format", "验证后以 .env 格式复制到剪贴板"],
  searchPlaceholder: ["검색 (이름·메모)", "Search (name, note)", "搜索（名称、备注）"],
  noMatches: ["\"{0}\" 와 맞는 시크릿이 없습니다", "No secrets match \"{0}\"", "没有与“{0}”匹配的密钥"],
  // 설정 창
  settingsTitle: ["Akey 설정", "Akey Settings", "Akey 设置"],
  language: ["언어", "Language", "语言"],
  languageHelp: ["메뉴바(트레이) 화면과 CLI 출력 언어", "Language for the menu bar (tray) window and CLI output", "菜单栏（托盘）窗口和命令行输出的语言"],
  stayUnlocked: ["잠금 해제 유지", "Stay unlocked for", "保持解锁"],
  minutes: ["{0}분", "{0} min", "{0} 分钟"],
  stayUnlockedHelp: ["인증 한 번으로 이 시간 동안 값 보기·복사를 다시 묻지 않습니다. 삭제·수정·허용 변경은 항상 묻습니다.", "After one authentication, viewing and copying won't ask again for this long. Delete, edit and access changes always ask.", "验证一次后，在这段时间内查看和复制不会再询问。删除、编辑和更改访问权限每次都会询问。"],
  conceal: ["클립보드 기록 앱에서 숨기기", "Hide from clipboard history apps", "在剪贴板历史应用中隐藏"],
  concealHelp: ["복사한 값을 클립보드 기록 앱(Raycast, Paste, Windows 클립보드 기록 등)이 저장하지 않도록 표시합니다.", "Marks copied values so clipboard history apps (Raycast, Paste, Windows clipboard history and others) don't store them.", "标记复制的值，使剪贴板历史应用（Raycast、Paste、Windows 剪贴板历史等）不保存它们。"],
  autostart: ["로그인 시 자동 실행", "Open at login", "登录时打开"],
  agentAccess: ["에이전트 전체 접근", "Agent access to all secrets", "助手访问所有密钥"],
  allowAll8h: ["8시간 전체 허용", "Allow all for 8 hours", "全部允许 8 小时"],
  allowAllAlways: ["항상 전체 허용", "Always allow all", "始终全部允许"],
  allOff: ["꺼짐 (시크릿별 설정)", "Off (per-secret settings)", "关闭（按密钥设置）"],
  individualCount: ["시크릿별 허용 {0}개", "Per-secret access: {0}", "按密钥允许：{0} 个"],
  revokeAllButton: ["모든 허용 해제", "Revoke all access", "撤销全部访问"],
  revokeAllHelp: ["전체 허용과 시크릿별 허용을 모두 지워 모든 시크릿을 '매번 묻기'로 되돌립니다.", "Clears all-access and every per-secret access, so every secret asks again.", "清除全部允许和每个密钥的允许，使所有密钥恢复“每次询问”。"],
  agentAccessHelp: ["켜면 모든 시크릿(앞으로 추가될 것 포함)을 에이전트(CLI)가 인증 없이 읽습니다. 바꿀 때 인증을 묻습니다.", "When on, agents (CLI) can read every secret, including future ones, without authentication. Changing it asks for authentication.", "开启后，助手（命令行）无需验证即可读取所有密钥（包括以后添加的）。更改时需要验证。"],
  helloWarning: ["Windows Hello PIN 을 재설정하거나 TPM 을 초기화하면 '매번 묻기' 시크릿 값은 복구할 수 없습니다. 중요한 키는 원본을 따로 보관하세요.", "If you reset your Windows Hello PIN or clear the TPM, values of 'ask every time' secrets can't be recovered. Keep originals of important keys elsewhere.", "如果重置 Windows Hello PIN 或清除 TPM，“每次询问”密钥的值将无法恢复。请另外保存重要密钥的原始值。"],
};

let currentLang = "ko";

const INDEX = { ko: 0, en: 1, zh: 2 };

export function setLang(lang) {
  currentLang = lang in INDEX ? lang : "ko";
  document.documentElement.lang = currentLang === "zh" ? "zh-CN" : currentLang;
}

export function t(key, ...args) {
  const row = STRINGS[key];
  if (!row) return key;
  let text = row[INDEX[currentLang]] ?? row[1];
  args.forEach((a, i) => { text = text.replace(`{${i}}`, a); });
  return text;
}
