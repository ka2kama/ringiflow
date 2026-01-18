# pnpm コマンド

## 概要

pnpm（ピーエヌピーエム）は高速でディスク効率の良いパッケージマネージャ。
npm や yarn と互換性を持ちつつ、独自の機能も備える。

## 基本コマンド

| コマンド | 説明 |
|---------|------|
| `pnpm install` | 依存関係をインストール |
| `pnpm add <pkg>` | パッケージを追加 |
| `pnpm add -D <pkg>` | devDependencies に追加 |
| `pnpm remove <pkg>` | パッケージを削除 |
| `pnpm update` | パッケージを更新 |
| `pnpm run <script>` | スクリプトを実行 |

## 調査・デバッグ用コマンド

### pnpm why

**なぜそのパッケージがインストールされているか**を表示する。

```bash
$ pnpm why esbuild

vite 7.3.1
└── esbuild 0.27.2
```

用途:
- 知らないパッケージがどこから来たか確認
- 重複バージョンの原因調査
- パッケージ削除時の影響確認

### pnpm list

インストール済みパッケージの一覧を表示。

```bash
# 直接の依存関係のみ
pnpm list

# 全ての依存関係（ツリー表示）
pnpm list --depth=Infinity

# 特定パッケージの情報
pnpm list <pkg>
```

### pnpm outdated

更新可能なパッケージを表示。

```bash
$ pnpm outdated

Package  Current  Wanted  Latest
vite     7.3.1    7.3.2   7.3.2
```

## ワークスペース関連

モノレポでの複数パッケージ管理に使用。

| コマンド | 説明 |
|---------|------|
| `pnpm -r <cmd>` | 全パッケージで再帰実行 |
| `pnpm --filter <pkg> <cmd>` | 特定パッケージで実行 |
| `pnpm -w add <pkg>` | ルートに追加 |

```bash
# 全パッケージでテスト実行
pnpm -r test

# frontend パッケージのみでビルド
pnpm --filter frontend build
```

## セキュリティ関連

### pnpm approve-builds

依存パッケージのビルドスクリプト実行を許可/無視する。

```bash
$ pnpm approve-builds

? esbuild@0.27.2 has a build script. Allow it?
> allow
  ignore
  deny
```

詳細: [pnpm ビルドスクリプト制限](./pnpm_ビルドスクリプト制限.md)

### pnpm audit

セキュリティ脆弱性をチェック。

```bash
$ pnpm audit

┌─────────────────────┬──────────────────────────────────────────────┐
│ moderate            │ Prototype Pollution in lodash                │
├─────────────────────┼──────────────────────────────────────────────┤
│ Package             │ lodash                                       │
│ Vulnerable versions │ <4.17.21                                     │
│ Fix available       │ Yes                                          │
└─────────────────────┴──────────────────────────────────────────────┘
```

## npm / yarn との対応表

| 操作 | pnpm | npm | yarn |
|------|------|-----|------|
| インストール | `pnpm install` | `npm install` | `yarn` |
| パッケージ追加 | `pnpm add` | `npm install` | `yarn add` |
| 削除 | `pnpm remove` | `npm uninstall` | `yarn remove` |
| 依存元の調査 | `pnpm why` | `npm explain` | `yarn why` |
| スクリプト実行 | `pnpm run` | `npm run` | `yarn run` |
| 更新確認 | `pnpm outdated` | `npm outdated` | `yarn outdated` |

## 関連リソース

- [pnpm 公式ドキュメント](https://pnpm.io/)
- [pnpm ビルドスクリプト制限](./pnpm_ビルドスクリプト制限.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-19 | 初版作成 |
