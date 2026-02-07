# prompts/plans/

Claude Code の plan mode で作成された計画ファイルを保存する。

## 概要

計画ファイルには、設計段階での思考過程が記録されている:

- 設計判断とその理由
- ブラッシュアップループの記録（設計の反復改善過程）
- 収束確認チェックリスト

セッションログ（`runs/`）が「何をしたか」を記録するのに対し、計画ファイルは「どう考えが変遷したか」を記録する。

## ファイル命名

Claude Code が自動生成するランダム名（例: `clever-napping-panda.md`）をそのまま使用する。ファイルの内容は1行目のタイトル（例: `# Plan: #288 ...`）で識別する。

## 索引

| 日付 | ファイル | タイトル |
|------|---------|---------|
| 2026-01-25 | [optimized-giggling-minsky.md](optimized-giggling-minsky.md) | Issue #98: E2E API テストを hurl で追加する |
| 2026-01-25 | [snuggly-wishing-popcorn.md](snuggly-wishing-popcorn.md) | CI/justfile シェルスクリプトチェック強化 |
| 2026-01-27 | [hashed-wishing-blum.md](hashed-wishing-blum.md) | Issue #115: フロントエンド ワークフロー申請フォーム |
| 2026-01-28 | [purring-weaving-owl.md](purring-weaving-owl.md) | mprocs 導入: 開発サーバー一括起動 (dev-all) |
| 2026-01-29 | [adaptive-herding-wadler.md](adaptive-herding-wadler.md) | Issue #157: 楽観的ロックの TOCTOU 問題を修正する |
| 2026-01-29 | [cosmic-seeking-floyd.md](cosmic-seeking-floyd.md) | ワークフロー詳細 API にステップデータを含める修正 |
| 2026-01-30 | [expressive-roaming-puffin.md](expressive-roaming-puffin.md) | 技術ノート・学習ノート統合計画 |
| 2026-01-30 | [nifty-strolling-rose.md](nifty-strolling-rose.md) | 開発ワークフローへの段階的英語統合 |
| 2026-01-30 | [ticklish-tinkering-sparkle.md](ticklish-tinkering-sparkle.md) | Issue #37: タスク一覧・詳細画面 |
| 2026-01-30 | [velvet-tickling-meadow.md](velvet-tickling-meadow.md) | Issue #38: ダッシュボード実装計画 |
| 2026-01-31 | [abstract-twirling-peacock.md](abstract-twirling-peacock.md) | Issue #196: 申請詳細画面のユーザーID表示修正 |
| 2026-01-31 | [bright-sniffing-diffie.md](bright-sniffing-diffie.md) | Issue #146: API パス設計の統一 |
| 2026-01-31 | [compressed-zooming-papert.md](compressed-zooming-papert.md) | Issue #178: 承認セクションにコメント入力欄を追加 |
| 2026-01-31 | [eventual-twirling-nebula.md](eventual-twirling-nebula.md) | Issue #176: 破壊的操作の確認ダイアログ追加 |
| 2026-01-31 | [floofy-herding-teacup.md](floofy-herding-teacup.md) | #174: Tailwind CSS 導入とアプリシェルレイアウト |
| 2026-01-31 | [fuzzy-roaming-tide.md](fuzzy-roaming-tide.md) | Issue #182: 共有UIコンポーネントの抽出 |
| 2026-01-31 | [sleepy-watching-fountain.md](sleepy-watching-fountain.md) | MCP PostgreSQL サーバー導入 + justfile ヘルパー |
| 2026-01-31 | [staged-exploring-wozniak.md](staged-exploring-wozniak.md) | #180: 共有 RemoteData モジュールの抽出 |
| 2026-01-31 | [streamed-enchanting-hennessy.md](streamed-enchanting-hennessy.md) | #125: ApiResponse\<T\> 統一 |
| 2026-02-01 | [twinkly-yawning-iverson.md](twinkly-yawning-iverson.md) | Issue #198: 人間向け表示用 ID の導入 |
| 2026-02-02 | [quirky-crunching-newt.md](quirky-crunching-newt.md) | Issue #173: 共有 UI コンポーネント + アクセシビリティ |
| 2026-02-02 | [staged-churning-metcalfe.md](staged-churning-metcalfe.md) | 自己検証プロトコル（Self-Review Protocol）の導入 |
| 2026-02-04 | [distributed-cuddling-hedgehog.md](distributed-cuddling-hedgehog.md) | `/review-and-merge` スキル作成計画 |
| 2026-02-04 | [shimmying-leaping-rocket.md](shimmying-leaping-rocket.md) | #207: 表示用 ID API + フロントエンド（Phase A-3） |
| 2026-02-05 | [binary-riding-pearl.md](binary-riding-pearl.md) | #208: 表示用 ID WorkflowStep への導入（Phase B） |
| 2026-02-05 | [graceful-conjuring-candle.md](graceful-conjuring-candle.md) | Issue #229: URL パスパラメータに表示用番号を使用 |
| 2026-02-05 | [nifty-drifting-lerdorf.md](nifty-drifting-lerdorf.md) | `new` / `from_db` にパラメータ構造体を導入 |
| 2026-02-05 | [quizzical-zooming-crystal.md](quizzical-zooming-crystal.md) | Issue #222: ドメインモデルから非決定的な値の生成を排除 |
| 2026-02-06 | [abstract-orbiting-codd.md](abstract-orbiting-codd.md) | #181: ErrorResponse 統一計画 |
| 2026-02-06 | [agile-wondering-waterfall.md](agile-wondering-waterfall.md) | Issue 精査プロセスの導入 |
| 2026-02-06 | [crystalline-beaming-taco.md](crystalline-beaming-taco.md) | Issue #267: サマリーカードから一覧ページへの遷移 |
| 2026-02-06 | [elegant-dreaming-avalanche.md](elegant-dreaming-avalanche.md) | Lightsail OS を AlmaLinux 9.4 へ変更 |
| 2026-02-06 | [generic-moseying-island.md](generic-moseying-island.md) | #177: フォーム dirty-state 検出 |
| 2026-02-06 | [generic-swimming-manatee.md](generic-swimming-manatee.md) | Issue #203 Phase 3: ユーザー検索・承認者選択 UI |
| 2026-02-06 | [hidden-noodling-beaver.md](hidden-noodling-beaver.md) | #184: テストフィクスチャ集約 |
| 2026-02-06 | [moonlit-strolling-book.md](moonlit-strolling-book.md) | Issue #269: 品質チェックリストの重複整理 |
| 2026-02-06 | [velvety-seeking-river.md](velvety-seeking-river.md) | Issue #203: 承認者選択のユーザー検索 UI |
| 2026-02-07 | [agile-wobbling-mccarthy.md](agile-wobbling-mccarthy.md) | #266: README.md 更新計画 |
| 2026-02-07 | [clever-growing-corbato.md](clever-growing-corbato.md) | Issue #300: 設計書の実装状態マーカー導入 |
| 2026-02-07 | [clever-napping-panda.md](clever-napping-panda.md) | #288: DevAuth の本番ビルド除外 |
| 2026-02-07 | [drifting-shimmying-kernighan.md](drifting-shimmying-kernighan.md) | Issue #106: テスト設計方針の策定 |
| 2026-02-07 | [quirky-cooking-reef.md](quirky-cooking-reef.md) | Issue #271: 方法論にもベストプラクティス起点を適用 |
| 2026-02-07 | [replicated-weaving-flute.md](replicated-weaving-flute.md) | #265: ConfirmDialog アクセシビリティ改善 |
| 2026-02-07 | [snuggly-wibbling-pony.md](snuggly-wibbling-pony.md) | #289: セキュリティスキャン CI 追加 (cargo-deny) |
| 2026-02-07 | [validated-sniffing-waffle.md](validated-sniffing-waffle.md) | #279: TDD テストと品質保証テストの区別 |
| 2026-02-07 | [wondrous-orbiting-panda.md](wondrous-orbiting-panda.md) | 計画ファイルを git 管理下に配置する |
