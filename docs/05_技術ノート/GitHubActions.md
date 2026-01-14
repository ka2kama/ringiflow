# GitHub Actions

## 概要

GitHub Actions は GitHub が提供する CI/CD プラットフォーム。
リポジトリ内のイベント（push、pull_request など）をトリガーにワークフローを実行する。

## アクション許可設定

### 許可モード

Settings → Actions → General → Actions permissions で設定。

| モード | 説明 |
|--------|------|
| Allow all actions | すべてのアクションを許可 |
| Allow local actions only | 同一リポジトリ内のアクションのみ |
| Allow select actions | 許可リストに基づいて制限 |

### 許可パターンの設定

「Allow select actions」を選択した場合、許可するアクションをパターンで指定する。

```
actions/*                    # GitHub 公式アクション
owner/repo@*                 # 特定リポジトリの全バージョン
owner/repo@v1                # 特定バージョンのみ
```

### 間接的な依存に注意

一部のアクションは内部で別のアクションを呼び出す。
その場合、間接的に使用されるアクションも許可リストに追加する必要がある。

**例: `extractions/setup-just@v3`**

`setup-just@v3` は内部で `extractions/setup-crate@v1` を使用する。

```
# エラーメッセージ
The action extractions/setup-crate@v1 is not allowed in owner/repo because all actions must be from a repository owned by ...
```

**対処法:**

許可パターンに両方を追加:

```
extractions/setup-just@*
extractions/setup-crate@*   # ← 間接依存も追加
```

### 許可設定のデバッグ

CI が「action not allowed」エラーで失敗した場合:

1. エラーメッセージから不許可のアクション名を特定
2. Settings → Actions → General で許可パターンを追加
3. CI を再実行

## プロジェクトでの許可設定

| パターン | 用途 |
|----------|------|
| `actions/*` | GitHub 公式アクション |
| `dorny/paths-filter@*` | 変更検出 |
| `dtolnay/rust-toolchain@*` | Rust ツールチェーン |
| `extractions/setup-just@*` | just コマンドランナー |
| `extractions/setup-crate@*` | setup-just の間接依存 |
| `pnpm/action-setup@*` | pnpm パッケージマネージャ |

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-15 | 初版作成（アクション許可設定） |
