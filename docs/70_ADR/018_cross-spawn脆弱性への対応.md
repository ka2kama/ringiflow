# ADR-018: cross-spawn 脆弱性への対応

## ステータス

承認済み

## コンテキスト

Dependabot が `cross-spawn` パッケージの ReDoS（Regular Expression Denial of Service）脆弱性を検出した。

- CVE: CVE-2024-21538
- 深刻度: High（CVSS 7.5）
- 影響: 巧妙に作成された文字列で CPU 使用率を上げ、プログラムをクラッシュ可能
- 修正バージョン: 6.0.6 以上、または 7.0.5 以上

依存関係の状況:

```
ringiflow-web
├── elm-review → cross-spawn 7.0.6 ✓ 修正済み
├── elm-test → cross-spawn 7.0.6 ✓ 修正済み
└── vite-plugin-elm → node-elm-compiler → cross-spawn 6.0.5 ❌ 脆弱
```

`node-elm-compiler` の最新版（5.0.6）でも脆弱な 6.0.5 を使用しており、上流での修正が行われていない。

## 検討した選択肢

### 選択肢 1: pnpm overrides で強制上書き

pnpm の overrides 機能を使い、脆弱なバージョンを修正済みバージョンに強制的に置き換える。

評価:

- 利点: 即座に修正可能、上流の対応を待つ必要がない
- 欠点: 上流が修正されても自動反映されないため、定期的な確認が必要

### 選択肢 2: 上流の修正を待つ

`node-elm-compiler` または `vite-plugin-elm` が依存を更新するのを待つ。

評価:

- 利点: 正攻法であり、overrides の管理が不要
- 欠点: いつ修正されるか不明、その間アラートが残り続ける

### 選択肢 3: 代替パッケージへの移行

`vite-plugin-elm` を別の Elm ビルドツールに置き換える。

評価:

- 利点: 根本的な解決
- 欠点: 大きな変更が必要、同等の代替が存在しない可能性

### 比較表

| 観点 | overrides | 上流待ち | 代替移行 |
|------|-----------|----------|----------|
| 即時性 | ◎ | × | △ |
| 保守コスト | △ | ◎ | × |
| リスク | 低（パッチバージョン） | 中（放置） | 高（大変更） |

## 決定

選択肢 1: pnpm overrides で強制上書きを採用する。

理由:

1. パッチバージョンの差（6.0.5 → 6.0.6）であり、互換性リスクが極めて低い
2. 上流の修正時期が不明であり、アラートを放置する習慣は避けたい
3. 対応が簡単で、将来不要になれば overrides を削除するだけ

## 帰結

### 肯定的な影響

- Dependabot アラートが解消される
- セキュリティ意識の高い開発体制を維持できる

### 否定的な影響・トレードオフ

- 上流が修正されても自動では反映されない
- 定期的に overrides が不要になったか確認する運用が必要

### 運用ルール

上流パッケージの更新時に overrides の要否を確認する:

```bash
# 確認コマンド
cd frontend && pnpm why cross-spawn
```

すべての `cross-spawn` が 6.0.6 以上または 7.0.5 以上になったら、overrides を削除する。

### 関連ドキュメント

- Dependabot アラート: https://github.com/ka2kama/ringiflow/security/dependabot/2
- CVE: https://nvd.nist.gov/vuln/detail/CVE-2024-21538

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-19 | 初版作成 |
