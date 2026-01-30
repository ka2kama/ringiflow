# Git 履歴の書き換え

## 概要

Git の履歴を書き換える方法をまとめる。コミットの統合、メッセージの変更、署名の追加などに使用する。

## git rebase -i（Interactive Rebase）

### 基本的な使い方

```bash
# 直近3コミットを対象
git rebase -i HEAD~3

# 最初のコミットから全て対象
git rebase -i --root
```

### --root オプション

通常の `HEAD~N` 指定では最初のコミット（ルートコミット）を対象にできない。`--root` を付けると最初のコミットから全て対象になる。

```
HEAD~3:  直近3件のみ（ルートコミットは対象外）
--root:  最初のコミットから全て対象
```

**使いどころ:**
- 全履歴のコミットを整理したいとき
- 最初のコミットのメッセージを変更したいとき
- 全コミットに署名を追加したいとき

### rebase-todo の操作

```
pick   - コミットをそのまま使用
reword - コミットを使用し、メッセージを編集
edit   - コミットを使用し、修正のために停止
squash - 前のコミットに統合（メッセージも統合）
fixup  - 前のコミットに統合（メッセージは破棄）
drop   - コミットを削除
```

### --exec オプション

各コミット適用後にコマンドを実行する。

```bash
# 全コミットに署名を追加
git rebase --root --exec 'git commit --amend --no-edit -S'
```

**処理の流れ:**

```
コミット1を適用 → exec コマンド実行
コミット2を適用 → exec コマンド実行
...
```

### GIT_SEQUENCE_EDITOR

rebase-todo を自動生成するためのエディタ指定。

```bash
# ファイルから rebase-todo をコピー
GIT_SEQUENCE_EDITOR="cp rebase-todo" git rebase -i --root
```

## git filter-branch

### --msg-filter

コミットメッセージを書き換える。

```bash
git filter-branch --msg-filter 'スクリプト' -- --all
```

**スクリプト例:**

```bash
#!/bin/bash
case "$GIT_COMMIT" in
  <ハッシュ>)
    echo "新しいメッセージ"
    ;;
  *)
    cat  # 変更しない場合は標準入力をそのまま出力
    ;;
esac
```

### 実行後のクリーンアップ

filter-branch はバックアップを作成する。不要なら削除:

```bash
rm -rf .git/refs/original
```

## コミット署名

### 署名の種類

| 種類 | 設定 |
|------|------|
| GPG | `git config --global gpg.format gpg` |
| SSH | `git config --global gpg.format ssh` |

### 署名の設定

```bash
# 署名キーの設定（SSH の場合）
git config --global user.signingkey "ssh-ed25519 AAAA..."

# 自動署名の有効化
git config --global commit.gpgsign true
```

### 署名の確認

```bash
git log --show-signature
```

### 注意点

- rebase や filter-branch で履歴を書き換えると署名は無効になる
- 署名し直すには SSH agent へのアクセスが必要

## 使い分け

| 目的 | 手段 |
|------|------|
| コミットの統合 | `rebase -i` + `fixup`/`squash` |
| メッセージ変更（少数） | `rebase -i` + `reword` |
| メッセージ変更（大量） | `filter-branch --msg-filter` |
| 署名の追加 | `rebase --exec 'git commit --amend -S'` |

## 関連リソース

- [Git - git-rebase Documentation](https://git-scm.com/docs/git-rebase)
- [Git - git-filter-branch Documentation](https://git-scm.com/docs/git-filter-branch)
- [Signing commits - GitHub Docs](https://docs.github.com/en/authentication/managing-commit-signature-verification/signing-commits)
