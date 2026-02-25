# review-and-merge で rebase 確認が遅い

## 事象

PR #349 のマージ時に `gh pr merge` が「base branch と同期が取れていない」で失敗した。Claude Auto Review の完了を待ってからマージを試みたが、その時点で初めて rebase が必要だと判明し、rebase + push → CI + Review のやり直しが発生した。

## 原因分析

`/review-and-merge` スキルの Step 1（PR 状態確認）で base branch との同期状態を確認していなかった。Review 完了を待ってからマージを試みる順序のため、rebase が必要な場合に Review の待ち時間が完全に無駄になる。

## 対策

`/review-and-merge` の Step 1 に **base branch との同期確認** を追加する。差分があれば、Review 完了を待つ前に rebase + push を行い、CI + Review が1回で済むようにする。

## 次のアクション

- [x] `/review-and-merge` スキルの Step 1 に同期確認を追加（このセッションで実施）

## 分類

- カテゴリ: 視点不足
- 失敗タイプ: プロセスギャップ
- 問題の性質: プロセス的

## 検証（対策実行後に追記）

- 実施日: 2026-02-11
- 対策の実行状況: 実行済み
- 効果: `/review-and-merge` スキルの Step 1 に base branch との同期確認が追加された（#349）。CLAUDE.md の PR 完了フローにも Step 6「base branch 同期確認」として組み込まれている。rebase の遅延による Review やり直しは再発していない
- 備考: なし
