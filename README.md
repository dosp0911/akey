# Akey

**Akey** stands for **Agent key**: the keys your AI agents use, kept in one safe place.

**English** | [한국어](README.ko.md) | [简体中文](README.zh-CN.md)

A small menu bar (tray) app and CLI that keeps the API keys and tokens used by AI coding agents (Claude Code, Codex, Cursor and others) **out of files**, in your computer's secure storage.

- **macOS**: stored in the login keychain, confirmed with Touch ID (or your login password).
- **Windows**: protected with user-account encryption (DPAPI) and Windows Hello (PIN, fingerprint, face).
- Agents receive values **only as environment variables** through `akey exec NAME -- command`. Values never end up in chat logs or `.env` files, and if the command prints a value, akey replaces it with `***`.

<p align="center">
  <img src="docs/images/list-en.png" width="400" alt="Secret list in the menu bar">
  <img src="docs/images/select-en.png" width="400" alt="Selecting several secrets to copy as .env">
</p>
<p align="center">
  <img src="docs/images/settings-en.png" width="370" alt="Settings window">
</p>
<p align="center"><sub>Screenshots use sample data, not real keys.</sub></p>

## Install

### macOS

```bash
brew tap dosp0911/akey
brew trust dosp0911/akey
brew install dosp0911/akey/akey
```

Homebrew only loads casks from third-party taps you have trusted, hence `brew trust`.

or

```bash
curl -fsSL https://github.com/dosp0911/akey/releases/latest/download/install.sh | sh
```

### Windows

```powershell
winget install Akey
```

or Scoop

```powershell
scoop bucket add akey https://github.com/dosp0911/scoop-akey
scoop install akey
```

or PowerShell

```powershell
irm https://github.com/dosp0911/akey/releases/latest/download/install.ps1 | iex
```

On macOS this installs `/Applications/Akey.app` and the `akey` command. On Windows it installs `akey.exe` (CLI) and `akey-app.exe` (tray app) into `%LOCALAPPDATA%\Programs\akey`.

> **About signing**: builds are not yet signed with an Apple Developer ID. Homebrew and the install script remove the Gatekeeper quarantine flag for you. After an update, if macOS asks for your keychain password the first time Akey opens, click **"Always Allow"**.

> **Verifying downloads**: the install scripts check each file against `checksums.txt` from the same release. That catches corrupted downloads, but not a release whose files *and* checksums were both replaced. Every release file also carries a GitHub build-provenance attestation, so you can confirm it was built by this repository's release workflow:
>
> ```bash
> gh attestation verify akey-macos-universal.tar.gz -R dosp0911/akey
> ```

## Usage

Menu bar (tray) icon:
- **Left click**: opens the list of secrets. Values are masked; click a value or 👁 to reveal it after authenticating.
- **Right click**: About / Settings / Quit. The app and CLI speak English, Korean and Simplified Chinese (Settings → Language).
- **Groups**: the seven circles at the top split your secrets into groups (personal, work, a project…). Click a circle to see that group; click the group name to rename it. New secrets go into the group you're looking at; move one with ✏️ → Group. Search looks through every group.
- **Lock badge**: click **Locked** to authenticate and show every value at once; click **Unlocked** to hide them all and lock again.
- **Agent access** (top right): per secret / all secrets for 8 hours / all secrets always. Turning it on asks for authentication.
- After one authentication, viewing and copying won't ask again for the time you set (30 minutes by default).
- Deleting, editing (name or value) and changing access always ask.

### Sharing several secrets

To hand a set of keys to a teammate or set up another machine:

1. Click the **select** button (checklist icon) at the bottom of the list.
2. Tick the secrets you want, or use **Select all**. Clicking anywhere on a row toggles it.
3. Click **Copy**. akey always asks for authentication here, even while unlocked, then copies them in `.env` format:

```dotenv
ANTHROPIC_API_KEY=sk-ant-...
DATABASE_URL=postgres://dev:dev@localhost:5432/app
GREETING="hello world"
```

