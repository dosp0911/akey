# akey

[English](README.md) | **한국어** | [简体中文](README.zh-CN.md)

AI 코딩 에이전트(Claude Code, Codex, Cursor 등)가 쓰는 API 키와 토큰을 **파일에 두지 않고** 이 컴퓨터의 안전한 저장소에 보관하는 작은 메뉴바(트레이) 앱 + CLI.

- **macOS**: 로그인 키체인에 저장하고, Touch ID(또는 로그인 비밀번호)로 확인한다.
- **Windows**: 사용자 계정 암호화(DPAPI)와 Windows Hello(PIN, 지문, 얼굴)로 보호한다.
- 에이전트는 `akey exec NAME -- 명령`으로 값을 **환경변수로만** 받는다. 값이 대화 기록이나 `.env` 파일에 남지 않고, 명령이 값을 출력해도 `***`로 가려진다.

<p align="center">
  <img src="docs/images/list-ko.png" width="400" alt="메뉴바의 시크릿 목록">
  <img src="docs/images/select-ko.png" width="400" alt="여러 개 골라 .env로 복사">
</p>
<p align="center">
  <img src="docs/images/settings-ko.png" width="370" alt="설정 창">
</p>
<p align="center"><sub>스크린샷은 실제 키가 아닌 예시 데이터입니다.</sub></p>

## 설치

### macOS

```bash
brew tap dosp0911/akey
brew trust dosp0911/akey
brew install dosp0911/akey/akey
```

Homebrew는 직접 신뢰(`brew trust`)한 외부 tap의 Cask만 불러온다.

또는

```bash
curl -fsSL https://github.com/dosp0911/akey/releases/latest/download/install.sh | sh
```

### Windows

```powershell
winget install akey
```

또는 Scoop

```powershell
scoop bucket add akey https://github.com/dosp0911/scoop-akey
scoop install akey
```

또는 PowerShell

```powershell
irm https://github.com/dosp0911/akey/releases/latest/download/install.ps1 | iex
```

설치하면 Mac은 `/Applications/akey.app`과 `akey` 명령이, Windows는 `%LOCALAPPDATA%\Programs\akey`에 `akey.exe`(CLI)와 `akey-app.exe`(트레이 앱)가 생긴다.

> **서명 안내**: 지금은 Apple Developer ID 서명이 없는 빌드다. brew와 설치 스크립트가 Gatekeeper 격리 표시를 지워 준다. 업데이트한 뒤 처음 열 때 키체인 비밀번호를 한 번 물으면 **"항상 허용"**을 누른다.

> **받은 파일 확인**: 설치 스크립트는 같은 릴리스의 `checksums.txt`와 비교한다. 다운로드 중 깨진 파일은 걸러지지만, 릴리스 파일과 체크섬이 함께 바뀐 경우는 막지 못한다. 모든 릴리스 파일에는 GitHub 빌드 출처 증명(attestation)이 붙어 있어, 이 저장소의 릴리스 워크플로가 만든 파일인지 확인할 수 있다.
>
> ```bash
> gh attestation verify akey-macos-universal.tar.gz -R dosp0911/akey
> ```

## 사용

메뉴바(트레이) 아이콘 사용법:
- **왼쪽 클릭**: 시크릿 목록이 열린다. 값은 가려져 있고, 값이나 👁를 누르면 인증 뒤 보인다.
- **오른쪽 클릭**: 정보 / 설정 / 종료.
- 한 번 인증하면 설정한 시간(기본 30분) 동안 보기와 복사를 다시 묻지 않는다.
- 삭제, 수정(이름·값), 허용 변경은 매번 묻는다.

### 여러 개 골라 공유하기

동료에게 키 묶음을 넘기거나 다른 컴퓨터를 설정할 때 쓴다.

1. 목록 아래쪽의 **여러 개 골라 복사** 버튼(체크리스트 아이콘)을 누른다.
2. 공유할 시크릿을 체크하거나 **전체 선택**을 누른다. 줄 아무 곳이나 눌러도 체크된다.
3. **복사**를 누른다. 잠금 해제 상태여도 **매번 인증**을 묻고, `.env` 형식으로 클립보드에 복사한다.

```dotenv
ANTHROPIC_API_KEY=sk-ant-...
DATABASE_URL=postgres://dev:dev@localhost:5432/app
GREETING="hello world"
```

