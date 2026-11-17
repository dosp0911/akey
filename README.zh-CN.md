# Akey

[English](README.md) | [한국어](README.ko.md) | **简体中文**

一个小巧的菜单栏（托盘）应用和命令行工具。它把 AI 编程助手（Claude Code、Codex、Cursor 等）使用的 API 密钥和令牌**从文件中移出**，保存在本机的安全存储里。

- **macOS**：保存在登录钥匙串中，用 Touch ID（或登录密码）确认。
- **Windows**：用用户账户加密（DPAPI）和 Windows Hello（PIN、指纹、面部）保护。
- 助手通过 `akey exec NAME -- 命令` **只以环境变量的形式**拿到值。值不会留在对话记录或 `.env` 文件中；即使命令输出了值，也会被替换为 `***`。

<p align="center">
  <img src="docs/images/list-en.png" width="400" alt="菜单栏中的密钥列表">
  <img src="docs/images/select-en.png" width="400" alt="选择多个密钥以 .env 格式复制">
</p>
<p align="center">
  <img src="docs/images/settings-en.png" width="370" alt="设置窗口">
</p>
<p align="center"><sub>截图使用的是示例数据，并非真实密钥。</sub></p>

## 安装

### macOS

```bash
brew tap dosp0911/akey
brew trust dosp0911/akey
brew install dosp0911/akey/akey
```

Homebrew 只会加载你信任过的第三方 tap 中的 cask，因此需要 `brew trust`。

或者

```bash
curl -fsSL https://github.com/dosp0911/akey/releases/latest/download/install.sh | sh
```

### Windows

```powershell
winget install akey
```

或者使用 Scoop

```powershell
scoop bucket add akey https://github.com/dosp0911/scoop-akey
scoop install akey
```

或者使用 PowerShell

```powershell
irm https://github.com/dosp0911/akey/releases/latest/download/install.ps1 | iex
```

在 macOS 上会安装 `/Applications/Akey.app` 和 `akey` 命令；在 Windows 上会把 `akey.exe`（命令行）和 `akey-app.exe`（托盘应用）安装到 `%LOCALAPPDATA%\Programs\akey`。

> **关于签名**：目前的版本还没有 Apple Developer ID 签名。Homebrew 和安装脚本会替你移除 Gatekeeper 的隔离标记。更新后第一次打开 Akey 时，如果 macOS 询问钥匙串密码，请点击 **“始终允许”**。

> **校验下载文件**：安装脚本会用同一版本中的 `checksums.txt` 校验文件，可以发现下载损坏，但无法防止发布文件和校验和被同时替换。每个发布文件都附带 GitHub 构建来源证明（attestation），可以确认它确实由本仓库的发布工作流构建：
>
> ```bash
> gh attestation verify akey-macos-universal.tar.gz -R dosp0911/akey
> ```

## 使用

菜单栏（托盘）图标：
- **左键点击**：打开密钥列表。值默认隐藏，点击值或 👁 并通过验证后显示。
- **右键点击**：关于 / 设置 / 退出。
- **分组**：顶部的 7 个圆点把密钥分成不同的组（个人、工作、项目等）。点击圆点只显示该组，点击组名可以重命名。新密钥会放进当前查看的组，可通过 ✏️ → 分组移动。搜索会查找所有组。
- **锁定徽章**：点击 **已锁定** 并通过验证后一次显示所有值；点击 **已解锁** 会隐藏所有值并重新锁定。
- **助手访问**（右上角）：按密钥 / 8 小时全部 / 始终全部。开启时需要验证。
- 验证一次后，在设定的时间内（默认 30 分钟）查看和复制都不会再询问。
- 删除、编辑（名称或值）和更改访问权限每次都会询问。

### 选择多个密钥分享

需要把一组密钥交给同事或配置另一台电脑时：

1. 点击列表底部的清单图标按钮（**Select several to copy**）。
2. 勾选要分享的密钥，或点击 **Select all**。点击行内任意位置也可以勾选。
3. 点击 **Copy**。即使已解锁，akey 也**每次都会要求验证**，然后以 `.env` 格式复制到剪贴板：

```dotenv
ANTHROPIC_API_KEY=sk-ant-...
DATABASE_URL=postgres://dev:dev@localhost:5432/app
GREETING="hello world"
```

含有空格、引号、`$`、反引号或换行的值会被加上引号并转义，因此可以直接粘贴到 `.env` 文件中，或用 `source` 安全加载。请通过安全渠道（密码管理器的共享功能、端到端加密的聊天工具等）发送，不要使用公开渠道。

命令行：

```text
akey list                         名称、访问状态和备注（从不显示值）
akey exec NAME... -- CMD...       以环境变量注入后运行 CMD（推荐助手使用，输出中的值显示为 ***）
akey get NAME                     输出值
akey set NAME [--note 备注]       保存或更新（隐藏输入或 stdin）
akey note NAME [备注|--clear]     查看 / 写入 / 清除备注
akey rm NAME                      删除
akey mv OLD NEW                   重命名（保留值、备注和访问权限）
akey allow NAME|--all [--hours N] 允许助手无需验证即可读取（省略 N 表示不过期）
akey deny NAME|--all              撤销访问
```

