# CLAUDE.md

このファイルは Claude Code (claude.ai/code) がこのリポジトリで作業する際のガイダンスを提供する。

## 言語

応答は日本語で行う。コメント、ドキュメントも日本語で記述する。

### 段階的英語化

コミットメッセージと PR タイトルは英語で記述する。
それ以外（Issue タイトル、コードコメント、ドキュメント、PR 本文）は日本語。

### プロンプト英訳

ユーザーの日本語プロンプトに対し、応答の冒頭で英訳を添える:

```
💬 Your prompt in English: [自然な英訳]
```

カジュアル〜ニュートラルな表現。短い相槌や返答には不要。意訳可。

## プロジェクト概要

RingiFlow: 承認フロー・タスク管理・ドキュメント管理を一元化するエンタープライズ向けワークフロー管理システム（SaaS）

| レイヤー | 技術 |
|---------|------|
| バックエンド | Rust + axum（BFF / Core Service / Auth Service） |
| フロントエンド | Elm |
| インフラ | AWS（ECS Fargate, Aurora PostgreSQL, ElastiCache Redis, DynamoDB） |
| IaC | Terraform |

## 開発コマンド

| コマンド | 用途 |
|---------|------|
| `just setup` | 初回セットアップ |
| `just dev-all` | 全サーバー一括起動（推奨） |
| `just dev-deps` | PostgreSQL, Redis のみ起動 |
| `just check` | リント + テスト（実装中） |
| `just check-all` | リント + テスト + API テスト + E2E テスト（プッシュ前に必須） |
| `just fmt` | 全体フォーマット |

開発サーバーは `just` コマンドで起動すること（環境変数の設定に必要）。

→ 長時間コマンド: [`.claude/rules/long-running-commands.md`](.claude/rules/long-running-commands.md)
→ 個別サーバー起動・テスト・DB 操作: `just --list` で確認

## コードアーキテクチャ

バックエンド（Rust Workspace）: `backend/apps/`（bff, core-service, auth-service）+ `backend/crates/`（domain, infra, shared）
依存関係: `apps → domain → shared`, `apps → infra → shared`

フロントエンド: Elm + Vite（TEA パターン）。`frontend/src/` 配下に Page/Component/Data/Api 等。

## プロジェクト理念

2 つの理念がすべての作業・設計・判断を駆動する。速度や利便性のために犠牲にしない。

### 理念1: 学習効果の最大化

設計判断を伴う箇所では、意図・代替案・トレードオフを解説する。

### 理念2: 品質の追求