공백, 따옴표, `$`, 백틱, 줄바꿈이 있는 값은 따옴표로 감싸고 이스케이프해서, 그대로 `.env` 파일에 붙여 넣거나 `source`로 읽어도 안전하다. 공개 채널이 아닌 안전한 경로(비밀번호 관리자의 공유 기능, 종단 간 암호화 메신저 등)로 보낸다.

CLI:

```text
akey list                         이름, 허용 상태, 메모 (값은 표시 안 함)
akey exec NAME... -- CMD...       환경변수로 주입해 CMD 실행 (에이전트 권장, 출력의 값은 ***)
akey get NAME                     값 출력
akey set NAME [--note 메모]       저장/수정 (값은 숨김 입력 또는 stdin)
akey note NAME [메모|--clear]     메모 보기/쓰기/지우기
akey rm NAME                      삭제
akey mv OLD NEW                   이름 바꾸기 (값·메모·허용 유지)
akey allow NAME|--all [--hours N] 에이전트가 인증 없이 읽도록 허용 (N 생략 시 무기한)
akey deny NAME|--all              허용 해제
```

인증 규칙:
- **인증 없음**: 새 이름으로 `set` 하기, `list`, `deny`
- **허용 안 된 시크릿만 인증**: `get`, `exec`
- **항상 인증**: `allow`, `rm`, `mv`, 덮어쓰기, 기존 메모 바꾸기
- 인증이 거부되면 종료 코드는 `77`이다.

### 에이전트에게 알려 주기

에이전트의 전역 지침 파일에 아래 내용을 넣으면 로컬 시크릿을 akey로만 읽고 쓰고 관리한다.

```markdown
## Secrets
Local secrets (API keys, tokens, passwords) live in `akey`. Never put them in files, `.env`, commands, or output.
- List: `akey list`
- Use: `akey exec NAME... -- <cmd>` (not `akey get`)
- Add: `printf %s "$VALUE" | akey set NAME --note "<purpose>"`
- Found a plaintext secret? Move it into akey. Missing one? Ask me.
- Exit 77 = I declined; don't retry.
```

이 내용은 Claude Code(`~/.claude/CLAUDE.md`)와 Codex 등 `AGENTS.md`를 읽는 도구(`~/.codex/AGENTS.md`)에 한 번에 추가할 수 있다. 이미 들어 있으면 다시 넣지 않는다.

macOS / Linux

```bash
for f in ~/.claude/CLAUDE.md ~/.codex/AGENTS.md; do mkdir -p "$(dirname "$f")"; grep -qs 'akey exec' "$f" || cat >> "$f" <<'EOF'

## Secrets
Local secrets (API keys, tokens, passwords) live in `akey`. Never put them in files, `.env`, commands, or output.
- List: `akey list`
- Use: `akey exec NAME... -- <cmd>` (not `akey get`)
- Add: `printf %s "$VALUE" | akey set NAME --note "<purpose>"`
- Found a plaintext secret? Move it into akey. Missing one? Ask me.
- Exit 77 = I declined; don't retry.
EOF
done
```

Windows (PowerShell)

```powershell
$s = @'

## Secrets
Local secrets (API keys, tokens, passwords) live in `akey`. Never put them in files, `.env`, commands, or output.
- List: `akey list`
- Use: `akey exec NAME... -- <cmd>` (not `akey get`)
- Add: `printf %s "$VALUE" | akey set NAME --note "<purpose>"`
- Found a plaintext secret? Move it into akey. Missing one? Ask me.
- Exit 77 = I declined; don't retry.
'@
foreach ($f in "$HOME\.claude\CLAUDE.md", "$HOME\.codex\AGENTS.md") {
  New-Item -ItemType Directory -Force (Split-Path $f) | Out-Null
  if (-not ((Test-Path $f) -and (Select-String -Quiet -SimpleMatch 'akey exec' $f))) { Add-Content -Encoding utf8 $f $s }
}
```

- `get` 대신 `exec`를 쓰게 한다. `get`으로 꺼낸 값은 에이전트 대화 기록에 남는다.
- 허용·삭제(`allow`, `deny`, `rm`)는 일부러 넣지 않았다. 에이전트가 스스로 권한을 넓히지 않게 사용자만 한다.

## 보안 모델

