# akey 설치 (Windows)
#   irm https://github.com/dosp0911/akey/releases/latest/download/install.ps1 | iex
#
# 하는 일: 최신 릴리스의 akey.exe(CLI)와 akey-app.exe(트레이 앱)를 받아 체크섬을 확인하고
#          %LOCALAPPDATA%\Programs\akey 에 설치, 사용자 PATH 에 추가, 시작 메뉴 바로가기를 만든다.
# 환경변수: AKEY_VERSION=v0.2.0 (특정 버전), AKEY_INSTALL_DIR=경로
$ErrorActionPreference = "Stop"

$Repo = if ($env:AKEY_REPO) { $env:AKEY_REPO } else { "dosp0911/akey" }
$Asset = "akey-windows-x64.zip"
$Base = if ($env:AKEY_VERSION) { "https://github.com/$Repo/releases/download/$($env:AKEY_VERSION)" } else { "https://github.com/$Repo/releases/latest/download" }
$InstallDir = if ($env:AKEY_INSTALL_DIR) { $env:AKEY_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA "Programs\akey" }

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("akey-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $Tmp | Out-Null
try {
    Write-Host "akey 를 받는 중: $Base/$Asset"
    Invoke-WebRequest -UseBasicParsing "$Base/$Asset" -OutFile (Join-Path $Tmp $Asset)
    Invoke-WebRequest -UseBasicParsing "$Base/checksums.txt" -OutFile (Join-Path $Tmp "checksums.txt")

    $line = Get-Content (Join-Path $Tmp "checksums.txt") | Where-Object { $_ -match " $([regex]::Escape($Asset))$" }
    if (-not $line) { throw "체크섬 파일에 $Asset 이 없습니다" }
    $expected = ($line -split "\s+")[0].ToLower()
    $actual = (Get-FileHash -Algorithm SHA256 (Join-Path $Tmp $Asset)).Hash.ToLower()
    if ($expected -ne $actual) { throw "체크섬이 맞지 않습니다 (받은 파일이 손상되었거나 바뀌었습니다)" }
    Write-Host "체크섬 확인됨"

    # 실행 중이면 종료 (금고 파일은 %LOCALAPPDATA%\akey 에 있어 지워지지 않음)
    Get-Process akey-app -ErrorAction SilentlyContinue | Stop-Process -Force
    New-Item -ItemType Directory -Force $InstallDir | Out-Null
    Expand-Archive -Force (Join-Path $Tmp $Asset) $InstallDir
    Get-ChildItem $InstallDir -Filter *.exe | Unblock-File

    # 사용자 PATH 에 추가
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not (($userPath -split ";") -contains $InstallDir)) {
        [Environment]::SetEnvironmentVariable("Path", (($userPath.TrimEnd(";") + ";" + $InstallDir).TrimStart(";")), "User")
        Write-Host "PATH 에 추가됨: $InstallDir (새 터미널부터 적용)"
    }

    # 시작 메뉴 바로가기
    $startMenu = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut((Join-Path $startMenu "Akey.lnk"))
    $shortcut.TargetPath = Join-Path $InstallDir "akey-app.exe"
    $shortcut.Save()

    Start-Process (Join-Path $InstallDir "akey-app.exe")
    Write-Host ""
    Write-Host "설치됨: $InstallDir"
    Write-Host "작업 표시줄 오른쪽 아래(트레이)의 akey 아이콘을 눌러 시작하세요."
    Write-Host "처음 시크릿을 저장할 때 Windows Hello(PIN/지문) 확인을 묻습니다."
}
finally {
    Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
}