Values with spaces, quotes, `$`, backticks or line breaks are quoted and escaped, so the text can be pasted into a `.env` file or loaded with `source` safely. Send it over a private channel (a password manager's sharing, an end-to-end encrypted chat), not a public one.

CLI:

```text
akey list                         names, access and notes (values are never shown)
akey exec NAME... -- CMD...       run CMD with secrets as env vars (for agents; values in output show as ***)
akey get NAME                     print a value
akey set NAME [--note NOTE]       save or update (hidden prompt or stdin)
akey note NAME [NOTE|--clear]     show / write / clear a note
akey rm NAME                      delete
akey mv OLD NEW                   rename (keeps value, note and access)
akey allow NAME|--all [--hours N] let agents read without asking (no N = no expiry)
akey deny NAME|--all              revoke access
```

When akey asks:
- **Never**: `set` with a new name, `list`, `deny`
- **Only for secrets that aren't allowed**: `get`, `exec`
- **Always**: `allow`, `rm`, `mv`, overwriting, changing an existing note
- If authentication is declined, the exit code is `77`.

### Telling your agents

Put the following in your agents' global instructions, and they will read, write and manage local secrets only through akey.

```markdown
## Secrets
Local secrets (API keys, tokens, passwords) live in `akey`. Never put them in files, `.env`, commands, or output.
- List: `akey list`
- Use: `akey exec NAME... -- <cmd>` (not `akey get`)
- Add: `printf %s "$VALUE" | akey set NAME --note "<purpose>"`
- Found a plaintext secret? Move it into akey. Missing one? Ask me.
- Exit 77 = I declined; don't retry.
```

You can add it in one go to Claude Code (`~/.claude/CLAUDE.md`) and to tools that read `AGENTS.md`, such as Codex (`~/.codex/AGENTS.md`). If it is already there, it isn't added again.

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

- Agents use `exec`, not `get`: a value printed by `get` stays in the agent's chat log.
- Access and deletion (`allow`, `deny`, `rm`) are left out on purpose, so agents can't widen their own access. Only you do that.

## Security model

| | macOS | Windows |
|---|---|---|
| Where secrets live | One login keychain item, `akey` / `vault` | `%LOCALAPPDATA%\akey\vault.json` |
| "Ask every time" values | Keychain; only the akey app is allowed to read it | Sealed with an X25519 key that only Windows Hello can unlock |
| "Allowed" values | Keychain | DPAPI (readable by programs running as the same user) |
| How you confirm | Touch ID / login password | Windows Hello (PIN / fingerprint / face) |

- `akey exec` always masks secret values in the command's output (stdout and stderr) with `***`, even when a value is printed in pieces. The command itself still uses the real value. Values shorter than 4 characters, or printed in a transformed form (e.g. base64), aren't masked. To see a value yourself, use the menu bar or `akey get`.
- Values never leave this computer (no iCloud or cloud sync). If you need a backup, keep the originals in a password manager.
- "Always allow" and "agent access to all secrets" are convenient, but they let any program running as you read values without asking. Time-limited access (8 hours) is the better everyday choice.
- **Windows**: if you reset your Windows Hello PIN or clear the TPM, "ask every time" values can't be recovered.

## Build from source

```bash
cd rust
npm ci                                   # Tauri CLI
cargo test -p akey-core -p akey-windows -p akey-cli
cd app && npx tauri build                # macOS: target/release/bundle/macos/Akey.app
```

- `rust/crates/akey-core`: vault model, access rules, settings, Korean/English/Chinese text
- `rust/crates/akey-macos`: keychain + Touch ID
- `rust/crates/akey-windows`: vault.json + DPAPI + Windows Hello (`engine.rs` is tested on any OS)
- `rust/crates/akey-cli`: CLI
- `rust/app`, `rust/ui`: Tauri tray app and screens

Local macOS builds are signed with the `signingIdentity` in `tauri.conf.json` ("akey Local Signing"). Without that certificate, sign ad hoc with `--config '{"bundle":{"macOS":{"signingIdentity":"-"}}}'`.

## Releasing (maintainers)

1. Bump the version in `rust/app/tauri.conf.json` and push a tag: `git tag v0.2.0 && git push origin v0.2.0`
2. `.github/workflows/release.yml` builds the macOS universal app and the Windows executables and uploads them to a GitHub Release, along with the install scripts and `checksums.txt`.
3. Each package manager needs a one-time setup. Put every token in this repository's Settings → Secrets.

| Channel | One-time setup | Secret | After that |
|---|---|---|---|
| Homebrew | Create the `dosp0911/homebrew-akey` repository | `HOMEBREW_TAP_TOKEN` (write access to the tap repository) | `Casks/akey.rb` is updated on every release |
| Scoop | Create the `dosp0911/scoop-akey` repository | `SCOOP_BUCKET_TOKEN` (write access to the bucket repository) | `bucket/akey.json` is updated on every release |
| winget | Unzip the first release's `winget-manifests.zip` into `manifests/a/Akey/Akey/<version>/` of [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) and open a PR (package ID `Akey.Akey`; `wingetcreate submit <folder>` also works) | `WINGET_TOKEN` (PAT with `public_repo`) | Once accepted, an update PR is sent on every release |

Steps whose secret is missing are skipped. Microsoft reviews winget submissions, which usually takes a few days. Unsigned executables may get extra review along with a SmartScreen warning.

## License

[MIT](LICENSE)