| | macOS | Windows |
|---|---|---|
| 저장 위치 | 로그인 키체인 항목 `akey` / `vault` 하나 | `%LOCALAPPDATA%\akey\vault.json` |
| "매번 묻기" 값 | 키체인. akey 앱만 읽도록 허용됨 | Windows Hello 키로 풀리는 X25519 키로 봉인 |
| "허용" 값 | 키체인 | DPAPI (같은 사용자 계정의 프로그램이 읽을 수 있음) |
| 확인 방법 | Touch ID / 로그인 비밀번호 | Windows Hello (PIN / 지문 / 얼굴) |

- `akey exec`는 명령의 출력(표준 출력·표준 오류)에서 시크릿 값을 항상 `***`로 가린다. 값이 나눠서 출력돼도 가린다. 명령 자체는 원래 값을 쓴다. 4자 미만인 값이나 base64처럼 변형된 값은 가리지 못한다. 사람이 값을 확인할 때는 메뉴바나 `akey get`을 쓴다.
- 값은 이 컴퓨터를 떠나지 않는다(iCloud나 클라우드 동기화 없음). 백업이 필요하면 원본을 비밀번호 관리자에 따로 보관한다.
- "항상 허용"과 "에이전트 전체 접근"은 편하지만, 내 계정으로 도는 어떤 프로그램이든 인증 없이 값을 꺼낼 수 있게 된다. 평소에는 시간 제한(8시간) 허용을 권한다.
- **Windows**: Hello PIN을 재설정하거나 TPM을 초기화하면 "매번 묻기" 값은 복구할 수 없다.

## 소스에서 빌드

```bash
cd rust
npm ci                                   # Tauri CLI
cargo test -p akey-core -p akey-windows -p akey-cli
cd app && npx tauri build                # macOS: target/release/bundle/macos/akey.app
```

- `rust/crates/akey-core`: 금고 모델, 허용 계산, 설정, 한국어/영어
- `rust/crates/akey-macos`: 키체인 + Touch ID
- `rust/crates/akey-windows`: vault.json + DPAPI + Windows Hello (`engine.rs`는 운영체제와 무관하게 테스트됨)
- `rust/crates/akey-cli`: CLI
- `rust/app`, `rust/ui`: Tauri 트레이 앱과 화면

macOS 로컬 빌드는 `tauri.conf.json`의 `signingIdentity`("akey Local Signing")로 서명한다. 이 인증서가 없으면 `--config '{"bundle":{"macOS":{"signingIdentity":"-"}}}'`로 임시 서명한다.

## 릴리스 (관리자용)

1. `rust/app/tauri.conf.json`의 버전을 올리고 태그를 push한다: `git tag v0.2.0 && git push origin v0.2.0`
2. `.github/workflows/release.yml`이 macOS universal 앱과 Windows exe를 빌드해 GitHub Release에 올린다. 설치 스크립트와 `checksums.txt`도 함께 올린다.
3. 패키지 관리자는 처음 한 번 준비가 필요하다. 토큰은 모두 이 저장소의 Settings → Secrets에 넣는다.

| 채널 | 처음 한 번 | Secret | 이후 |
|---|---|---|---|
| Homebrew | `dosp0911/homebrew-akey` 저장소 만들기 | `HOMEBREW_TAP_TOKEN` (tap 저장소 쓰기 권한) | 릴리스마다 `Casks/akey.rb` 자동 갱신 |
| Scoop | `dosp0911/scoop-akey` 저장소 만들기 | `SCOOP_BUCKET_TOKEN` (bucket 저장소 쓰기 권한) | 릴리스마다 `bucket/akey.json` 자동 갱신 |
| winget | 첫 릴리스의 `winget-manifests.zip`을 풀어 [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs)의 `manifests/a/akey/akey/<버전>/`에 넣고 PR을 올린다 (패키지 ID `akey.akey`) (`wingetcreate submit <폴더>`로도 가능) | `WINGET_TOKEN` (`public_repo` 권한 PAT) | 등록된 뒤 릴리스마다 업데이트 PR을 자동으로 보냄 |

Secret이 없으면 해당 단계는 건너뛴다. winget 심사는 Microsoft가 하며 보통 며칠 걸린다. 서명되지 않은 exe는 SmartScreen 경고와 함께 추가 검토를 받을 수 있다.

## 라이선스

[MIT](LICENSE)
