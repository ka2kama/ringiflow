# CLAUDE.md

このファイルは Claude Code (claude.ai/code) がこのリポジトリで作業する際のガイダンスを提供する。

## 言語

応答は日本語で行う。コメント、ドキュメントも日本語で記述する。

### 段階的英語化

開発ワークフローを通じた英語力向上のため、以下は英語で記述する:

- **コミットメッセージ**: 英語で記述する
- **PR タイトル**: 英語で記述する（squash merge でコミットメッセージになるため）

それ以外（Issue タイトル、コードコメント、ドキュメント、PR 本文）は引き続き日本語。Issue タイトルは一覧でのスキャナビリティを優先し、日本語で記述する。

### プロンプト英訳

ユーザーの日本語プロンプトに対し、応答の冒頭で英訳を添える:

```
💬 Your prompt in English: [自然な英訳]
```

- 基本はカジュアル〜ニュートラル（日常で使える表現）
- 堅すぎる場合は口語的な言い換えも併記してよい
- 短い相槌や「はい」「いいえ」程度の返答には不要
- 意訳で構わない。自然な英語表現を優先する
- ユーザーが英語の意味を質問した場合や、日本語での説明を求めた場合は日本語で応答してよい

## プロジェクト概要

RingiFlow: 承認フロー・タスク管理・ドキュメント管理を一元化するエンタープライズ向けワークフロー管理システム（SaaS）

| レイヤー | 技術 |
|---------|------|
| バックエンド | Rust + axum（BFF / Core Service / Auth Service） |
| フロントエンド | Elm |
| インフラ | AWS（ECS Fargate, Aurora PostgreSQL, ElastiCache Redis） |
| IaC | Terraform |

## 開発コマンド

```bash
just setup              # 初回セットアップ
just dev-deps           # PostgreSQL, Redis を起動
just check              # リント + テスト（実装中の軽量チェック）
just check-all          # リント + テスト + API テスト + E2E テスト（プッシュ前に必須）
just fmt                # 全体フォーマット
```

→ 長時間コマンドの実行時の注意: [`.claude/rules/long-running-commands.md`](.claude/rules/long-running-commands.md)

### 開発サーバー起動

開発サーバーは必ず `just` コマンドで起動する。直接 `pnpm run dev` や `cargo run` を実行してはいけない（環境変数が設定されないため）。

| コマンド | 用途 |
|---------|------|
| `just dev-all` | 全サーバー一括起動（推奨。PostgreSQL/Redis + BFF/Core/Auth/Web） |
| `just dev-down` | 依存サービス停止 |
| `just dev-deps` | PostgreSQL, Redis のみ起動 |
| `just dev-bff` / `just dev-core-service` / `just dev-auth-service` / `just dev-web` | 個別起動 |

