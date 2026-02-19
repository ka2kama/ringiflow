# RingiFlow Web フロントエンド

Elm + Vite で構築されたフロントエンドアプリケーション。

## ディレクトリ構造

```
apps/web/
├── src/
│   ├── Main.elm       # アプリケーションエントリポイント（TEA）
│   ├── Route.elm      # URL ルーティング
│   ├── Ports.elm      # JavaScript 連携
│   ├── main.js        # JavaScript エントリポイント
│   └── Page/          # ページコンポーネント（将来）
├── tests/
│   └── Example.elm    # テストファイル
├── elm.json           # Elm パッケージ設定
├── package.json       # Node.js パッケージ設定
├── vite.config.js     # Vite ビルド設定
├── index.html         # HTML テンプレート
├── .env.example       # 環境変数テンプレート
└── .gitignore         # Git 除外設定
```

## コマンド

```bash
# 依存関係インストール
pnpm install

# 開発サーバー起動（HMR 対応）
pnpm run dev

# 本番ビルド
pnpm run build

# テスト実行
pnpm run test

# フォーマット
pnpm run format

# フォーマットチェック
pnpm run format:check
```

---

## 設定ファイル解説

### elm.json

Elm パッケージマネージャの設定ファイル。
`elm install` コマンドで依存関係を追加すると自動更新される。

```json
{
    "type": "application",
    "source-directories": ["src"],
    "elm-version": "0.19.1",
    "dependencies": {
        "direct": { ... },
        "indirect": { ... }
    },
    "test-dependencies": {
        "direct": { ... },
        "indirect": { ... }
    }
}
```

#### フィールド説明

| フィールド | 説明 |
|-----------|------|
| `type` | `application`（実行可能）または `package`（ライブラリ） |
| `source-directories` | Elm ソースコードのパス |
| `elm-version` | 使用する Elm コンパイラのバージョン |
| `dependencies.direct` | 直接依存するパッケージ |
| `dependencies.indirect` | 間接依存（direct の依存先） |
| `test-dependencies` | テスト専用の依存関係 |

#### 依存パッケージの役割

**direct（直接依存）**

| パッケージ | バージョン | 用途 |
|-----------|----------|------|
| `elm/browser` | 1.0.2 | SPA 構築（Browser.application） |
| `elm/core` | 1.0.5 | 基本型・関数（List, String, Maybe 等） |
| `elm/html` | 1.0.0 | HTML 生成（Virtual DOM） |
| `elm/http` | 2.0.0 | HTTP リクエスト |
| `elm/json` | 1.1.3 | JSON エンコード/デコード |
| `elm/url` | 1.0.0 | URL パース・構築 |

**indirect（間接依存）**

| パッケージ | 用途 |
|-----------|------|
| `elm/bytes` | バイナリデータ処理（http が使用） |
| `elm/file` | ファイル操作（http が使用） |
| `elm/time` | 時刻操作（http が使用） |
| `elm/virtual-dom` | 仮想 DOM 実装（html が使用） |

**test-dependencies**

| パッケージ | 用途 |
|-----------|------|
| `elm-explorations/test` | テストフレームワーク |
| `elm/random` | ランダム値生成（test が使用） |

#### Elm パッケージの特徴

1. **セマンティックバージョニング強制**:
   Elm コンパイラが API の変更を検出し、
   破壊的変更があれば自動的にメジャーバージョンを上げる

2. **依存関係の自動解決**:
   `elm install` が依存関係を解決し、
   互換性のあるバージョンを選択

3. **純粋関数のみ**:
   Elm パッケージは副作用を持てないため、
   セキュリティリスクが低い

---

### package.json

Node.js プロジェクト設定。ビルドツール（Vite）の依存関係を管理。

```json
{
  "name": "ringiflow-web",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": { ... },
  "devDependencies": { ... },
}
```

#### フィールド説明

| フィールド | 値 | 説明 |
|-----------|---|------|
| `name` | `ringiflow-web` | パッケージ名（npm publish しないため任意） |
| `version` | `0.1.0` | バージョン（セマンティックバージョニング） |
| `private` | `true` | npm への公開を防止 |
| `type` | `module` | ES モジュールとして扱う（import/export 構文） |

#### scripts

| スクリプト | コマンド | 説明 |
|-----------|---------|------|
| `dev` | `vite` | 開発サーバー起動（HMR 対応） |
| `build` | `vite build` | 本番用ビルド |
| `preview` | `vite preview` | ビルド結果のプレビュー |
| `test` | `elm-test` | Elm テスト実行 |
| `format` | `elm-format --yes src/` | コードフォーマット |
| `format:check` | `elm-format --validate src/` | フォーマットチェック |

#### devDependencies

| パッケージ | バージョン | 説明 |
|-----------|----------|------|
| `vite` | `^6.0.7` | ビルドツール |
| `vite-plugin-elm` | `^3.0.1` | Elm ファイルのコンパイル |

