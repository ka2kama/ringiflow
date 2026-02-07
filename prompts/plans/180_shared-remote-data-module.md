# #180 共有 RemoteData モジュールの抽出

## 概要

6つのページモジュールに独立定義されている `RemoteData` 型を `frontend/src/RemoteData.elm` に統一する。

## 設計判断

### 統一型（2型パラメータ）

```elm
type RemoteData e a
    = NotAsked
    | Loading
    | Failure e
    | Success a
```

- 3-variant (4ページ) と 4-variant (2ページ) を統一
- エラー型をパラメータ化し、全ページで `ApiError` を保持するよう改善
- 現在エラー情報を破棄しているページ (`Err _ -> Failure`) も `ApiError` を保持

### ユーティリティ関数

| 関数 | 型 | 用途 |
|------|-----|------|
| `map` | `(a -> b) -> RemoteData e a -> RemoteData e b` | 成功値の変換 |
| `withDefault` | `a -> RemoteData e a -> a` | デフォルト値の提供 |
| `toMaybe` | `RemoteData e a -> Maybe a` | Maybe への変換 |
| `fromResult` | `Result e a -> RemoteData e a` | Result → RemoteData 変換 |
| `isLoading` | `RemoteData e a -> Bool` | ローディング判定 |

`andThen`, `mapError` は YAGNI により今回は見送り。

### モジュール配置

`frontend/src/RemoteData.elm`（トップレベル。ドメイン非依存の汎用型）

## 実装計画

### Phase 1: RemoteData モジュール実装（TDD）

テストファイル: `frontend/tests/RemoteDataTest.elm`

**テストリスト:**
- [ ] `map` - NotAsked/Loading/Failure はそのまま、Success は変換
- [ ] `withDefault` - Success 以外はデフォルト値、Success は中身を返す
- [ ] `toMaybe` - Success は Just、それ以外は Nothing
- [ ] `fromResult` - Ok → Success、Err → Failure
- [ ] `isLoading` - Loading のみ True

実装ファイル: `frontend/src/RemoteData.elm`

コミット: `Implement shared RemoteData module with core utilities`

### Phase 2: 全ページの移行

対象6ファイル:

| ファイル | 現状 | 移行後のエラー型 |
|---------|------|-----------------|
| `Page/Home.elm` | 3-variant, `Err _` | `RemoteData ApiError DashboardStats` |
| `Page/Task/List.elm` | 3-variant, `Err _` | `RemoteData ApiError (List TaskItem)` |
| `Page/Task/Detail.elm` | 3-variant, `Err _` | `RemoteData ApiError TaskDetail` |
| `Page/Workflow/List.elm` | 3-variant, `Err _` | `RemoteData ApiError (List WorkflowInstance)` |
| `Page/Workflow/Detail.elm` | 4-variant, `Err _` | `RemoteData ApiError WorkflowInstance` 等 |
| `Page/Workflow/New.elm` | 4-variant, `Failure ApiError` | `RemoteData ApiError (List WorkflowDefinition)` |

各ページの変更パターン:

1. `import RemoteData exposing (RemoteData(..))` を追加
2. ローカル `type RemoteData a = ...` を削除
3. Model の型注釈に `ApiError` パラメータを追加
4. `Err _ -> Failure` を `Err err -> Failure err` に変更
5. view の `Failure ->` を `Failure _ ->` に変更（エラー表示は従来通り）
6. `fromResult` で簡略化できる箇所は適用

検証: `just check-all`（Elm コンパイラの型チェック + 既存テスト）

コミット: `Migrate all pages to shared RemoteData module`

## 対象ファイル一覧

**新規作成:**
- `frontend/src/RemoteData.elm`
- `frontend/tests/RemoteDataTest.elm`

**変更:**
- `frontend/src/Page/Home.elm`
- `frontend/src/Page/Task/List.elm`
- `frontend/src/Page/Task/Detail.elm`
- `frontend/src/Page/Workflow/List.elm`
- `frontend/src/Page/Workflow/Detail.elm`
- `frontend/src/Page/Workflow/New.elm`

## 検証方法

1. `just test-elm` — RemoteData ユニットテスト + 既存テスト
2. `just check-all` — lint + テスト + ビルド
3. `just dev-all` で動作確認（各ページの Loading → Success/Failure 遷移）
