# 別 Issue 作業時の lefthook 自動付与

**発生日時**: 2026-01-29
**重大度**: 中

## 問題

### 事象

`feature/36-workflow-approval` ブランチで Issue #157（楽観的ロック TOCTOU 修正）の実装を行い、コミットした。lefthook の `prepare-commit-msg` フックがブランチ名から `#36` を自動抽出し、コミットメッセージに `#36` を付与した。結果、Issue #157 の変更に `#36` のプレフィックスが付いた。

```
# 実際のコミットメッセージ（誤り）
#36 楽観的ロックの TOCTOU 問題を修正: save を insert + update_with_version_check に分離

# 期待するコミットメッセージ
#157 楽観的ロックの TOCTOU 問題を修正: save を insert + update_with_version_check に分離
```

### 原因分析

1. lefthook の `prepare-commit-msg` フックはブランチ名（`feature/36-xxx`）から Issue 番号を機械的に抽出する
2. AI エージェントは「このブランチで作業する = このブランチの Issue 番号が使われる」という仕組みを認識していなかった
3. 別 Issue の作業を同一ブランチで行う場合のワークフローが定義されていなかった

### 根本原因

「1ブランチ = 1 Issue」の前提が暗黙的であり、別 Issue の修正を同一ブランチで行う場合の手順が明文化されていなかった。

## 対策

### 即時対策（本セッション）

1. 新しいブランチ `fix/157-optimistic-lock-toctou` を `feature/36` の HEAD~1 から作成
2. 該当コミットを cherry-pick（`--no-commit`）し、正しい `#157` プレフィックスで再コミット
3. `feature/36` から誤ったコミットを除去（`git reset --hard HEAD~1` + force push）
4. PR #158 を `fix/157-optimistic-lock-toctou` → `feature/36-workflow-approval` で作成

### 恒久対策

別 Issue の作業を行う場合は、必ず専用ブランチを作成してからコミットする:

1. 現在のブランチが対象 Issue と異なる場合は、新しいブランチを切る
2. lefthook が正しい Issue 番号を付与できるブランチ名にする（例: `fix/157-xxx`）
3. 元のブランチに対して PR を作成し、マージする

## AI エージェントへの教訓

### 「1ブランチ = 1 Issue」の原則

lefthook がブランチ名から Issue 番号を自動付与する環境では、コミット前に以下を確認する:

1. 現在のブランチ名に含まれる Issue 番号は何か
2. これから行う変更は、そのブランチの Issue に対応するものか
3. 異なる Issue の変更であれば、専用ブランチを作成してから作業する

### コミット前チェックリスト

- [ ] 変更内容が現在のブランチの Issue に対応しているか確認
- [ ] 別 Issue の場合は専用ブランチを作成したか確認
- [ ] lefthook が付与する Issue 番号が正しいか確認

## 関連

- Issue: [#157](https://github.com/ka2kama/ringiflow/issues/157)（楽観的ロック TOCTOU 修正）
- PR: [#158](https://github.com/ka2kama/ringiflow/pull/158)
- lefthook 設定: `lefthook.yaml`（prepare-commit-msg フック）

## 分類

- カテゴリ: 知識-実行乖離
- 失敗タイプ: プロセスギャップ

## 検証（対策実行後に追記）

- 実施日: 2026-02-11
- 対策の実行状況: 実行済み
- 効果: cherry-pick + 再コミットで Issue 番号の紐付けを修正（#157、PR #158）。以降、別 Issue の作業は専用ブランチを作成するフローが定着している。lefthook 自動付与の誤りは再発していない
- 備考: なし
