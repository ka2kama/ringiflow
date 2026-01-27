# 関連ファイルを芋づる式に見つける

## いつ使うか

- ある型・関数がどこで使われているか把握したいとき
- 機能の影響範囲を調べたいとき
- リファクタリング前に依存関係を確認したいとき

## 手順

### Claude Code でやる場合

Task ツール（Explore エージェント）に依頼する:

```
「UserRepository がどこで使われているか調べて」
「WorkflowService に依存しているファイルを一覧して」
```

Explore エージェントは Glob → Grep → Read を組み合わせて探索する。

### 手動でやる場合

#### 1. まずファイル一覧を取得

```bash
# Rust の場合
rg "UserRepository" --type rust -l

# Elm の場合
rg "UserRepository" --type elm -l
```

`-l` オプションでファイル名のみ表示。件数を確認してから次へ進む。

#### 2. コンテキスト付きで確認

```bash
# 前後3行を表示
rg "UserRepository" --type rust -C 3

# 特定ディレクトリに絞る
rg "UserRepository" --type rust -C 3 backend/apps/
```

#### 3. 依存の連鎖を追う

見つかったファイルで使われている型・関数を、さらに検索:

```bash
# UserRepository を使っている UserService を発見
#   → UserService を使っている箇所を探す
rg "UserService" --type rust -l
```

これを繰り返すと、影響範囲の全体像が見える。

#### 4. 定義元を確認

```bash
# struct/impl の定義を探す
rg "struct UserRepository|impl UserRepository" --type rust
```

## なぜこの方法か

### 段階的に絞り込む理由

- 全文検索は結果が多すぎると読めない
- まず `-l` で件数を確認し、対処可能か判断
- 件数が多ければディレクトリで絞る

### ripgrep (`rg`) を使う理由

- `grep -r` より高速（特に大規模リポジトリ）
- `.gitignore` を自動で尊重
- `--type` でファイル種別を簡単に指定

### IDE の検索との違い

IDE の「Find Usages」も有効だが:
- 正規表現で柔軟な検索ができる（部分一致、パターン）
- 複数リポジトリをまたいで検索できる
- スクリプトに組み込める