[ISO/IEC 25010](https://iso25000.com/en/iso-25000-standards/iso-25010) に基づく品質追求。→ 詳細: [KB: ISO25010](docs/06_ナレッジベース/methodology/ISO25010.md)

品質戦略:
- Validation: 正しい問題を解いているか → [問題解決フレームワーク](.claude/rules/problem-solving.md)
- Verification: 正しく作っているか → 守り（品質チェックリスト）+ 攻め（[設計原則レンズ](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md#設計原則レンズ)）

→ 詳細: [KB: V&V](docs/06_ナレッジベース/methodology/V&V.md)

### 設計原則

- シンプルさを保つ（KISS）。必要十分な複雑さに留める
- 責務を明確に分離する。一つのモジュール/関数は一つの責務
- 依存関係の方向を意識する。詳細が抽象に依存する構造
- 過度な抽象化・過度な DRY を避ける。3 回繰り返すまでは重複を許容
- 型で表現できるものは型で表現する。不正な状態を表現不可能にする
- YAGNI/KISS/MVP は**機能スコープ**の原則。**設計品質**を妥協する根拠にしない。判定: 「放置すると後続で同パターンが使われるか？」→ Yes なら設計品質の問題

### ベストプラクティス起点

判断はその分野のベストプラクティスを起点とし、逸脱する場合は理由を記録する（ADR、Issue、コメント）。

→ [最新ベストプラクティス採用方針](.claude/rules/latest-practices.md)
→ [方法論・プロセス設計の方針](.claude/rules/methodology-design.md)

## 技術選定・設計判断の原則

判断の優先順位:

1. セキュリティ要件への適合
2. プロジェクト理念（学習効果、品質追求）との整合
3. 非機能要件（可用性、パフォーマンス等）の充足
4. 技術的なシンプルさ・効率

設計判断の前に関連ドキュメント（要件定義書、ADR、設計書）を確認すること。新しい判断は ADR に記録する。

**禁止:**

- 既存ドキュメントを確認せずに設計判断を提案すること
- 過去の ADR で却下された選択肢を同じ文脈で再提案すること（新しい情報がある場合を除く）

## 俯瞰・実装リズム

全体（俯瞰）と局所（実装）を往復する。→ 詳細: [`.claude/rules/zoom-rhythm.md`](.claude/rules/zoom-rhythm.md)

| タイミング | 俯瞰の内容 |
|-----------|-----------|
| TDD の Refactor ステップ | [設計原則レンズ](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md#設計原則レンズ)で設計品質を評価 |
| 予期しないエッジケース発見時 | 局所対処か全体設計への統合かの判断 |
| Phase の区切り | 全レンズ + 品質チェックリストで全体確認 |

## 問題解決のアプローチ

問題や指摘を受けたとき、修正案を出す前に分析する。→ 詳細: [`.claude/rules/problem-solving.md`](.claude/rules/problem-solving.md)

1. Want（本質） → 2. To-Be（理想） → 3. As-Is（現状） → 4. ギャップ分析 → 5. 根本原因 → 6. 対策設計

→ 技術的問題: [`.claude/rules/troubleshooting.md`](.claude/rules/troubleshooting.md)

## 運用サイクル

3つのスキルが連携してフィードバックループを形成する: `/assess`（月次診断）→ `/retro`（週次検証）→ Issue 化 → `/next`（セッション毎の作業選定）→ 実装 → `/assess` ...

| スキル | 頻度 | 役割 |
|-------|------|------|
| `/assess` | 月次 | Discovery / Delivery / Sustainability の3軸で現状診断 |
| `/retro` | 週次 | 改善記録の有効性検証、トレンド分析 |
| `/next` | セッション毎 | GitHub Issues から次の作業を選定 |

`/next` は GitHub Issues のみをデータソースとする。`/assess` と `/retro` のアクションアイテムは Issue 化することで `/next` に接続される。

## ドキュメント体系

すべての知識はコード・実装・文書で明示する。暗黙知を許容しない。

| 知りたいこと | 参照先 |
|-------------|--------|
| 要件・機能仕様 | [`docs/01_要件定義書/`](docs/01_要件定義書/) |
| 設計（基本/詳細） | [`docs/02_基本設計書/`](docs/02_基本設計書/) / [`docs/03_詳細設計書/`](docs/03_詳細設計書/) |
| 操作手順 | [`docs/04_手順書/`](docs/04_手順書/) |
| 意思決定 | [`docs/05_ADR/`](docs/05_ADR/) |
| 技術知識 / 実装解説 / テスト | [`docs/06_ナレッジベース/`](docs/06_ナレッジベース/) / [`docs/07_実装解説/`](docs/07_実装解説/) / [`docs/08_テスト/`](docs/08_テスト/) |
| セッションログ | [`prompts/runs/`](prompts/runs/) |
| 改善・調査記録 | [`process/`](process/) |

作業開始時は [`docs/01_要件定義書/00_はじめに.md`](docs/01_要件定義書/00_はじめに.md) から読むこと。

### 情報管理: ローカルファースト

→ [ADR-025](docs/05_ADR/025_情報管理とローカル知識集約の方針.md)

GitHub Issues/PR は一時的なワークフロー用。判断・学びはローカル docs に記録する。

### AI エージェントの手順案内

- 操作手順を聞かれたら `justfile` または手順書を参照するよう案内する
- 会話の中で手順を独自に生成・要約しない
- 新しい手順が必要なら、justfile か手順書に追記する

### ナレッジベースの活用

コード内のコメントは簡潔に、詳細解説はナレッジベースに書いてコードからリンクする。

### ドキュメント自動作成ルール

セッション中に該当する活動があれば対応ドキュメントを自発的に作成する。`/wrap-up` 実行時にも振り返る。

**禁止:** 該当する活動があったにもかかわらず、ドキュメントを作成しないこと

| 活動 | ドキュメント | 出力先 |
|------|------------|--------|
| 技術選定・方針選択・見送り | ADR | `docs/05_ADR/` |
| 新ツール・パターン導入 | ナレッジベース | `docs/06_ナレッジベース/` |
| コード変更・設計判断 | セッションログ | `prompts/runs/` |
| PR 単位の機能解説 | 実装解説（`/explain`） | `docs/07_実装解説/` |
| 非自明な問題解決 | 操作レシピ | `prompts/recipes/` |
| プロセスの問題と対策 | 改善記録 | `process/improvements/` |
| 仮説検証 2 回以上の調査 | 調査記録 | `process/investigations/` |

## 学習支援

設計判断を伴う箇所（アーキテクチャ、パターン選択、トレードオフ）では解説を提供する。単純な CRUD や定型修正では不要。

- 内容: 意図、代替案、トレードオフ、関連知識
- 想定レベル: 中級者。注力: アーキテクチャ（DDD, CQRS）、Elm/TEA、設計原則
- 複数選択肢の提示時は推奨を明示する（Yes/No には不要）

Insight ブロック形式:

```
★ Insight ─────────────────────────────────────
[日本語の教育的ポイント 2-3 点]

📝 In English: [1-2 sentence summary of the key takeaway]
─────────────────────────────────────────────────
```

## Issue 駆動開発

機能実装前に必ず [手順書](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md) を読み、記載されたフローに従うこと。

**禁止:** 手順書を読まずに実装を開始すること

設計フェーズ完了後は [TDD 開発フロー](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md) に従い、テストから書き始めること。

**禁止:** テストを書かずにプロダクションコードを書き始めること

コードを書く前に、関連する型定義と既存パターンを確認する。→ [`.claude/rules/pre-implementation.md`](.claude/rules/pre-implementation.md)

## 破壊的操作の判定

→ 経緯: [改善記録: ユーザー確認なしに破壊的操作を実行した](process/improvements/2026-02/2026-02-15_1600_ユーザー確認なしに破壊的操作を実行した.md)

Claude Code のシステムプロンプト（「Executing actions with care」）を補完する、プロジェクト固有の判定基準。

破壊的操作を実行する前に、以下のチェックリストで判定する:

| # | 確認項目 | 判定基準 |
|---|---------|---------|
| 1 | 操作は可逆か？ | 削除、`--force`、上書きは不可逆 → 確認必要 |
| 2 | データ損失の可能性は？ | 未コミットの変更、未 push のコミットが失われる可能性 → 確認必要 |
| 3 | ユーザーの意図は明示的か？ | 質問と依頼を区別する。意図が曖昧なら確認必要 |

いずれかに該当する場合、実行前にユーザーに確認を求める。

**禁止:** ユーザーの発言を推測で解釈し、確認なしに破壊的操作を実行すること

## Git 操作ルール

→ 詳細手順: [手順書: Issue 駆動開発](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md)
→ ブランチ戦略: [ADR-046](docs/05_ADR/046_Story-per-PRブランチ戦略.md)

### ブランチ命名

```bash
git checkout -b feature/Story番号-機能名   # 新機能（Story 単位）
git checkout -b fix/Issue番号-バグ名       # バグ修正
```

**禁止:**

- main ブランチで直接作業・コミット（ドキュメントのみの変更でも例外なし）
- Epic 番号でブランチを作成すること（ブランチは Story 単位）

### コミットメッセージ

```bash
git commit -m "#34 Implement find_by_email for UserRepository"
```

lefthook により、ブランチ名が `feature/34-xxx` 形式なら Issue 番号は自動付与される。

### PR ワークフロー

→ 作成: [手順書 #3](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md#3-draft-pr-を作成)
→ 完了フロー: [手順書 #6](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md#6-品質ゲートと-ready-for-review)

PR 本文の形式:
- 先頭に `## Issue` セクション: Story PR は `Closes #<Story番号>`、参照のみは `Related to #123`
- Epic に対して `Closes` は使用しない
- AI エージェントは `--body` で本文を直接指定し、末尾に署名: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`

**禁止:**

- Draft なしで PR を作成すること
- wrap-up 完了を検証せずに Ready にすること
- 品質ゲートを通過せずに Ready for Review に戻すこと
- `--admin` で CI バイパス、CI 失敗状態での強制マージ、ユーザー指示なしのマージ

## PR レビュー

Claude Code Action による自動 PR レビューが有効。→ [`.github/workflows/claude-auto-review.yaml`](.github/workflows/claude-auto-review.yaml)

承認基準: Critical/High は修正必須（request-changes）、Medium/Low は改善推奨だがマージ可能（approve + コメント）。

```bash
gh pr checks && gh pr view --comments  # 指摘対応フロー
gh pr merge --squash                   # 対応後マージ
```
