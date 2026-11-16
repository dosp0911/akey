# Homebrew Cask 템플릿. 릴리스 workflow 가 __VERSION__, __SHA256__, 저장소 이름을 채워
# <owner>/homebrew-akey 저장소의 Casks/akey.rb 로 올린다.
#
#   brew install <owner>/akey/akey
cask "akey" do
  version "__VERSION__"
  sha256 "__SHA256__"

  url "https://github.com/__OWNER__/akey/releases/download/v#{version}/akey-macos-universal.tar.gz"
  name "akey"
  desc "Local secret store for AI agents (menu bar app + CLI, Touch ID)"
  homepage "https://github.com/__OWNER__/akey"

  depends_on macos: ">= :ventura"

  app "akey.app"
  # 앱 실행 파일이 CLI 를 겸한다 (같은 서명이라 키체인 권한이 같음)
  binary "#{appdir}/akey.app/Contents/MacOS/akey"

  # Apple Developer ID 서명 전까지는 Gatekeeper 격리 표시를 지워야 실행된다
  postflight do
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/akey.app"]
  end

  uninstall quit: "local.akey"

  zap trash: [
    "~/Library/Application Support/akey",
    "~/Library/Preferences/local.akey.plist",
  ]

  caveats <<~EOS
    시크릿은 로그인 키체인의 'akey' 항목에 저장됩니다. 앱을 지워도 남아 있습니다.
    업데이트 후 처음 열 때 키체인 비밀번호를 한 번 물으면 '항상 허용'을 누르세요.
  EOS
end
