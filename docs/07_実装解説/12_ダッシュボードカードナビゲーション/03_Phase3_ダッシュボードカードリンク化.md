# Phase 3: ダッシュボードカードのリンク化

## 対応 Issue

[#267](https://github.com/ka2kama/ringiflow/issues/267)

## 概要

ダッシュボードの KPI カードを静的な `div` からクリック可能な `a` 要素に変更し、対応するフィルタ付き一覧ページにリンクする。

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`frontend/src/Page/Home.elm`](../../../../frontend/src/Page/Home.elm) | KPI カードのリンク化 |

## 実装内容

### カードの遷移先

| カード | 遷移先 | 理由 |
|--------|--------|------|
| 承認待ちタスク | `/tasks` | API が Active タスクのみ返すためフィルタ不要 |
| 申請中 | `/workflows?status=in_progress` | status フィルタで InProgress に絞り込み |
| 本日完了 | `/workflows?completed_today=true` | completedToday プリセットフィルタ |

### viewStatCardLink

```elm
viewStatCardLink :
    { label : String, value : Int
    , bgColorClass : String, textColorClass : String
    , route : Route.Route
    } -> Html Msg
viewStatCardLink config =
    a
        [ href (Route.toString config.route)
        , class ("block rounded-xl p-6 text-center no-underline transition-shadow hover:shadow-md " ++ config.bgColorClass)
        ]
        [ div [ class ("text-3xl font-bold " ++ config.textColorClass) ]
            [ text (String.fromInt config.value) ]
        , div [ class "mt-2 text-sm text-secondary-500" ]
            [ text config.label ]
        ]
```

## 設計解説

### 1. `div` → `a` 要素への変更

場所: [`Home.elm`](../../../../frontend/src/Page/Home.elm) の `viewStatCardLink`

代替案: `div` + `onClick` + `Nav.pushUrl`

`<a>` 要素を使う理由:
- ブラウザの標準的なリンク動作（右クリック「新しいタブで開く」、Ctrl+クリック）が利用可能
- SEO やアクセシビリティに適合
- Elm SPA のリンク処理（`Browser.application` の `onUrlRequest`）と自然に統合される
- `href` 属性があるためクローラーやスクリーンリーダーが遷移先を把握できる

### 2. レコード型による設定パラメータ

場所: [`Home.elm`](../../../../frontend/src/Page/Home.elm) の `viewStatCardLink` の引数

5 つのパラメータを個別引数にするとシグネチャが長くなり、呼び出し側で引数の順序を間違えるリスクがある。レコード型にすることで名前付き引数として機能し、可読性と安全性が向上する。

Elm ではレコード型にラベルを付けることで、Haskell の Named Arguments パターンと同等の効果を得られる。
