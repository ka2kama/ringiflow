# Elm コンポーネント設計

## 概要

Elm コミュニティでは「コンポーネント」という概念に対して慎重な姿勢がある。Elm の作者 Evan Czaplicki は [The Life of a File](https://www.youtube.com/watch?v=XpDsk374LDE)（Elm Europe 2017）や[公式ガイド](https://guide.elm-lang.org/webapps/structure.html)で、「コンポーネント思考」を明確に戒めている。

> "Actively trying to make components is a recipe for disaster in Elm."
> — [Elm Guide: Structure](https://guide.elm-lang.org/webapps/structure.html)

このドキュメントでは、Elm の哲学、プロジェクトでの設計判断、その根拠を整理する。

## Elm の公式スタンス

### 避けるべきもの: コンポーネント = ミニ TEA

Elm で「コンポーネント」が問題になるのは、React のように**各 UI 部品が独自の Model/Msg/update を持つ**パターン。これを「Nested TEA をコンポーネントに適用する」と呼ぶ。

```elm
-- ❌ 避けるべき: コンポーネントレベルの Nested TEA
module Component.Sidebar exposing (Model, Msg, init, update, view)

type alias Model = { ... }
type Msg = Toggle | ...

update : Msg -> Model -> ( Model, Cmd Msg )
view : Model -> Html Msg
```

問題点:

| 問題 | 説明 |
|------|------|
| Msg ラッピング | 親が `SidebarMsg Sidebar.Msg` でラップし、`Cmd.map` が必要 |
| 状態の分散 | 各コンポーネントに状態が分散し、全体の見通しが悪くなる |
| コンポーネント間通信 | 兄弟コンポーネント同士の通信が非常に面倒 |
| オブジェクト指向への回帰 | コンポーネント = ローカルステート + メソッド = オブジェクト |

### 推奨するもの: ヘルパー関数

Elm が推奨するのは、**ただの関数**としてコードを分離すること。

```elm
-- ✅ Elm 推奨: ただの view 関数
viewSidebar : List Item -> Bool -> Html Msg
viewSidebar items isOpen = ...
```

`viewSidebar` を作ったからといって、`Sidebar.Model` や `Sidebar.update` が必要になるわけではない。

### The Life of a File の教え

Evan Czaplicki の [The Life of a File](https://www.youtube.com/watch?v=XpDsk374LDE) での主張:

1. **ファイルが大きくなるまで分割しない** — 早すぎる抽象化を避ける
2. **痛みを感じてから分割する** — 理論的な「正しさ」ではなく、実際の不便さを基準にする
3. **モジュールはドメイン概念で分ける** — UI の見た目（サイドバー、ヘッダー）ではなく、データやドメインの概念で分ける

## ページレベル TEA との区別

Elm における Nested TEA には2つのレベルがある:

| レベル | 例 | Elm の評価 |
|--------|-----|----------|
| ページレベル | Main → Page.Workflow.New | ✅ 推奨（SPA に必須） |
| コンポーネントレベル | Page.Workflow.New → Component.Sidebar | ⚠️ 慎重に |

ページレベルの Nested TEA は Elm SPA の標準パターン。問題になるのはページ**内部**をさらにコンポーネントに分割すること。

→ ページレベル TEA の詳細: [Nested TEA](NestedTEA.md)

## プロジェクトでのアプローチ

### Config Record パターン

このプロジェクトの `Component/` は、コンポーネントレベルの Nested TEA を**使わない**。代わりに config record パターンを採用している。

```elm
-- プロジェクトの実際のパターン（ApproverSelector）
view :
    { state : State
    , users : RemoteData ApiError (List UserItem)
    , onSearch : String -> msg   -- 親の Msg を直接受け取る
    , onSelect : UserItem -> msg
    , ...
    }
    -> Html msg
```

Nested TEA との比較:

| 観点 | Nested TEA（❌） | Config Record（✅） |
|------|-----------------|-------------------|
| 独自の `Msg` 型 | あり | なし — 親のコールバックを受け取る |
| 独自の `update` | あり | なし — 親が直接管理 |
| `Cmd.map` | 必要 | 不要 |
| State の所有者 | コンポーネント自身 | 親ページ |
| コンポーネント間通信 | 困難（親経由） | 容易（同じ update 内） |

### State + init + 純粋関数のパターン

ApproverSelector は config record パターンに加えて、`State` type alias と `init`、`handleKeyDown` 純粋関数を持つ。

```elm
type alias State =
    { selection : ApproverSelection
    , search : String
    , dropdownOpen : Bool
    , highlightIndex : Int
    }

init : State

handleKeyDown :
    { key : String, candidates : List UserItem, highlightIndex : Int }
    -> KeyResult
```

これは「ただの view ヘルパー関数」よりは一歩踏み込んでいるが、Nested TEA の問題は発生しない:

- `State` はただの type alias — 親が直接フィールドにアクセスできる
- `handleKeyDown` は純粋関数 — `( Model, Cmd Msg )` ではなく `KeyResult` を返す
- 副作用（`markDirty`、`validationErrors` 更新）は親ページに残る

### スペクトラム上の位置

Elm のコード分離はスペクトラムであり、プロジェクトのコンポーネントは安全な範囲に位置する。

```
ヘルパー関数          Config Record         State + 純粋関数       Nested TEA
(view 関数のみ)        (view + callbacks)    (+ State, init)       (Model/Msg/update)
     ←────────────────────────────────────────────────────────→
     Button              ConfirmDialog        ApproverSelector      ❌ 使わない
     LoadingSpinner      MessageAlert
     Badge
```

### 設計判断の根拠

ApproverSelector を State + 純粋関数パターンにした理由:

1. **New.elm が 1115行** — Evan も「ファイルが大きくなったら分割する」と言っている
2. **承認者選択は明確な責務の塊** — 4つの状態フィールド + キーボードロジック + 7つの view 関数が密結合
3. **再利用の可能性** — 承認者選択は他のページ（編集画面等）でも使いうる
4. **Nested TEA の問題が発生しない** — 親がすべてを制御し続ける

→ ADR-043 で Nested TEA（選択肢 B）は明確に却下: [ADR-043](../../70_ADR/043_500行超ファイルの分割戦略.md)

## ガイドライン

### コンポーネント化を検討するタイミング

1. ファイルが大きくなり、特定の責務が明確に区別できるとき
2. 関連する複数の view 関数 + 状態フィールドがセットで存在するとき
3. 実際に痛み（可読性の低下、変更の困難さ）を感じたとき

### コンポーネント化で守るべきルール

1. **Nested TEA にしない** — 独自の `Msg` 型と `update` 関数を持たせない
2. **Config Record パターンを使う** — 親のコールバックを受け取る
3. **State は type alias** — opaque type にしない。親がフィールドに直接アクセスできる
4. **ロジックは純粋関数** — `( Model, Cmd Msg )` ではなく、結果型（`KeyResult` 等）を返す
5. **副作用は親に残す** — dirty 状態管理、バリデーション等のページ固有ロジック

## 関連リソース

- [Elm Guide: Structure](https://guide.elm-lang.org/webapps/structure.html) — 公式ガイドのコード構造解説
- [The Life of a File](https://www.youtube.com/watch?v=XpDsk374LDE) — Evan Czaplicki, Elm Europe 2017
- [Elm Radio: The Life of a File](https://elm-radio.com/episode/life-of-a-file/) — ポッドキャスト解説
- [Nested TEA](NestedTEA.md) — ページレベル TEA の解説（プロジェクトナレッジ）
- [ADR-043: 500行超ファイルの分割戦略](../../70_ADR/043_500行超ファイルの分割戦略.md) — Component 抽出の判断根拠

## プロジェクトでの使用箇所

| コンポーネント | パターン | 特徴 |
|--------------|---------|------|
| `Component/Button.elm` | ヘルパー関数 | 引数を受けて Html を返すだけ |
| `Component/LoadingSpinner.elm` | ヘルパー関数 | 状態なし |
| `Component/Badge.elm` | ヘルパー関数 | 状態なし |
| `Component/MessageAlert.elm` | Config Record | コールバックあり |
| `Component/ConfirmDialog.elm` | Config Record | コールバック + 表示状態 |
| `Component/ApproverSelector.elm` | State + 純粋関数 | State, init, handleKeyDown, view |

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-10 | 初版作成（ApproverSelector 抽出を契機に） |
