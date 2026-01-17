# 2026-01-17_05_PostToolUseHook修正

## 概要

Claude Code の PostToolUse hook が動作していなかった問題を修正。

## 背景と目的

PR #46 の CI で Rust ファイルのフォーマットエラーが発生。
調査の結果、`.rs` ファイル作成後の自動フォーマット hook が動作していなかった。

## 実施内容

### 1. 問題の特定

PostToolUse hook のコマンドを調査:

```bash
# 修正前（動作しない）
file_path=$(jq -r '.tool_input.file_path // empty' 2>/dev/null)
```

**問題点:**
1. `jq` に入力（`$TOOL_INPUT`）を渡していない
2. JSON のキーが `.tool_input.file_path` ではなく `.file_path`

### 2. 追加の問題

`cargo fmt` を使用していたが、`Cargo.toml` がカレントディレクトリにない場合に失敗する。

```bash
# エラー
error: could not find `Cargo.toml` in `/path/to/project` or any parent directory
```

### 3. 修正

```bash
# 修正後
file_path=$(echo "$TOOL_INPUT" | jq -r '.file_path // empty' 2>/dev/null)
if [[ "$file_path" == *.rs ]]; then
    just fmt-rust "$file_path" 2>/dev/null || true
elif [[ "$file_path" == *.elm ]]; then
    just fmt-elm "$file_path" 2>/dev/null || true
fi
```

**修正点:**
1. `echo "$TOOL_INPUT" |` で jq に入力を渡す
2. `.file_path` で直接キーを参照
3. `just fmt-rust` / `just fmt-elm` を使用（設定を justfile に集約）

### 4. justfile の改善

`fmt-rust` をオプショナル引数対応にして、設定を一箇所に集約:

```just
# Rust フォーマット（引数なし=全ファイル、引数あり=指定ファイル）
fmt-rust *files:
    #!/usr/bin/env bash
    if [ -z "{{files}}" ]; then
        cd backend && cargo +nightly fmt --all
    else
        rustfmt +nightly --edition 2024 --quiet {{files}}
    fi
```

## 成果物

### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.claude/settings.json` | PostToolUse hook を修正 |
| `justfile` | `fmt-rust` をオプショナル引数対応に改善 |
| `docs/06_技術ノート/ClaudeCode_Hooks.md` | 環境変数とプロジェクト設定例を追加 |

## 学んだこと

### Claude Code hook のデバッグ

1. hook のコマンドは手動で実行してテストできる:
   ```bash
   TOOL_INPUT='{"file_path":"test.rs"}' bash -c 'hook_command_here'
   ```

2. `$TOOL_INPUT` は JSON 形式で、ツールの引数がそのまま入る

3. `cargo fmt` は `Cargo.toml` が必要なので、`rustfmt` を直接使う方が汎用的

### hook が動作しない場合のチェックポイント

1. `$TOOL_INPUT` を正しくパイプしているか
2. JSON のキー名が正しいか
3. コマンドがカレントディレクトリに依存していないか

詳細: [ClaudeCode_Hooks 技術ノート](../../docs/06_技術ノート/ClaudeCode_Hooks.md)

## ユーザープロンプト（抜粋）

> commit 時に lint してフォーマットも含めチェックしているはずだが hook が動いていないのか間違っているのか

> hook のコマンドが複雑なので技術ノートかコメントで補足してください。あとセッションログもお願いします。
