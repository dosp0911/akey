#!/bin/sh
# akey 설치 (macOS)
#   curl -fsSL https://github.com/dosp0911/akey/releases/latest/download/install.sh | sh
#
# 하는 일: 최신 릴리스의 Akey.app 을 받아 체크섬을 확인하고 /Applications(쓸 수 없으면 ~/Applications)에 설치,
#          Gatekeeper 격리 표시를 지우고, CLI 를 ~/.local/bin/akey 로 연결한다.
# 환경변수: AKEY_VERSION=v0.2.0 (특정 버전), AKEY_APP_DIR=/경로 (앱 설치 위치), AKEY_BIN_DIR=/경로 (CLI 링크 위치)
set -eu

REPO="${AKEY_REPO:-dosp0911/akey}"
ASSET="akey-macos-universal.tar.gz"

say() { printf '%s\n' "$*"; }
fail() { printf 'akey 설치 실패: %s\n' "$*" >&2; exit 1; }

[ "$(uname -s)" = "Darwin" ] || fail "macOS 전용 스크립트입니다. Windows 는 install.ps1 을 쓰세요."
command -v curl >/dev/null || fail "curl 이 필요합니다"
command -v shasum >/dev/null || fail "shasum 이 필요합니다"

if [ -n "${AKEY_BASE_URL:-}" ]; then
  BASE="$AKEY_BASE_URL"   # 테스트·미러용
elif [ -n "${AKEY_VERSION:-}" ]; then
  BASE="https://github.com/$REPO/releases/download/$AKEY_VERSION"
else
  BASE="https://github.com/$REPO/releases/latest/download"
fi

if [ -n "${AKEY_APP_DIR:-}" ]; then
  APP_DIR="$AKEY_APP_DIR"
elif [ -w /Applications ]; then
  APP_DIR="/Applications"
else
  APP_DIR="$HOME/Applications"
fi
BIN_DIR="${AKEY_BIN_DIR:-$HOME/.local/bin}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

say "akey 를 받는 중: $BASE/$ASSET"
curl -fsSL "$BASE/$ASSET" -o "$TMP/$ASSET" || fail "다운로드 실패"
curl -fsSL "$BASE/checksums.txt" -o "$TMP/checksums.txt" || fail "체크섬 파일 다운로드 실패"

EXPECTED="$(grep " $ASSET\$" "$TMP/checksums.txt" | cut -d' ' -f1)"
ACTUAL="$(shasum -a 256 "$TMP/$ASSET" | cut -d' ' -f1)"
[ -n "$EXPECTED" ] && [ "$EXPECTED" = "$ACTUAL" ] || fail "체크섬이 맞지 않습니다 (받은 파일이 손상되었거나 바뀌었습니다)"
say "체크섬 확인됨"

tar -xzf "$TMP/$ASSET" -C "$TMP"
[ -d "$TMP/Akey.app" ] || fail "압축 파일에 Akey.app 이 없습니다"

# 실행 중이면 종료 (금고는 키체인에 있어 지워지지 않음). AKEY_NO_LAUNCH=1 이면 건드리지 않음
[ -n "${AKEY_NO_LAUNCH:-}" ] || pkill -f "[Aa]key.app/Contents/MacOS/akey$" 2>/dev/null || true

mkdir -p "$APP_DIR"
# 예전 이름(akey.app)도 지운다 (대소문자를 구분하는 볼륨이면 둘 다 남을 수 있음)
rm -rf "$APP_DIR/akey.app" "$APP_DIR/Akey.app"
mv "$TMP/Akey.app" "$APP_DIR/Akey.app"
# Apple Developer ID 서명 전까지는 격리 표시를 지워야 열린다
xattr -dr com.apple.quarantine "$APP_DIR/Akey.app" 2>/dev/null || true

mkdir -p "$BIN_DIR"
ln -sf "$APP_DIR/Akey.app/Contents/MacOS/akey" "$BIN_DIR/akey"

say "설치됨: $APP_DIR/Akey.app"
say "CLI:    $BIN_DIR/akey"
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) say ""; say "$BIN_DIR 이(가) PATH 에 없습니다. 셸 설정에 추가하세요:"; say "  echo 'export PATH=\"$BIN_DIR:\$PATH\"' >> ~/.zshrc" ;;
esac

[ -n "${AKEY_NO_LAUNCH:-}" ] || open "$APP_DIR/Akey.app" || true
say ""
say "메뉴바의 Akey 아이콘을 눌러 시작하세요. 처음 열 때 키체인 비밀번호를 물으면 '항상 허용'을 누르세요."
