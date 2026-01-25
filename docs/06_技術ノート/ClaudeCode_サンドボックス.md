# Claude Code サンドボックス

Claude Code のサンドボックス機能に関する技術ノート。

## 概要

Claude Code はセキュリティのためにサンドボックス機能を提供している。OS によって実装が異なる。

| OS | 実装 |
|----|------|
| macOS | Seatbelt（組み込みフレームワーク） |
| Linux | bubblewrap |
| WSL2 | bubblewrap |

## サンドボックスの動作

### ファイルシステム隔離

デフォルト動作:
- 書き込み: 作業ディレクトリとそのサブディレクトリのみ
- 読み取り: コンピュータ全体（拒否リストを除く）

### 拒否リスト（デフォルト）

以下のファイルは読み取りが制限される:
- `.env`, `.env.local`, `.env.production`
- `~/.aws/credentials`
- `~/.ssh`
- `~/.gnupg`

## Linux（bubblewrap）での問題

### 根本原因

**Claude Code サンドボックス実装のバグ**（[Issue #17258](https://github.com/anthropics/claude-code/issues/17258)）

bubblewrap の呼び出し時に、パスの指定が誤っている:

```bash
# 実際の動作（バグ）
--ro-bind /home/user/project/.bashrc /home/user/project/.bashrc

# 期待される動作
--ro-bind $HOME/.bashrc $HOME/.bashrc
```

ホームディレクトリのドットファイルを保護する意図で、誤ってプロジェクトディレクトリに空のバインドマウントターゲットを作成してしまっている。

### 発生環境

| 環境 | bubblewrap | 発生 |
|------|------------|------|
| Fedora 43 | 0.11.0 | ✅ 発生 |
| Ubuntu 24.04 | 0.8.0（推定） | ❌ 発生しない |

環境による差異の推測:
1. Claude Code のバージョン差（バグ導入前/後）
2. bubblewrap のバージョン差（bind mount の挙動差異）
3. サンドボックス設定の差（`excludedCommands` の内容）

### 症状

プロジェクトディレクトリに以下の空ファイル（0 バイト、読み取り専用）が作成される:

```
.bashrc
.bash_profile
.zshrc
.zprofile
.profile
.gitconfig
.gitmodules
.ripgreprc
.mcp.json
.vscode/
.claude/agents/
.claude/commands/
.claude/settings.local.json
```

### 原因

bubblewrap がサンドボックス初期化時に、シェル環境ファイルをマウントポイントとして作成する。これはシェル起動時の設定ファイル読み込みを隔離するための機構。

macOS（Seatbelt）では異なる初期化機構を使用するため、この問題は発生しない。

### 確認方法

```bash
# ファイルがサンドボックスによるものか確認
ls -la .bashrc  # 0 バイト、読み取り専用なら該当

# サンドボックス内から削除を試みる
rm .bashrc  # 「デバイスもしくはリソースがビジー状態です」→ マウントポイント
```

## 解決策

### 設定変更

`.claude/settings.json` の `excludedCommands` に `bash` と `sh` を追加する。

```json
{
  "sandbox": {
    "enabled": true,
    "excludedCommands": [
      "bash",
      "sh",
      "git",
      "gh",
      // ... 他のコマンド
    ]
  }
}
```

これにより bash コマンドがサンドボックス外で実行され、マウントポイントが作成されなくなる。

### 既存ファイルの削除

設定変更後、Claude Code セッションを終了してからファイルを削除する。

```bash
# セッション終了後に実行
rm -f .bashrc .bash_profile .zshrc .zprofile .profile .gitconfig .gitmodules .ripgreprc .mcp.json
rm -rf .vscode .claude/agents .claude/commands
rm -f .claude/settings.local.json
```

サンドボックス内からは削除できない（マウントポイントがビジー状態）。

## トレードオフ

`bash` を `excludedCommands` に追加することで:
- ✅ スタブファイルが作成されなくなる
- ⚠️ bash コマンドはサンドボックス外で実行される

セキュリティは `permissions` 設定の `deny` ルールで担保する:

```json
{
  "permissions": {
    "deny": [
      "Bash(rm -rf:*)",
      "Bash(sudo:*)",
      "Bash(chmod 777:*)",
      // ...
    ]
  }
}
```

## /sandbox コマンド

Claude Code セッション中に `/sandbox` を実行すると、サンドボックスモードを切り替えられる。

| モード | 挙動 |
|--------|------|
| Auto-allow mode | サンドボックス内のコマンドは自動承認 |
| Regular permissions mode | すべてのコマンドが権限フローを通す |

## 参考

- [Claude Code Sandboxing](https://docs.anthropic.com/en/docs/claude-code/security#sandboxing)
- [Claude Code Settings](https://docs.anthropic.com/en/docs/claude-code/settings)
- [Issue #17258: Sandbox creates phantom dotfiles](https://github.com/anthropics/claude-code/issues/17258)
- [bubblewrap - GitHub](https://github.com/containers/bubblewrap)
- [Bubblewrap - ArchWiki](https://wiki.archlinux.org/title/Bubblewrap)
