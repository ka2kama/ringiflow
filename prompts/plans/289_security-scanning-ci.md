# Plan: #289 Add security scanning to CI (cargo-deny)

## Context

Issue #289: CI にセキュリティスキャニングを追加する。

**Want**: 品質の追求（セキュリティ）— 依存関係に既知の脆弱性やライセンス違反がないことを継続的に保証する。

**要件定義書との対応**: CORE-02.2.3「セキュリティ監査において重大な脆弱性が検出されないこと」、12.8 節「依存関係スキャン | Rust クレート | cargo audit | 毎 CI」。

**現状**: Dependabot による週次の依存更新はあるが、CI パイプラインでの脆弱性スキャン・ライセンスチェックは未導入。

## ツール選定: cargo-deny のみ

cargo-audit ではなく **cargo-deny 単体** を採用する。

| 観点 | cargo-audit | cargo-deny | 両方 |
|------|-------------|------------|------|
| 脆弱性スキャン | ○ | ○（同じ RustSec Advisory DB） | 重複 |
| ライセンスチェック | × | ○ | ○ |
| 禁止クレート | × | ○ | ○ |
| ソース制限 | × | ○ | ○ |
| メンテナンスコスト | 低 | 低 | 高（2ツール管理） |
| CI Action | なし（cargo install） | `EmbarkStudios/cargo-deny-action@v2` | — |

要件定義書の「cargo audit」は機能要件（依存関係の脆弱性検出）の記述であり、cargo-deny の advisories チェックで要件を満たす。この判断は ADR-033 に記録する。

## 実装計画

### Phase 1: ローカルセットアップ & 設定

1. `cargo-deny` をインストール（`cargo install --locked cargo-deny`）
2. `cd backend && cargo deny list` でライセンス一覧を確認
3. `backend/deny.toml` を作成
4. `cd backend && cargo deny check` が通ることを確認
5. ライセンスエラーが出た場合、`allow` リストを調整

**`backend/deny.toml` の構成:**

```toml
[advisories]
vulnerability = "deny"      # 既知の脆弱性 → CI 失敗
unmaintained = "warn"       # 非メンテナンスクレート → 警告
yanked = "warn"             # yank されたクレート → 警告
notice = "warn"             # 通知レベル → 警告

[licenses]
unlicensed = "deny"         # ライセンス不明 → CI 失敗
confidence-threshold = 0.8
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    # 実際の `cargo deny list` 結果に基づいて調整
]

[bans]
multiple-versions = "warn"  # 同一クレートの複数バージョン → 警告
wildcards = "allow"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### Phase 2: justfile 統合

1. `check-tools` に `cargo-deny` を追加（行 42 の `cargo-watch` の後）:
   ```
   @which cargo-deny > /dev/null || (echo "ERROR: cargo-deny がインストールされていません" && exit 1)
   ```

2. 「API テスト」セクションと「全チェック」セクションの間に新セクションを追加:
   ```just
   # =============================================================================
   # セキュリティチェック
   # =============================================================================

   # 依存関係の脆弱性・ライセンスチェック（cargo-deny）
   audit:
       cd backend && cargo deny check
   ```

3. `check` タスクに `audit` を追加:
   ```just
   check: lint test test-rust-integration build-elm sqlx-check audit
   ```

### Phase 3: CI 統合

`.github/workflows/ci.yaml` に `security` ジョブを追加（`api-test` の後、`ci-success` の前）:

```yaml
  # セキュリティチェック: 依存関係の脆弱性・ライセンスチェック
  security:
    name: Security
    runs-on: ubuntu-latest
    needs: changes
    if: needs.changes.outputs.rust == 'true'

    steps:
      - name: Checkout
        uses: actions/checkout@v6

      - name: Check dependencies
        uses: EmbarkStudios/cargo-deny-action@v2
        with:
          manifest-path: backend/Cargo.toml
          command: check
          arguments: --all-features
