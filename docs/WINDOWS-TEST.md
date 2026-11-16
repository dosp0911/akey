# Windows 테스트 확인 목록

Mac에서는 Windows용 컴파일과 금고 로직 테스트(가짜 DPAPI·Hello)까지만 확인했다. 아래 항목은 실제 Windows에서 확인해야 한다.

**준비**: Windows 10/11, Windows Hello PIN 설정. 지문이 없는 가상 머신(가상 TPM + PIN)도 된다.

**설치**: 릴리스의 `install.ps1`로 설치한다. 소스에서 빌드한다면 `cargo build --release -p akey-cli`와 `cd rust/app && npx tauri build --no-bundle`로 만든다.

테스트할 때는 `AKEY_VAULT_PATH=C:\temp\akey-test\vault.json`를 지정하면 실제 금고를 건드리지 않는다.

## 1. 설치
- [ ] `irm …/install.ps1 | iex` 로 설치되고, 체크섬을 확인한다는 문구가 나온다
- [ ] 새 터미널에서 `akey help`가 동작한다 (PATH 추가 확인)
- [ ] 시작 메뉴에 akey가 있고, 실행하면 트레이 아이콘이 생긴다
- [ ] 작업 표시줄에 창 버튼이 생기지 않는다

## 2. Windows Hello와 저장
- [ ] 처음 `printf`/`echo`로 `akey set TEST`를 하면 Hello 확인이 뜬다 (키 생성)
  - PowerShell: `"value" | akey set TEST`
- [ ] `%LOCALAPPDATA%\akey\vault.json`을 열어 봤을 때 값과 이름(TEST)이 평문으로 없다
- [ ] `akey list` → `TEST  매번 묻기`
- [ ] `akey get TEST`: Hello 확인 뒤 값이 나온다. 취소하면 종료 코드 77이다 (`$LASTEXITCODE`)
- [ ] **Hello 창이 터미널 뒤에 숨지 않고 앞에 뜬다** (알려진 위험 항목)

## 3. 허용
- [ ] `akey allow TEST --hours 1`: Hello 확인 뒤 `60분 남음`이 된다
- [ ] 그 뒤 `akey get TEST`는 묻지 않는다
- [ ] `akey deny TEST`는 묻지 않고, 그 뒤 `get`은 다시 Hello를 묻는다
- [ ] `akey allow --all --hours 1` 뒤에 새로 `set`한 시크릿도 묻지 않는다
- [ ] 허용 시간이 지나면(또는 `deny --all` 뒤) 다시 묻는다

## 4. exec
- [ ] `akey exec TEST -- powershell -c "echo $env:TEST"`: 값이 하위 프로세스에 전달된다
- [ ] `akey exec TEST -- cmd /c exit 7` → `$LASTEXITCODE`가 7이다
- [ ] `akey exec TEST -- ping -t 127.0.0.1` 중에 Ctrl+C를 누르면 ping이 멈추고 akey도 끝난다

## 5. 트레이 앱
- [ ] 왼쪽 클릭: 목록 창이 트레이 위쪽에 뜬다. 바깥을 누르면 닫힌다
- [ ] 오른쪽 클릭: akey 정보 / 설정… / 종료
- [ ] 값을 누르거나 👁를 누르면 Hello 확인 뒤 보인다. Hello 창이 떠 있는 동안 목록 창이 닫히지 않는다
- [ ] 한 번 확인한 뒤에는 설정한 시간 동안 다른 값의 보기와 복사를 묻지 않는다
- [ ] 수정(이름, 값, 메모), 삭제, 허용 배지 메뉴가 동작한다
- [ ] 복사한 값이 클립보드에 남는다. "클립보드 기록 앱에서 숨기기"를 켜면 Win+V 기록에 나타나지 않는다
- [ ] 설정: 언어 전환이 바로 반영된다 (목록, 오른쪽 클릭 메뉴, CLI 출력 모두)
- [ ] 설정: 로그인 시 자동 실행을 켜고 다시 로그인하면 트레이에 나타난다
- [ ] 다크 모드에서 글자가 잘 보인다

## 6. 복구 경고
- [ ] 설정 창 아래에 Hello 재설정 시 복구할 수 없다는 경고가 보인다
