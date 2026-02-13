# #494 cargo-outdated 導入と /assess スキル修正

## 概要

Issue #494 の精査により、`cargo-audit` は ADR-033 で選択した `cargo-deny` で完全にカバー済みと判明。スコープを調整し、`cargo-outdated` の導入と `/assess` スキルのセキュリティ監査コマンド修正を実施した。

## 実施内容

### 1. Issue #494 の精査とスコープ調整

元の Issue は「cargo-audit / cargo-outdated を導入する」だったが、調査の結果:

- `cargo-audit` の機能は `cargo-deny check advisories` で完全にカバー済み（同じ RustSec Advisory DB を使用）
- `/assess` が `cargo-audit` の有無をチェックしていたため、2 回連続で誤検出していた

スコープを以下に調整:
- `cargo-outdated` の導入（ローカルのみ）
- `/assess` スキルの修正（`cargo audit` → `cargo deny check advisories`）

### 2. justfile の更新

- `check-tools` に `cargo-outdated` の存在確認を追加
- `outdated` タスクを新設（`cargo outdated --root-deps-only` + `pnpm outdated`）
- `pnpm outdated` は outdated パッケージがあると exit code 1 を返すため、just の `-` プレフィックスでエラーを吸収

### 3. 開発環境構築手順の更新

- 概要テーブルに cargo-outdated 行を追加
- セクション 21（cargo-outdated インストール）を新設
- 旧 21（全ツール確認）→ 22、旧 22（IDE 設定）→ 23 にリナンバリング
- 全ツール確認セクションに `cargo outdated --version` を追加

### 4. /assess スキルの修正

- セキュリティ監査コマンドを `cargo audit` → `cargo deny check advisories` に変更
- ツール可用性の注記を修正（cargo-outdated はオプショナル、cargo-deny は必須）

### 5. Issue #494 にスコープ変更コメントを追加

## 判断ログ

- `cargo-outdated` を CI に追加しない判断: outdated な依存は直ちにセキュリティリスクにならない。Dependabot（週次）+ `/assess`（月次）で十分
- `just outdated` を `check-all` に含めない判断: `check-all` はブロッキングチェックのみ。outdated は情報提供
- `pnpm outdated` の exit code 1 への対処: just の `-` プレフィックスでエラー吸収

## 成果物

### コミット

- `ccf16fc` #494 Add cargo-outdated and fix /assess security audit command

### 変更ファイル

- `justfile` — check-tools に cargo-outdated 追加、outdated タスク新設
- `docs/04_手順書/01_開発参画/01_開発環境構築.md` — セクション 21 新設、リナンバリング
- `.claude/skills/assess/SKILL.md` — セキュリティ監査コマンド修正
