# #494 cargo-outdated 導入と /assess スキル修正

## Context

Issue #494 は「cargo-audit / cargo-outdated を導入しセキュリティ・依存関係チェックを有効化する」という内容だが、Issue の前提が不正確だった。

**調査結果:**
- `cargo-audit` の機能は **ADR-033 で選択した `cargo-deny` で完全にカバー済み**（同じ RustSec Advisory DB を使用）
- CI の `security` ジョブとローカルの `just audit` で既に実行可能
- `/assess` スキルが `cargo-audit` の有無だけをチェックし、`cargo-deny` でカバー済みであることを考慮していなかったため、2 回連続で誤検出していた

**調整後のスコープ:**
1. `cargo-outdated` の導入（ローカルのみ。依存関係の鮮度可視化は新しい機能）
2. `/assess` スキルの修正（`cargo audit` → `cargo deny check advisories`）
3. Issue #494 のコメントでスコープ変更を記録

**対象外:**
- `cargo-audit` の導入（ADR-033 で `cargo-deny` を選択済み。機能を完全に包含）
- `cargo-outdated` の CI 統合（outdated は直ちに問題にならない。Dependabot 週次 + `/assess` 月次で十分）

## 設計判断

### 1. cargo-outdated を CI に追加しない理由

outdated な依存は直ちにセキュリティリスクや品質問題にならない。CI でブロッキングチェックにすると、Dependabot PR がマージされるまで他の PR がブロックされる。Dependabot（週次）+ `/assess`（月次）で十分。

### 2. `just outdated` タスクの設計

Rust と Frontend を 1 タスクで実行。分割（`outdated-rust` / `outdated-frontend`）は現時点でオーバーエンジニアリング。

`pnpm outdated` は outdated パッケージがあると exit code 1 を返すため、just の `-` プレフィックスでエラーを吸収する。

### 3. `just check-all` に含めない理由

`check-all` はブロッキングチェック（`check audit test-api test-e2e`）のみを含む。outdated は品質ゲートではなく情報提供。

### 4. セクション番号

開発環境構築手順で、新ツールをセクション 21 に追加。現在の 21（全ツール確認）を 22 に、22（IDE 設定）を 23 にリナンバリング。

---

## Phase 1: justfile の更新

### 確認事項
- [ ] パターン: check-tools の行追加パターン → `justfile` L48
- [ ] パターン: セキュリティチェックセクション → `justfile` L342-348
- [ ] ライブラリ: `cargo outdated --root-deps-only` フラグ → 既存使用なし、公式 README で確認

### 変更内容

**1. `check-tools` に `cargo-outdated` を追加（L48 の `cargo-deny` の後）:**
```just
@which cargo-outdated > /dev/null || (echo "ERROR: cargo-outdated がインストールされていません" && exit 1)
```

**2. `outdated` タスクを追加（L348 `audit` タスクの後、L350 構造品質チェックの前）:**
```just
# =============================================================================
# 依存関係鮮度チェック
# =============================================================================

# 依存関係の更新状況を確認（cargo-outdated + pnpm outdated）
outdated:
    cd backend && cargo outdated --root-deps-only
    -cd frontend && pnpm outdated
```

### 検証手順
- `just check-tools` で cargo-outdated の確認が含まれること
- `just outdated` で Rust/Frontend の outdated チェックが実行されること

---

## Phase 2: 開発環境構築手順の更新

### 確認事項
- [ ] パターン: 概要テーブル → `01_開発環境構築.md` L10-35
- [ ] パターン: cargo-deny セクション構造 → `01_開発環境構築.md` L552-571
- [ ] パターン: 全ツール確認 → `01_開発環境構築.md` L676-744

### 変更内容

**1. 概要テーブル（L35 の Playwright 行の後）に行を追加:**
```markdown
| 依存関係鮮度 | cargo-outdated | 最新 | Cargo 依存関係の更新状況確認 |
```

**2. セクション 21: cargo-outdated を新設（現 21 の前に挿入）:**
```markdown
## 21. cargo-outdated のインストール

Cargo 依存関係の更新状況を確認するツール。`just outdated` で使用する。

公式: https://github.com/kbknapp/cargo-outdated

### 21.1 Cargo でインストール

```bash
cargo install --locked cargo-outdated
```

### 21.2 バージョン確認

```bash
cargo outdated --version
# 出力例: cargo-outdated v0.18.0
```
```

**3. リナンバリング:** 旧 21 → 22（全ツール確認）、旧 22 → 23（IDE 設定）

**4. 全ツール確認セクション（旧 21）に確認コマンドを追加:**
```bash
# 依存関係鮮度
cargo outdated --version
```

### 検証手順
- セクション番号が連続していること
- 概要テーブルの行数がツール数と一致すること

---

## Phase 3: /assess スキルの修正

### 確認事項
- [ ] パターン: 3c テーブル → `SKILL.md` L115-123
- [ ] パターン: 補足セクション → `SKILL.md` L269-274

### 変更内容

**1. 3c. 依存関係の鮮度テーブル（L121）:**
- 変更前: `| セキュリティ監査 | cd backend && cargo audit 2>/dev/null | 脆弱性件数 |`
- 変更後: `| セキュリティ監査 | cd backend && cargo deny check advisories 2>/dev/null | 脆弱性件数 |`

**2. ツール未インストール時の注記（L123）:**
- 変更前: `ツールがインストールされていない場合（コマンドが見つからない場合）はスキップし、その旨を報告する。`
- 変更後: `cargo-outdated がインストールされていない場合はスキップし、その旨を報告する。cargo-deny は必須ツール（just check-tools で確認済み）のため常に利用可能。`

**3. 補足セクション（L274）:**
- 変更前: `依存関係チェック（cargo outdated, cargo audit）はツールのインストール状態に依存する。...`
- 変更後: `依存関係鮮度チェック（cargo outdated）はツールのインストール状態に依存する。未インストール時はスキップし「未インストールのためスキップ」と報告する。セキュリティ監査（cargo deny）は必須ツールのため常に利用可能`

### 検証手順
- `cargo deny check advisories` がローカルで正常に実行できること

---

## Phase 4: Issue #494 のスコープ変更コメント

### 確認事項
- 確認事項: なし（既知パターンのみ）

### 変更内容

Issue #494 にコメントを追加し、スコープ変更の理由と実施内容を記録する。

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `pnpm outdated` が exit code 1 を返す問題 | 不完全なパス | just の `-` プレフィックスでエラーを吸収 |
| 1回目 | `/assess` 補足セクション（L274）の `cargo audit` 言及 | 未定義 | Phase 3 に修正を追加 |
| 2回目 | `just outdated` を `check-all` に含めるべきか | 設計判断の完結性 | 含めない。品質ゲートではなく情報提供 |
| 2回目 | CLAUDE.md の開発ツール追加時必須対応に該当するか | 既存ドキュメント整合 | 該当する。check-tools（Phase 1）+ 開発環境構築手順（Phase 2）で対応済み |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | justfile（check-tools + outdated）、開発環境構築手順（セクション + 概要テーブル + 全ツール確認 + リナンバリング）、/assess（テーブル + 注記 + 補足）、Issue コメント |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更内容が具体的なコード/テキストで記述。「検討」「必要に応じて」等なし |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | CI 見送り、check-all 不含、セクション番号、pnpm exit code 対処を全て記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象 4 Phase、対象外 2 項目（cargo-audit 導入、CI 統合）が明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮 | OK | pnpm outdated の exit code、cargo outdated のデフォルト exit 0、just `-` プレフィックス |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾なし | OK | ADR-033 と整合。CLAUDE.md の必須対応を遵守 |