**注意**: `elm`, `elm-format`, `elm-test` は含まれていない。
これらはグローバルインストールする。
理由は [ADR-002](../../docs/05_ADR/002_フロントエンドツールチェーンの選定.md) を参照。

#### .mise.toml（プロジェクトルート）

```toml
[tools]
node = "22"
elm = "0.19.1"
"npm:elm-format" = "0.8.8"
```

[mise](https://mise.jdx.dev/) でプロジェクト固有のツールバージョンを固定。
mise がインストールされていれば、このディレクトリで
自動的に指定バージョンが使用される。
経緯は [ADR-052](../../docs/05_ADR/052_Node.jsバージョン管理のmiseへの移行.md) を参照。

---

### index.html

Vite のエントリポイントとなる HTML ファイル。

```html
<!DOCTYPE html>
<html lang="ja">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>RingiFlow</title>
    <style>
      /* インラインスタイル */
    </style>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>
```

#### 設計ポイント

1. **`<div id="app">`**:
   Elm アプリケーションのマウントポイント。
   Elm はこの要素の中身を完全に制御する。

2. **`type="module"`**:
   ES モジュールとしてスクリプトを読み込む。
   Vite は開発時にこれを利用してバンドル不要で動作。

3. **インラインスタイル**:
   Phase 0 では最小限のスタイリングのみ。
   CSS フレームワークは Phase 1 以降で検討。

4. **`lang="ja"`**:
   日本語コンテンツを明示。
   スクリーンリーダーやブラウザの言語検出に使用。

---

### .env.example

環境変数のテンプレート。実際の `.env` ファイルは `.gitignore` に含める。

```bash
# API ベース URL（開発時は空で OK、プロキシを使用）
VITE_API_BASE_URL=
```

#### 使用方法

```bash
# テンプレートをコピー
cp .env.example .env

# 必要に応じて値を設定
echo "VITE_API_BASE_URL=https://api.example.com" >> .env
```

#### Vite の環境変数

- `VITE_` プレフィックスがある変数のみクライアントに公開
- `import.meta.env.VITE_*` でアクセス
- 機密情報（API キー等）は `VITE_` プレフィックスを**付けない**

---

### .gitignore

Git でバージョン管理しないファイルを指定。

```
# 依存関係
node_modules/     # pnpm install で生成
elm-stuff/        # Elm コンパイル時に生成

# ビルド出力
dist/             # pnpm run build で生成

# 環境変数
.env              # 機密情報を含む可能性
.env.local        # ローカル固有の設定

# エディタ
.vscode/          # VS Code 設定
.idea/            # IntelliJ 設定

# OS
.DS_Store         # macOS
Thumbs.db         # Windows
```

#### 含めないもの

- **pnpm-lock.yaml**: バージョン管理に**含める**（再現可能なビルド）
- **elm.json**: バージョン管理に**含める**（依存関係の明示）

---

## アーキテクチャ

### The Elm Architecture (TEA)

```
┌─────────────────────────────────────────────────────────────┐
│                        Elm Runtime                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌─────────┐    Msg     ┌────────┐    Model    ┌──────┐   │
│   │  View   │ ─────────→ │ Update │ ──────────→ │ View │   │
│   └─────────┘            └────────┘             └──────┘   │
│       ↑                      │                      │      │
│       │                      ↓                      │      │
│       │                 ┌────────┐                  │      │
│       └──── Model ───── │ Model  │ ←── Model ───────┘      │
│                         └────────┘                         │
│                              │                             │
│                              ↓ Cmd                         │
│                         ┌────────┐                         │
│                         │ Effect │ (HTTP, Ports, etc.)     │
│                         └────────┘                         │
│                              │                             │
│                              ↓ Msg                         │
│                         ┌────────┐                         │
│                         │ Update │                         │
│                         └────────┘                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

1. **View**: Model から HTML を生成（純粋関数）
2. **Update**: Msg を受け取り Model を更新（純粋関数）
3. **Model**: アプリケーション状態（不変データ）
4. **Cmd**: 副作用（HTTP リクエスト、Ports 等）
5. **Msg**: イベント（ユーザー操作、外部からの通知）

### データフロー

```
ユーザー操作
     │
     ↓
  HTML イベント
     │
     ↓
    Msg
     │
     ↓
  update 関数
     │
     ├──→ 新しい Model
     │         │
     │         ↓
     │      view 関数
     │         │
     │         ↓
     │    新しい HTML
     │
     └──→ Cmd（副作用）
              │
              ↓
         外部システム
              │
              ↓
            Msg
              │
              ↓
         update 関数
              │
              ...（繰り返し）
```

---

## 関連ドキュメント

- [開発環境構築手順](../../docs/04_手順書/01_開発参画/01_開発環境構築.md)
- [ADR-002: フロントエンドツールチェーンの選定](../../docs/05_ADR/002_フロントエンドツールチェーンの選定.md)
