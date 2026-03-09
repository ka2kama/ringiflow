# #1091 Api モジュールで ApiResponse のデータアンラップを自動化する

## コンテキスト

### 目的
- Issue: #1091
- Want: API 呼び出し時の `Decode.field "data"` 適用漏れを構造的に防止する（人の注意力への依存を排除）
- 完了基準:
  - `Api.post` / `Api.get` が内部で `Decode.field "data"` を自動適用し、呼び出し側は生のデコーダーを渡すだけで済む
  - 既存の全 API 呼び出し箇所から重複する `Decode.field "data"` を除去する
  - `just check` が通る

### ブランチ / PR
- ブランチ: `feature/1091-auto-unwrap-api-response`
- PR: #1092（Draft）

### As-Is（探索結果の要約）
- `Api.elm`: `get`/`post`/`put`/`patch`/`delete`/`deleteNoContent` を定義。デコーダーをそのまま `handleResponse` に渡す
- `handleResponse`: `Decode.decodeString decoder body` — auto-unwrap なし
- Data モジュール（24箇所）: `detailDecoder`/`listDecoder` で `Decode.field "data"` を手動付与
- `Api/Auth.elm`: `csrfTokenDecoder` = `Decode.at ["data", "token"]`、`userDecoder` = `Decode.field "data" (Decode.map5 ...)`
- `Api/AuditLog.elm`: `PaginatedResponse` を使用（`ApiResponse` ラッパーなし）。`auditLogListDecoder` は `required "data"` でペイロードの `data` フィールドを直接デコード
- テスト: 各 Data モジュールのデコーダーテストが `{"data": ...}` 付きの JSON でテスト

### 進捗
- [x] Phase 1: Api.elm のリファクタリング
- [ ] Phase 2: Data モジュールのデコーダー修正
- [ ] Phase 3: テストの修正

## 仕様整理

### スコープ
- 対象: `Api.elm`、`Api/Auth.elm`、`Api/AuditLog.elm`、全 Data モジュールのデコーダー、関連テスト
- 対象外: BFF 側の変更、`PaginatedResponse` の構造変更、`deleteNoContent` の変更（レスポンスボディなし）

### 操作パス

操作パス: 該当なし（内部リファクタリング。外部動作は変わらない。既存テストで検証）

## 設計

### 設計判断

| # | 判断 | 選択肢 | 選定理由 | 状態 |
|---|------|--------|---------|------|
| 1 | PaginatedResponse の扱い | A: `getRaw` 関数を追加 / B: BFF 側で `ApiResponse` ラップに統一 | A: API 契約を変更せず、影響範囲が Elm 側のみ | 確定 |
| 2 | `detailDecoder` / `listDecoder` の扱い | A: 名前を維持し定義を簡素化 / B: 削除して呼び出し側で構成 | A: 呼び出し側の変更が不要 | 確定 |

### Phase 1: Api.elm のリファクタリング

`handleResponse` で `Decode.field "data"` を自動適用する。`PaginatedResponse` 用に `handleRawResponse`（auto-unwrap なし）を追加し、`getRaw` を公開する。

#### 確認事項
- 型: `handleResponse` の現在のシグネチャ → `Api.elm:340`
- パターン: `deleteNoContent` / `expectNoContent` は変更不要であることを確認（レスポンスボディなし）

#### テストリスト

ユニットテスト（該当なし — `handleResponse` は内部関数で直接テスト不可）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — 既存 E2E で回帰検証）

### Phase 2: Data モジュールのデコーダー修正 + Auth デコーダー修正

全 Data モジュールの `detailDecoder`/`listDecoder` から `Decode.field "data"` を除去。
`Api/Auth.elm` の `csrfTokenDecoder` と `userDecoder` から `"data"` アクセスを除去。
`Api/AuditLog.elm` を `Api.getRaw` に変更。

#### 確認事項
- パターン: 各 Data モジュールの `detailDecoder`/`listDecoder` の定義 → Grep 結果で把握済み
- パターン: `Api/AuditLog.elm` が `Api.get` を使用していること → 確認済み

#### テストリスト

ユニットテスト（該当なし — デコーダー修正は Phase 3 のテスト修正で検証）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: テストの修正

全デコーダーテストの JSON から `{"data": ...}` ラッパーを除去。

#### 確認事項
- パターン: テスト内の JSON 構造 → Grep 結果で把握済み

#### テストリスト

ユニットテスト:
- [ ] 全既存デコーダーテストが `{"data": ...}` なしの JSON で通ること
- [ ] `just check` が通ること

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップ

### ギャップ発見の観点 進行状態

| 観点 | 状態 | メモ |
|------|------|------|
| 未定義 | 完了 | `getRaw` の必要性を特定 |
| 曖昧 | 完了 | Phase 分割と各 Phase の責務が明確 |
| 競合・エッジケース | 完了 | `PaginatedResponse` が唯一のエッジケース、`getRaw` で対応 |
| 不完全なパス | 完了 | `deleteNoContent` は変更不要（レスポンスボディなし） |
| アーキテクチャ不整合 | 完了 | 変更は Elm フロントエンドに閉じる |
| 責務の蓄積 | 完了 | `Api.elm` に ApiResponse ラッパー除去の責務を集約（適切） |
| 既存手段の見落とし | 完了 | Elm の `Decode.field` で十分 |
| テスト層網羅漏れ | 完了 | 既存デコーダーテスト + E2E で回帰検証 |
| 操作パス網羅漏れ | 完了 | 内部リファクタリングのため操作パスなし |

### ループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `PaginatedResponse` が `ApiResponse` と異なるレスポンス形式 | 競合・エッジケース | `getRaw` 関数を追加する設計判断を追加 |

### 未解決の問い
- なし

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Api.elm、全 Data モジュール（24箇所）、Auth、AuditLog、テスト |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更内容が具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | PaginatedResponse の扱い、detailDecoder の扱い |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | BFF 側変更、deleteNoContent を対象外に明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | Decode.field の動作、PaginatedResponse の構造 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | API 契約を変更しないため矛盾なし |