```

`ci-success` の更新:
- `needs` に `security` を追加
- `Check results` に security チェックを追加

### Phase 4: ドキュメント

1. **ADR-033**: `docs/70_ADR/033_セキュリティスキャニングツールの選定.md`
   - コンテキスト: 要件定義書 12.8 節への対応
   - 検討した選択肢: cargo-audit のみ / cargo-deny のみ / 両方
   - 決定: cargo-deny のみ
   - 帰結: 多層チェック（advisories + licenses + bans + sources）、deny.toml の管理

2. **開発環境構築手順書**: `docs/60_手順書/01_開発参画/01_開発環境構築.md`
   - 16 番目のツールとして cargo-deny を追加
   - インストール方法、バージョン確認を記載

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/deny.toml` | 新規作成 |
| `justfile` | audit タスク追加、check-tools 更新、check 更新 |
| `.github/workflows/ci.yaml` | security ジョブ追加、ci-success 更新 |
| `docs/70_ADR/033_セキュリティスキャニングツールの選定.md` | 新規作成 |
| `docs/60_手順書/01_開発参画/01_開発環境構築.md` | cargo-deny の追加 |

## スコープ境界

**対象外**:
- npm（pnpm）側のセキュリティスキャニング（将来の別 Issue）
- コンテナスキャン（trivy）— Docker イメージが未作成のため現時点では不要
- SAST（semgrep）— clippy が既に CI に含まれている
- DAST（OWASP ZAP）— 週次、本番環境向け

## 検証

1. `cargo install --locked cargo-deny` が成功する
2. `cd backend && cargo deny check` がローカルで通る
3. `just check-tools` が通る
4. `just audit` が通る
5. `just check` が通る（audit を含む）
6. CI の security ジョブが通る（PR で確認）

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 | cargo-deny-action の manifest-path パラメータの確認 | `manifest-path: backend/Cargo.toml` が必要。Action は独自に cargo-deny バイナリを取得するため Rust ツールチェインのセットアップ不要 |
| 2回目 | ライセンスリスト | allow リストの事前確定は可能か | `cargo deny list` の実行が必要。計画では想定リストを記載し、実装時に調整する方針 |
| 3回目 | check vs check-all | cargo deny check の実行速度確認 | ビルド不要で数秒。`check`（軽量チェック）に含めて問題なし |
| 4回目 | 要件定義書との整合 | 12.8 節の「cargo audit」表記との矛盾 | cargo-deny の advisories チェックは同一 DB を使用。上位互換であり要件を満たす。ADR-033 に記録 |
| 5回目 | npm スコープ | Issue #289 に npm が含まれるか | 完了基準に npm なし。スコープ外として明記 |
| 6回目 | cargo-deny-action バージョン | 最新バージョンの確認 | v2.0.15（cargo-deny 0.19.0 同梱）を確認 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue #289 の完了基準 4 項目（CI 追加、deny.toml 設定、check-tools 追加、手順書更新）をすべてカバー。追加で ADR も作成 |
| 2 | 曖昧さ排除 | OK | licenses allow リストは「実装時に cargo deny list で確定」と手順を明記。他に不確定要素なし |
| 3 | 設計判断の完結性 | OK | ツール選定（3 択）、check vs check-all、独立ジョブ vs 既存ジョブ、npm スコープの 4 判断にすべて理由あり |
| 4 | スコープ境界 | OK | 対象（5 ファイル）と対象外（4 項目）を明記 |
| 5 | 技術的前提 | OK | cargo-deny-action v2 の manifest-path サポート、Action が独自にバイナリ取得（Rust 不要）、cargo deny check がビルド不要であることを確認済み |
| 6 | 既存ドキュメント整合 | OK | 要件定義書 12.8 節、ADR-004（CI ジョブ構造）、ADR-018（cross-spawn 脆弱性対応パターン）と整合 |