何时需要验证：
- **从不**：用新名称 `set`、`list`、`deny`
- **仅针对未被允许的密钥**：`get`、`exec`
- **每次都要**：`allow`、`rm`、`mv`、覆盖已有的值、修改已有备注
- 如果验证被拒绝，退出码为 `77`。

### 告诉你的助手

把下面的内容放进助手的全局指令中，助手就只会通过 akey 读取、写入和管理本地密钥。

```markdown
## Secrets
Local secrets (API keys, tokens, passwords) live in `akey`. Never put them in files, `.env`, commands, or output.
- List: `akey list`
- Use: `akey exec NAME... -- <cmd>` (not `akey get`)
- Add: `printf %s "$VALUE" | akey set NAME --note "<purpose>"`
- Found a plaintext secret? Move it into akey. Missing one? Ask me.
- Exit 77 = I declined; don't retry.
```

可以一次性添加到 Claude Code（`~/.claude/CLAUDE.md`）以及读取 `AGENTS.md` 的工具（如 Codex，`~/.codex/AGENTS.md`）。如果已经存在，则不会重复添加。

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

Windows（PowerShell）

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

- 助手使用 `exec` 而不是 `get`：`get` 输出的值会留在助手的对话记录中。
- 故意没有写入访问和删除（`allow`、`deny`、`rm`），这样助手无法自行扩大权限，只有你自己能操作。

## 安全模型

| | macOS | Windows |
|---|---|---|
| 存储位置 | 一个登录钥匙串项目 `akey` / `vault` | `%LOCALAPPDATA%\akey\vault.json` |
| “每次询问”的值 | 钥匙串，只允许 akey 应用读取 | 用只有 Windows Hello 才能解开的 X25519 密钥封装 |
| “已允许”的值 | 钥匙串 | DPAPI（同一用户账户下的程序可以读取） |
| 确认方式 | Touch ID / 登录密码 | Windows Hello（PIN / 指纹 / 面部） |

- `akey exec` 总会把命令输出（标准输出和标准错误）中的密钥值替换为 `***`，即使值被分段输出也一样。命令本身仍使用真实的值。少于 4 个字符的值或经过变换（如 base64）的值无法遮盖。需要亲自查看值时，请使用菜单栏或 `akey get`。
- 值永远不会离开这台电脑（没有 iCloud 或云同步）。如需备份，请把原始值另外保存在密码管理器中。
- “始终允许”和“助手访问全部密钥”很方便，但会让以你身份运行的任何程序都能无需验证读取值。日常使用更推荐限时允许（8 小时）。
- **Windows**：如果重置了 Windows Hello PIN 或清除了 TPM，“每次询问”的值将无法恢复。

## 从源码构建

```bash
cd rust
npm ci                                   # Tauri CLI
cargo test -p akey-core -p akey-windows -p akey-cli
cd app && npx tauri build                # macOS: target/release/bundle/macos/Akey.app
```

- `rust/crates/akey-core`：保险库模型、访问规则、设置、韩语/英语文本
- `rust/crates/akey-macos`：钥匙串 + Touch ID
- `rust/crates/akey-windows`：vault.json + DPAPI + Windows Hello（`engine.rs` 可在任何系统上测试）
- `rust/crates/akey-cli`：命令行
- `rust/app`、`rust/ui`：Tauri 托盘应用和界面

本地 macOS 构建使用 `tauri.conf.json` 中的 `signingIdentity`（“akey Local Signing”）签名。没有该证书时，可用 `--config '{"bundle":{"macOS":{"signingIdentity":"-"}}}'` 进行临时签名。

## 发布（维护者）

1. 更新 `rust/app/tauri.conf.json` 中的版本号并推送标签：`git tag v0.2.0 && git push origin v0.2.0`
2. `.github/workflows/release.yml` 会构建 macOS 通用应用和 Windows 可执行文件，连同安装脚本和 `checksums.txt` 一起上传到 GitHub Release。
3. 每个包管理器都需要一次性设置。所有令牌都放在本仓库的 Settings → Secrets 中。

| 渠道 | 一次性设置 | Secret | 之后 |
|---|---|---|---|
| Homebrew | 创建 `dosp0911/homebrew-akey` 仓库 | `HOMEBREW_TAP_TOKEN`（对 tap 仓库有写权限） | 每次发布自动更新 `Casks/akey.rb` |
| Scoop | 创建 `dosp0911/scoop-akey` 仓库 | `SCOOP_BUCKET_TOKEN`（对 bucket 仓库有写权限） | 每次发布自动更新 `bucket/akey.json` |
| winget | 把首个版本的 `winget-manifests.zip` 解压到 [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) 的 `manifests/a/akey/akey/<版本>/` 并提交 PR（包 ID 为 `akey.akey`；也可以用 `wingetcreate submit <文件夹>`） | `WINGET_TOKEN`（具有 `public_repo` 权限的 PAT） | 收录后，每次发布会自动提交更新 PR |

缺少对应 Secret 的步骤会被跳过。winget 由 Microsoft 审核，通常需要几天。未签名的可执行文件可能会触发 SmartScreen 警告并接受额外审核。

## 许可证

[MIT](LICENSE)