→ 詳細: [ナレッジベース: Vite](docs/06_ナレッジベース/frontend/Vite.md#環境変数管理)

### テスト・データストア操作

```bash
# 単一テスト
cd backend && cargo test テスト名
cd frontend && pnpm run test -- --watch tests/Example.elm

# 統合テスト（DB 接続が必要）
just test-rust-integration

# データストア操作（MCP も利用可能）
just db-tables / just db-schema テーブル名 / just db-query "SELECT ..."
just db-migrate             # マイグレーション + スナップショット更新
just redis-keys / just redis-get キー名
```

## コードアーキテクチャ

### バックエンド（Rust Workspace）

```
backend/
├── apps/
│   ├── bff/            # BFF（セッション管理、API プロキシ）
│   ├── core-service/   # Core Service（ビジネスロジック）
│   └── auth-service/   # Auth Service（認証）
└── crates/
    ├── domain/         # ドメインモデル
    ├── infra/          # インフラ層（DB、Redis）
    └── shared/         # 共有ユーティリティ
```

依存関係: `apps → domain → shared`, `apps → infra → shared`

### フロントエンド（Elm + Vite）

```
frontend/src/
├── Main.elm            # エントリポイント
├── Route.elm           # ルーティング
├── Page/               # ページモジュール
└── Ports.elm           # JavaScript 連携
```

TEA（The Elm Architecture）パターンを採用。

## プロジェクト理念

2つの確固たる理念がある。すべての作業・設計・判断においてこれを最優先する。

### 理念1: 学習効果の最大化

このプロジェクトはオーナーの技術的学びを深める場である。

- 設計判断を伴う箇所では、なぜその選択をしたか解説する
- 代替案とトレードオフを示し、思考プロセスを共有する
- 応用パターンやベストプラクティスの観点から説明する

### 理念2: 品質の追求

[ISO/IEC 25010](https://iso25000.com/en/iso-25000-standards/iso-25010) のプロダクト品質モデルに基づき、ソフトウェア品質を体系的に追求する。

重点品質特性: 保守性（モジュール性、修正性、試験性）、機能適合性（完全性、正確性、適切性）、セキュリティ（機密性、完全性、真正性）。段階的に信頼性（Phase 2〜）、操作性（Phase 2〜）、性能効率性（Phase 3〜）に取り組む。

品質戦略は Validation（正しい問題を解いているか — [問題解決フレームワーク](.claude/rules/problem-solving.md)、[Issue 精査](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md#既存-issue-の精査)）と Verification（正しく作っているか）の2層構成。Verification には守り（欠陥除去: 設計レビュー、品質チェックリスト）と攻め（設計改善: TDD Refactor の[設計原則レンズ](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md#設計原則レンズ)）がある。

設計原則:

- シンプルさを保つ（KISS）。必要十分な複雑さに留める
- 責務を明確に分離する。一つのモジュール/関数は一つの責務
- 依存関係の方向を意識する。詳細が抽象に依存する構造
- 過度な抽象化・過度な DRY を避ける。3回繰り返すまでは重複を許容

YAGNI/KISS/MVP は**機能スコープ**の原則であり、**設計品質**を妥協する根拠にならない。判定テスト: 「この判断を放置すると、後続の実装で同じパターンが使われるか？」→ Yes なら設計品質の問題（割れ窓理論）。

型システムの活用: 型で表現できるものは型で表現する。不正な状態を表現不可能にする。安易な unwrap / expect を避ける。
コードの明確さ: 意図が伝わる命名。コメントは「なぜ」を書く。

これらの理念は「あれば良い」ではなく**必須**。速度や利便性のために犠牲にしない。

### 2つの理念が駆動するもの

| 理念 | 駆動するもの | 具体例 |
|------|-------------|--------|
| 理念1: 学習効果 | 解説・教育コンテンツ | 実装解説、ナレッジベース、Insight ブロック、プロンプト英訳 |
| 理念2: 品質追求 | 上記以外のすべて | プロセス、テスト、セキュリティ、ドキュメント体系、etc. |
| 両方 | 判断の記録 | ADR（追跡可能性 + 代替案から学ぶ）、セッションログ |

理念2 の品質追求が駆動するプロセス群は、学習のために過剰に設けているものではない。AI エージェント主導で商用プロダクトを開発・運用する際に必要な品質保証の水準として設計している。

### 共通アプローチ: ベストプラクティス起点

**あらゆる判断において、その分野の業界ベストプラクティスを起点（デフォルト）とし、プロジェクトの現実に合わせて意識的に調整する。**

- **起点を高く置く**: 「何もないところから足す」のではなく「ベストプラクティスから始めて調整する」
- **全領域に適用**: コード設計、UI/UX、アクセシビリティ、セキュリティ、テスト、検証、プロジェクト運営——例外なし
- **意識的な調整**: ベストプラクティスから外れるときは理由を記録する（ADR、Issue、コメント）。記録のない逸脱は許容しない
- **知って判断する**: 「知らなかったから従わなかった」は許容しない。まず調べ、知った上で判断する

→ 技術・ツール領域での具体化: [最新ベストプラクティス採用方針](.claude/rules/latest-practices.md)
→ 方法論・プロセス設計での具体化: [方法論・プロセス設計の方針](.claude/rules/methodology-design.md)
→ 収束の方法論: [俯瞰・実装リズム](.claude/rules/zoom-rhythm.md)の理想駆動

## 技術選定・設計判断の原則

### 1. 理念・要件への合致を最優先

判断の優先順位:

1. セキュリティ要件への適合
2. プロジェクト理念（学習効果、品質追求）との整合
3. 非機能要件（可用性、パフォーマンス等）の充足
4. 技術的なシンプルさ・効率

### 2. 既存ドキュメントの確認を必須とする

設計判断の前に、関連する既存ドキュメントを必ず確認する:

- 要件定義書（特にセキュリティ要件、非機能要件）
- 関連する ADR（過去の意思決定）
- 関連する設計書（基本設計、詳細設計）

**禁止事項:**

- 既存ドキュメントを確認せずに設計判断を提案すること
- 過去の ADR で却下された選択肢を、同じ文脈で再提案すること（新しい情報がある場合を除く）

### 3. 決定の文書化

新しい技術選定・設計判断を行った場合は、必ず ADR に記録する。

## 俯瞰・実装リズム

良いプログラミングは、設計と実装を忙しなく往復する。視点の高度を上げ下げしながら、全体と局所を行き来するリズムを大事にする。

| タイミング | 俯瞰の内容 |
|-----------|-----------|
| TDD の Refactor ステップ | [設計原則レンズ](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md#設計原則レンズ)で設計品質を評価 |
| 予期しないエッジケース発見時 | 局所対処か全体設計への統合かの判断 |
| Phase の区切り | 全レンズ + 品質チェックリストで全体確認 |

往復を繰り返した結果、理想状態（To-Be）と現状（As-Is）のギャップがゼロになることで収束する。

→ 詳細: [`.claude/rules/zoom-rhythm.md`](.claude/rules/zoom-rhythm.md)

**禁止:** 視点を上げずに実装だけを続けること、収束確認を実施せずに成果物を提示・コミットすること

## 問題解決のアプローチ

問題や指摘を受けたとき、すぐに修正案を出さない。まず腰を据えて考える。

1. **Want（本質）**: ユーザーが本当に望んでいることは何か？
2. **To-Be（理想状態）**: どうあるべきだったか？
3. **As-Is（現状）**: 実際に何が起きたか？
4. **ギャップ分析**: なぜ理想と現実にずれが生じたか？
5. **根本原因**: 表層的な原因の奥にある構造的な問題は何か？
6. **対策設計**: 根本原因を解消し、Want を満たす対策は何か？

How（具体的な対策）にこだわりすぎず、常に Want を満たすかを検証する。

→ 詳細: [`.claude/rules/problem-solving.md`](.claude/rules/problem-solving.md)
→ 技術的問題の調査: [`.claude/rules/troubleshooting.md`](.claude/rules/troubleshooting.md)

**禁止:** このフレームワークを経ずに対策を提示すること

## 運用サイクル

3つのスキルが連携してフィードバックループを形成する: `/assess`（月次診断）→ `/retro`（週次検証）→ Issue 化 → `/next`（セッション毎の作業選定）→ 実装 → `/assess` ...

| スキル | 頻度 | 役割 |
|-------|------|------|
| `/assess` | 月次 | Discovery / Delivery / Sustainability の3軸で現状診断 |
| `/retro` | 週次 | 改善記録の有効性検証、トレンド分析 |
| `/next` | セッション毎 | GitHub Issues から次の作業を選定 |

`/next` は GitHub Issues のみをデータソースとする。`/assess` と `/retro` のアクションアイテムは Issue 化することで `/next` に接続される。

## ドキュメント体系

このプロジェクトは一切の暗黙知を許容しない。すべての知識はコード・実装・文書で明示する。

| 知りたいこと | 参照先 |
|-------------|--------|
| 要件（WHAT） | [`docs/01_要件定義書/`](docs/01_要件定義書/) |
| 機能仕様（WHAT） | [`docs/01_要件定義書/機能仕様書/`](docs/01_要件定義書/機能仕様書/) |
| 全体設計（HOW） | [`docs/02_基本設計書/`](docs/02_基本設計書/) |
| 実装設計（HOW） | [`docs/03_詳細設計書/`](docs/03_詳細設計書/) |
| 操作手順 | [`docs/04_手順書/`](docs/04_手順書/) |
| 意思決定（WHY） | [`docs/05_ADR/`](docs/05_ADR/) |
| 技術知識 / 実装解説 / テスト | [`docs/06_ナレッジベース/`](docs/06_ナレッジベース/) / [`docs/07_実装解説/`](docs/07_実装解説/) / [`docs/08_テスト/`](docs/08_テスト/) |
| 設計思考過程 / セッションログ | [`prompts/plans/`](prompts/plans/) / [`prompts/runs/`](prompts/runs/) |
| 改善記録 / 診断レポート | [`process/improvements/`](process/improvements/) / [`process/reports/`](process/reports/) |

作業開始時は [`docs/01_要件定義書/00_はじめに.md`](docs/01_要件定義書/00_はじめに.md) から読み、全体像を把握すること。

### 情報管理の原則: ローカルファースト

→ 詳細: [ADR-025](docs/05_ADR/025_情報管理とローカル知識集約の方針.md)

GitHub Issues/PR は一時的なワークフロー用、ローカル docs は永続的な知識用。Issue/PR 内での長い議論は避け、判断・学びはローカルドキュメントに記録する。

### AI エージェントが手順を案内する場合

- 操作手順を聞かれたら `justfile` または手順書を参照するよう案内する
- 会話の中で手順を独自に生成・要約しない
- 新しい手順が必要なら、justfile か手順書に追記して形式知化する

### ナレッジベースの活用

コード内のコメントは簡潔に、詳細解説はナレッジベースに書いてコードからリンクする。

### ドキュメント自動作成ルール

セッション中に以下に該当する活動があった場合、対応ドキュメントを自発的に作成する。`/wrap-up` 実行時にも振り返る。

**禁止:** 該当する活動があったにもかかわらず、ドキュメントを作成しないこと

- **ADR** — 技術選定、実装方針の選択、見送りの判断
- **ナレッジベース** — 新しいツール・パターン導入、技術解説
- **セッションログ** — コード変更、設計判断（→ [`prompts/runs/`](prompts/runs/)）
- **実装解説** — PR 単位の機能解説。`/explain` で生成（→ [`docs/07_実装解説/`](docs/07_実装解説/)）
- **操作レシピ** — 非自明な操作で問題解決（→ [`prompts/recipes/`](prompts/recipes/)）
- **改善記録** — 開発プロセスの問題と対策（→ [`process/improvements/`](process/improvements/)）

## 学習支援

設計判断を伴う箇所（アーキテクチャ決定、パターン選択、非自明なロジック、トレードオフのある選択）では、コードとともに解説を提供する。単純な CRUD や定型修正では不要。

解説の内容: 意図（なぜこの設計か）、代替案（なぜ不採用か）、トレードオフ、関連知識。
想定レベル: 中級者（基礎は理解済み）。特に注力: アーキテクチャ設計（DDD, CQRS）、Elm/TEA パターン、ソフトウェア設計原則。

複数選択肢を提示する際は推奨オプションを明示し、判断材料と理由を提供する。Yes/No のクローズドクエスチョンには適用しない。

Insight ブロックには日本語の解説に加えて 1〜2 文の英語サマリーを必ず付ける:

```
★ Insight ─────────────────────────────────────
[日本語の教育的ポイント 2-3 点]

📝 In English: [1-2 sentence summary of the key takeaway]
─────────────────────────────────────────────────
```

## Issue 駆動開発

**AI エージェントへの必須事項:**

機能実装の依頼を受けたら、**コードを書く前に**必ず [手順書](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md) を読み、記載されたフローに従う。

**禁止:** 手順書を読まずに実装を開始すること

設計フェーズ完了後、実装フェーズに入る際は必ず [TDD 開発フロー](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md) を参照し、**テストから書き始める**こと。

**禁止:** テストを書かずにプロダクションコードを書き始めること

コードを書く前に、関連する型定義と既存パターンを必ず確認する。推測で書かない。

→ 詳細: [`.claude/rules/pre-implementation.md`](.claude/rules/pre-implementation.md)

→ 詳細: [手順書: Issue 駆動開発](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md)

1. Issue を確認または作成し、前提を精査する
2. Story 番号を含むブランチを作成（例: `feature/34-user-auth`）
3. 実装（[TDD](docs/04_手順書/04_開発フロー/02_TDD開発フロー.md)）
4. PR を作成し `Closes #34` で紐づけ（Story 単位）
5. マージで Story Issue 自動クローズ

補足:
- 機能実装前に対応 Issue の存在を確認
- ブランチ名には Story 番号を含める（Epic 番号ではない）
- Phase やタスク完了時は Issue のチェックボックスを更新

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

### ブランチ戦略（GitHub Flow + Story-per-PR）

→ 意思決定: [ADR-046](docs/05_ADR/046_Story-per-PRブランチ戦略.md)

各 Story を個別 PR で main にマージする。Integration branch（develop 等）は使用しない。

```bash
git checkout -b feature/Story番号-機能名   # 新機能（Story 単位）
git checkout -b fix/Issue番号-バグ名       # バグ修正
```

**禁止:**

- main ブランチで直接作業・コミット（変更の種類によらず例外なし。ドキュメントのみの変更でもブランチを作成する）
- Epic 番号でブランチを作成すること（ブランチは Story 単位）

### コミットメッセージ

```bash
git commit -m "#34 Implement find_by_email for UserRepository"
```

lefthook により、ブランチ名が `feature/34-xxx` 形式なら Issue 番号は自動付与される。

### PR 作成（Draft）

→ 詳細: [手順書: Draft PR を作成](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md#3-draft-pr-を作成)

```bash
git commit --allow-empty -m "#34 WIP: Implement login feature"
git push -u origin HEAD
gh pr create --draft --title "#34 Implement login feature" --body-file .github/pull_request_template.md
```

PR 本文の形式:
- 先頭に `## Issue` セクション: Story PR は `Closes #<Story番号>`、参照のみは `Related to #123`、Issue なしは `なし`
- Epic に対して `Closes` は使用しない（Epic は全サブ Issue 完了後に手動クローズ）
- AI エージェントは `--body` でテンプレート形式の本文を直接指定し、末尾に署名: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`

Test plan: 単一 PR は実装テスト＋手動テスト手順を記載。Epic の Story PR は各 Story のテスト手順を記載。

### PR 完了フロー

実装完了から Ready for Review までの手順。順番を守ること。

1. 実装完了、`just check-all` 通過
2. 収束確認完了（[zoom-rhythm.md](.claude/rules/zoom-rhythm.md)）
3. 計画ファイル確認（`git status` で `prompts/plans/` に未コミットファイルがあれば、現在の作業との関連を確認してコミットする）
4. Draft PR 作成（`gh pr create --draft`）
5. `/wrap-up` でドキュメント整備
6. wrap-up 完了の検証（`git -c core.quotepath=false diff --name-only main...HEAD | grep -E "^(prompts/runs/|process/improvements/|prompts/recipes/|docs/05_ADR/|docs/06_|docs/07_)"`）。なければ `/wrap-up` を実行
7. base branch 同期確認（`git fetch origin main && git log HEAD..origin/main --oneline`、差分あれば rebase + `just check-all` で再確認）
8. ユーザーに確認を求める（「Ready にしてよいですか？」）
9. ユーザー承認後、`gh pr ready` で Ready にする

**禁止:**

- Draft なしで PR を作成すること
- ユーザー確認なしに `gh pr ready` を実行すること
- wrap-up 完了を検証せずに Ready を提案すること

### Draft に戻した後、再度 Ready にする場合

Draft に戻した = 品質保証がリセットされた状態。コミットの作成者（AI / ユーザー）に関わらず、品質ゲートは必須。

1. 修正・追加コミットを実施
2. `just check-all` で品質ゲート通過
3. base branch 同期確認（差分があれば rebase + `just check-all` で再確認）
4. ユーザーに確認を求める（「Ready にしてよいですか？」）
5. ユーザー承認後、`gh pr ready` で Ready にする

**禁止:** 品質ゲートを通過せずに Ready for Review に戻すこと

### Ready for Review・マージ

```bash
just check-all  # lint + test + API test
gh pr ready     # Draft を解除
```

→ 詳細チェックリスト: [手順書: 品質ゲートと Ready for Review](docs/04_手順書/04_開発フロー/01_Issue駆動開発.md#6-品質ゲートと-ready-for-review)

```bash
gh pr merge --squash
just clean-branches  # マージ後のブランチ削除（worktree のリセット含む）
```

**禁止:** `--admin` で CI バイパス、CI 失敗状態での強制マージ

## PR レビュー

Claude Code Action による自動 PR レビューが有効。→ [`.github/workflows/claude-auto-review.yaml`](.github/workflows/claude-auto-review.yaml)

承認基準: Critical/High は修正必須（request-changes）、Medium/Low は改善推奨だがマージ可能（approve + コメント）。

```bash
gh pr checks && gh pr view --comments  # 指摘対応フロー
gh pr merge --squash                   # 対応後マージ
```
