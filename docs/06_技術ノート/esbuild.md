# esbuild（イーエスビルド）

## 概要

esbuild は Go 言語で書かれた高速な JavaScript/TypeScript バンドラー・トランスパイラー。
従来のツール（Webpack, Babel）より 10〜100 倍高速。

## 特徴

| 項目 | 説明 |
|------|------|
| 言語 | Go（ネイティブバイナリ） |
| 速度 | 従来ツールの 10〜100 倍 |
| 機能 | バンドル、ミニファイ、トランスパイル |
| 対応形式 | JavaScript, TypeScript, JSX, CSS |

## なぜ速いか

1. **Go で実装**: ネイティブコードにコンパイルされ、JavaScript より高速
2. **並列処理**: マルチコア CPU を最大限活用
3. **メモリ効率**: データを何度もパースし直さない
4. **ゼロキャッシュ**: キャッシュなしでも十分高速

## Vite との関係

Vite は esbuild を**開発時のトランスパイル**に使用している。

```
開発時 (dev server)
┌─────────────────────────────────────┐
│ ブラウザからリクエスト              │
│         ↓                           │
│ Vite が esbuild でトランスパイル    │  ← 高速
│         ↓                           │
│ ESM としてブラウザに返却            │
└─────────────────────────────────────┘

本番ビルド
┌─────────────────────────────────────┐
│ Rollup でバンドル                   │  ← 最適化重視
│         ↓                           │
│ esbuild でミニファイ                │  ← 高速
└─────────────────────────────────────┘
```

Vite が esbuild を使う理由:
- TypeScript → JavaScript の変換が爆速
- 開発サーバーの起動・HMR（Hot Module Replacement）が高速

## 本プロジェクトでの使用

本プロジェクトでは Vite の内部依存として esbuild が使用されている。

```bash
$ pnpm why esbuild

vite 7.3.1
└── esbuild 0.27.2
```

直接 `package.json` に記載しているわけではなく、Vite が依存している。

## インストール方法

esbuild はネイティブバイナリのため、OS/アーキテクチャごとに異なるバイナリが必要。

### npm/pnpm でのインストール

```bash
pnpm add -D esbuild
```

インストール時に `postinstall` スクリプトで適切なバイナリがダウンロードされる。

### オプショナル依存関係

esbuild は `optionalDependencies` としてプラットフォーム固有のパッケージを持つ：

```json
{
  "optionalDependencies": {
    "@esbuild/linux-x64": "0.27.2",
    "@esbuild/darwin-arm64": "0.27.2",
    "@esbuild/win32-x64": "0.27.2"
  }
}
```

pnpm は環境に応じて適切なパッケージのみをインストールする。

## CLI の基本的な使い方

```bash
# バンドル
esbuild src/index.ts --bundle --outfile=dist/bundle.js

# ミニファイ
esbuild src/index.ts --bundle --minify --outfile=dist/bundle.min.js

# ソースマップ付き
esbuild src/index.ts --bundle --sourcemap --outfile=dist/bundle.js
```

## API としての使用

```javascript
import * as esbuild from 'esbuild'

await esbuild.build({
  entryPoints: ['src/index.ts'],
  bundle: true,
  minify: true,
  outfile: 'dist/bundle.js',
})
```

## 制限事項

esbuild は速度を優先しているため、一部機能に制限がある：

| 項目 | 状況 |
|------|------|
| TypeScript 型チェック | 行わない（トランスパイルのみ） |
| 一部の Babel プラグイン | 非互換 |
| CSS Modules | 基本対応（一部制限あり） |

型チェックが必要な場合は `tsc --noEmit` を別途実行する。

## 関連リソース

- [esbuild 公式サイト](https://esbuild.github.io/)
- [Vite - Why Vite](https://vite.dev/guide/why.html)
- [pnpm ビルドスクリプト制限](./pnpm_ビルドスクリプト制限.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-19 | 初版作成 |
