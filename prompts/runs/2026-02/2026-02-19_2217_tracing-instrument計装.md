# tracing::instrument 計装の導入

Issue: #654（Epic #648 Observability 基盤 / Story 4）
PR: #692

## 概要

`tracing::instrument` マクロを全アプリケーションスタック（ハンドラ、BFF HTTP クライアント、リポジトリ、セッションマネージャ）に導入し、関数レベルのスパン構造を確立した。

## 実施内容

### Phase 1: ハンドラ計装

BFF（14 ハンドラ）、Core Service（20 ハンドラ）、Auth Service（3 ハンドラ）に `#[tracing::instrument]` を追加。

パターン分類:
- パラメータなし → `skip_all` のみ
- `Path(i64)` → `skip_all, fields(display_number)`
- `Path(Uuid)` → `skip_all, fields(%id)`
- `Path(StepPathParams)` → `skip_all, fields(display_number = params.display_number, ...)`

### Phase 2: BFF クライアント計装

5 つのクライアントファイル（34 メソッド）に DEBUG レベルで計装。

PII 排除ルール:
- email, password, credential_data → fields に含めない
- tenant_id, user_id 等の ID → `%` プレフィックスで記録

### Phase 3: インフラ層計装

10 ファイル（~53 メソッド）に DEBUG レベルで計装。

- PostgreSQL リポジトリ: 検索系は ID を記録、挿入/更新系はエンティティを skip
- Redis セッション: SessionData は PII 含むため create は fields なし
- DynamoDB 監査ログ: record は fields なし、find は tenant_id を記録

### Phase 4: 品質ゲート

`just check-all` 全通過（421 unit tests + 158 integration tests、clippy、fmt、sqlx prepare check、cargo deny）。

## 判断ログ

- `skip_all` パターンを全関数で採用。個別 skip より PII 漏洩リスクがゼロで、新フィールド追加時にも安全。Story #651 の PII マスキング方針と一貫
- ハンドラは INFO（デフォルト）、クライアント・リポジトリは DEBUG レベルに設定。デフォルトフィルタ `info,ringiflow=debug` で ringiflow モジュールの DEBUG スパンも記録される
- newtype ID は `%`（Display）で記録、i64 は prefix なし（Debug）で記録
- `health_check` を対象外とした。ロードバランサーの高頻度呼び出しでノイズになるため
- ユースケース層を対象外とした。ハンドラ（入口）とリポジトリ（出口）のスパンで処理フロー追跡に十分

## 成果物

### コミット

- `#654 Add tracing::instrument to handlers, clients, and repositories`（32 ファイル、+540 行）

### 計画ファイル

- `prompts/plans/654_tracing-instrument.md`
