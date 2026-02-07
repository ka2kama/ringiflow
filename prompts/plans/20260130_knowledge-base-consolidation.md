# 技術ノート・学習ノート統合計画

`docs/06_技術ノート/` と `docs/08_学習ノート/` を統合し、カテゴリ別サブディレクトリで管理する `docs/06_ナレッジベース/` を構築する。

## カテゴリ構成

```
docs/06_ナレッジベース/
├── README.md
├── rust/          (4)  Cargoワークスペース, Newtype, エラーハンドリング, sqlx-cli
├── elm/           (5)  アーキテクチャ, ポート, ルーティング, NestedTEA, TEAメインループ(←学習ノート)
├── frontend/      (4)  Vite, esbuild, pnpm_コマンド, pnpm_ビルドスクリプト制限
├── infra/         (4)  DockerCompose, PostgreSQL, Redis, UUID
├── architecture/  (8)  BFF, DDD×2, EventSourcing, REST_API, サーキットブレーカー, マイクロサービス, モノレポ
├── security/      (4)  CSRF, DevAuth, エンタープライズ認証, パスワードハッシュ
├── devtools/      (15) ClaudeCode×5, GitHub×4, Git×3, Make, Redocly, hurl
└── english/       (4)  エンジニア略語, コミット動詞, 程度の副詞, 同意・肯定(←学習ノート)
```

合計: 48 ファイル（README 除く）、8 カテゴリ

## 実装ステップ

### Step 1: git mv でファイル移動

1. `mkdir -p docs/06_ナレッジベース/{rust,elm,frontend,infra,architecture,security,devtools,english}`
2. 各カテゴリへ `git mv`（ファイル名は変更しない。唯一の例外: `01_Elm_TEAメインループ.md` → `Elm_TEAメインループ.md`（連番プレフィクス除去））
3. `docs/06_技術ノート/README.md` → `docs/06_ナレッジベース/README.md`
4. 空ディレクトリ削除: `docs/06_技術ノート/`, `docs/08_学習ノート/`

### Step 2: README.md を書き換え

`docs/06_ナレッジベース/README.md`:
- タイトル・概要を「ナレッジベース」に更新
- カテゴリ一覧セクションを新設
- ADR との違いテーブルは維持（名称のみ変更）
- ファイル命名規則にサブディレクトリ運用を明記
- 変更履歴に統合日を追記

### Step 3: コードベース全体の参照更新

#### ソースコード（6 ファイル）
- `backend/crates/infra/src/session.rs` — `06_技術ノート/Redis.md` → `06_ナレッジベース/infra/Redis.md`
- `backend/crates/infra/src/password.rs` — `06_技術ノート/パスワードハッシュ.md` → `06_ナレッジベース/security/パスワードハッシュ.md`
- `backend/apps/bff/src/dev_auth.rs` — `06_技術ノート/DevAuth.md` → `06_ナレッジベース/security/DevAuth.md`
- `frontend/src/main.js` — `06_技術ノート/DevAuth.md` → `06_ナレッジベース/security/DevAuth.md`
- `scripts/generate-env.sh` — `06_技術ノート/DevAuth.md` → `06_ナレッジベース/security/DevAuth.md`
- `.github/workflows/ci.yaml` — `06_技術ノート/GitHubActions.md` → `06_ナレッジベース/devtools/GitHubActions.md`

#### 古いパス修正（`05_技術ノート` → `06_ナレッジベース/<カテゴリ>`、4 ファイル）
- `frontend/vite.config.js` — `05_技術ノート/Vite.md` → `06_ナレッジベース/frontend/Vite.md`
- `frontend/src/Main.elm` (2箇所) — `05_技術ノート/Elmアーキテクチャ.md`, `05_技術ノート/Elmポート.md`
- `frontend/src/Ports.elm` — `05_技術ノート/Elmポート.md`

#### プロジェクト設定（6 ファイル）
- `CLAUDE.md` — パスリンク 4箇所 + テキスト「技術ノート」→「ナレッジベース」3箇所 + ドキュメント体系テーブルから `08_学習ノート` 行を削除
- `README.md`（ルート） — ドキュメント体系テーブル + ディレクトリツリー
- `.claude/rules/code-comments.md` — パス例 3箇所 + テキスト 2箇所
- `.claude/rules/rust.md` — `06_技術ノート/sqlx-cli.md` → `06_ナレッジベース/rust/sqlx-cli.md`
- `.claude/settings.json` — テキスト「技術ノート」→「ナレッジベース」
- `.claude/skills/walkthrough/SKILL.md` — `08_学習ノート` → `06_ナレッジベース` + 説明文更新

#### docs 配下ドキュメント + README（多数）
- 設計書、ADR、手順書、実装解説内の技術ノートパス参照を一括更新
- `backend/migrations/README.md`, `tests/api/README.md`

#### レシピ（1 ファイル）
- `prompts/recipes/DevAuth開発環境セットアップ.md`

#### 更新しないファイル
- `prompts/runs/`（セッションログ — 歴史的記録）
- `prompts/improvements/`（改善記録 — 歴史的記録）

### Step 4: 検証

```bash
# 古い参照が残っていないか確認（セッションログ・改善記録は除外）
grep -r "06_技術ノート" --include="*.md" --include="*.rs" --include="*.elm" --include="*.js" --include="*.yaml" --include="*.sh" --include="*.json" . | grep -v "prompts/runs/" | grep -v "prompts/improvements/"
grep -r "05_技術ノート" --include="*.md" --include="*.rs" --include="*.elm" --include="*.js" --include="*.yaml" --include="*.sh" --include="*.json" . | grep -v "prompts/runs/" | grep -v "prompts/improvements/"
grep -r "08_学習ノート" . | grep -v "prompts/runs/" | grep -v "prompts/improvements/"

# ビルド・テスト
just check-all
```

## 重要ファイル

| ファイル | 変更理由 |
|---------|---------|
| `CLAUDE.md` | ドキュメント体系の定義、パスリンク、テキスト言及が集中 |
| `.claude/skills/walkthrough/SKILL.md` | 学習ノートパスの変更 + 説明文の整合性 |
| `.claude/rules/code-comments.md` | コードコメント規約内のパス例 |
| `docs/06_ナレッジベース/README.md` | 新 README への書き換え |
| `README.md`（ルート） | ドキュメント体系テーブル |
