# 計画: #373 AI が生成するコードの DRY 違反を検出・改善する仕組みの導入

## Context

AI 主導の開発において、コンテキストウィンドウの制約により、過去に書いたコードとの類似性を AI が能動的に検出できない。結果として ~730 行の重複コードが蓄積している。行動規範（「DRY を意識しろ」）は認知の問題であり構造的に解決困難。機械的な検出の仕組みを導入する。

## 問題解決フレームワーク

| 項目 | 内容 |
|------|------|
| Want | 品質の追求（保守性: モジュール性、修正性） |
| To-Be | DRY 違反が CI で機械的に検出され、設計原則レンズで具体的に確認される状態 |
| As-Is | 検出ツールなし、設計原則レンズは宣言的な問いのみ、~730 行の既存重複 |
| ギャップ | ツール不在 + プロセスの非具体性 |
| 根本原因 | AI の構造的限界（コンテキストウィンドウ外の情報を認知できない） |
| 対策 | (1) 検出ツール導入 (2) プロセス具体化 (3) 既存重複の Issue 化 |

## スコープ

- 対象: 仕組みの導入（ツール + CI + プロセス改善 + フォローアップ Issue 作成）
- 対象外: 既存重複コードの実際のリファクタリング（別 Issue で段階的に対応）

## 推奨ツール: jscpd

| 観点 | jscpd | duplicate_code | PMD CPD |
|------|-------|----------------|---------|
| 対応言語 | 150+（Rust + Elm 両対応） | Rust のみ | 多言語（Rust 非確認） |
| エコシステム | Node.js / npx（既存） | cargo install | JDK 必要 |
| 出力形式 | console, json, html, markdown | JSON | XML, CSV |
| メンテナンス | 活発 | 不明 | 活発だが重い |
| CI 統合 | `npx --yes jscpd@latest` | `cargo install` 要 | JDK セットアップ要 |

選定理由:
1. Rust + Elm 両対応（プロジェクトの両言語をカバー）
2. Node.js 既存（`lint-openapi` で `npx @redocly/cli@latest` パターン確認済み）
3. `npx` で即時実行（`check-tools` への追加不要）
4. 成熟度: 150+ 言語対応、活発なメンテナンス

注意: Elm の対応状況は Phase 1 でローカル試行時に検証する。

## Phase 構成

### Phase 1: ツール評価 + 設定ファイル + justfile 統合

#### 確認事項
- ライブラリ: jscpd CLI オプション → `npx --yes jscpd@latest --help` で確認
- ライブラリ: jscpd の Rust/Elm サポート状況 → ローカル実行で確認
- パターン: `npx` 使用の既存パターン → `justfile` L249 (`npx --yes @redocly/cli@latest`)
- パターン: `check-file-size.sh` の警告パターン → `scripts/check-file-size.sh`

#### 成果物
1. ローカルで jscpd を試行し、既知の重複を検出できるか確認
2. `.jscpd.json` をプロジェクトルートに作成
3. `justfile` に `check-duplicates` タスクを追加
4. `justfile` の `check` タスクに `check-duplicates` を追加

#### jscpd 設定方針

```json
{
  "threshold": 0,
  "minLines": 10,
  "minTokens": 50,
  "format": ["rust", "haskell"],
  "ignore": [
    "**/target/**",
    "**/node_modules/**",
    "**/elm-stuff/**"
  ],
  "reporters": ["console"],
  "absolute": true
}
```

設定根拠:
- `minLines: 10`: 10 行未満はノイズ（use 文、短い match 等）。運用しながら調整
- `minTokens: 50`: デフォルト。トークン数でも最小粒度を制御
- `threshold: 0`: 閾値なし。検出結果を全て表示（CI ブロックは別途制御）
- `format`: Rust は `rust`、Elm は `haskell`（jscpd が Elm を直接サポートしない場合、Haskell トークナイザーで代用を試す）→ Phase 1 で検証
- テストファイル除外は検討事項: 既知の Mock 重複も検出したい場合は除外しない。ただし false positive が多すぎれば除外を追加

#### justfile 変更

```just
# 構造品質チェック セクションに追加
# コード重複（コピー＆ペースト）を検出（jscpd）
# 選定理由: docs/70_ADR/042_コピペ検出ツールの選定.md
check-duplicates:
    npx --yes jscpd@latest .
```

`check` タスク更新:
```just
check: lint test test-rust-integration build-elm sqlx-check audit check-file-size check-duplicates
```

#### テストリスト
- [ ] `npx --yes jscpd@latest --help` が動作する
- [ ] `.jscpd.json` の設定で `just check-duplicates` が動作する
- [ ] BFF Client の `match response.status()` パターンが検出される（既知の重複）
- [ ] Infra 層の Row→Entity 変換が検出される（既知の重複）
- [ ] Elm ファイルが対象に含まれる（format 設定の検証）

### Phase 2: CI 統合

#### 確認事項
- パターン: CI ジョブの構造 → `.github/workflows/ci.yaml`
- パターン: `actions/setup-node@v6` + Node.js 22 → ci.yaml L212-215
- パターン: 変更検出フィルタ → ci.yaml L42-55
- パターン: `ci-success` のチェック → ci.yaml L421-469

#### 設計判断: 専用 `code-quality` ジョブ

jscpd は Rust + Elm 両方を検査するため、既存の `rust` / `elm` ジョブどちらにも入れられない。専用ジョブを作成する。

- 変更検出: `backend/**` or `frontend/**` のいずれかが変更された場合にトリガー
- `ci-success` には追加**しない**（初期は警告のみ。既存重複があるため CI をブロックしない）

#### CI ジョブ設計

```yaml
code-quality:
  name: Code Quality
  runs-on: ubuntu-latest
  needs: changes
  if: needs.changes.outputs.rust == 'true' || needs.changes.outputs.elm == 'true'

  steps:
    - name: Checkout
      uses: actions/checkout@v6

    - name: Setup Node.js
      uses: actions/setup-node@v6
      with:
        node-version: '22'

    - name: Install just
      uses: extractions/setup-just@v3

    - name: Check code duplicates
      run: just check-duplicates
```

注意: 既存の Action のみ使用（`actions/checkout@v6`, `actions/setup-node@v6`, `extractions/setup-just@v3`）。GitHub Actions 許可設定の追加は不要。

#### テストリスト
- [ ] `backend/**` 変更時に `code-quality` ジョブが実行される
- [ ] `frontend/**` 変更時に `code-quality` ジョブが実行される
- [ ] `docs/**` のみの変更時にジョブがスキップされる
- [ ] jscpd の結果が CI ログに表示される
- [ ] ジョブが exit 0 で完了する（重複があっても CI は通る）

### Phase 3: ADR 作成

#### 確認事項
- パターン: ADR-038 の構造 → `docs/70_ADR/038_未使用依存検出ツールの選定.md`
- パターン: ADR テンプレート → `docs/70_ADR/template.md`
- 連番: 最新 041 → `042_コピペ検出ツールの選定.md`

#### 成果物

`docs/70_ADR/042_コピペ検出ツールの選定.md` を ADR-038 と同じ構造で作成:
- コンテキスト: AI のコンテキストウィンドウ制約による DRY 違反の構造的発生
- 選択肢: jscpd / duplicate_code / PMD CPD
- 決定: jscpd（理由: 多言語対応、既存エコシステム、成熟度）
- 帰結: CI での自動検出、初期は警告のみ

Phase 1 の実行結果（検出精度、Elm サポート状況）を反映して記述する。

#### テストリスト
- [ ] ADR テンプレートの構造に準拠している
- [ ] 3 候補の比較表がある
- [ ] Phase 1 の検証結果が反映されている

### Phase 4: プロセス改善

#### 確認事項
- 型: 設計原則レンズの現在の記述 → `docs/60_手順書/04_開発フロー/02_TDD開発フロー.md` L130-137
- パターン: structural-review.md の構造 → `.claude/rules/structural-review.md`

#### 4a: 設計原則レンズの具体化

TDD 開発フロー `02_TDD開発フロー.md` の設計原則レンズ（L135）を更新:

現状:
```markdown
| 重複の排除 | 同じ知識が複数箇所に存在していないか？ |
```

改善後:
```markdown
| 重複の排除 | 同じ知識が複数箇所に存在していないか？新しく書いたコードの主要パターン（関数シグネチャ、match ブロック、変換コード等）を Grep で既存コードと照合する |
```

#### 4b: structural-review.md に重複検出セクション追加

`.claude/rules/structural-review.md` に以下を追加:

```markdown
## コード重複の検出

`just check-duplicates` でコピー＆ペーストを検出できる。

新しいコードを追加したとき、または既存コードを拡張したときに実行する。

### AI エージェントへの指示

TDD Refactor ステップで、以下を実施する:

1. 書いたコードの主要パターンのキーワードを Grep で既存コードと照合する
2. 類似パターンが見つかった場合、共通化を検討する
3. 共通化が現スコープ外であれば、セッションログに記録する
```

#### テストリスト
- [ ] TDD 開発フローの設計原則レンズが具体的な Grep アクションを含む
- [ ] structural-review.md に重複検出セクションが追加されている
- [ ] AI エージェントへの指示が実行可能である

### Phase 5: フォローアップ Issue 作成

#### 確認事項: なし（既知のパターンのみ）

Phase 1 の jscpd 実行結果をもとに、以下の Issue を作成:

**Issue A**: BFF Client のレスポンスハンドリング共通化
- 対象: `backend/apps/bff/src/client/core_service/` の 19 箇所の `match response.status()`
- ラベル: `backend`, `enhancement`

**Issue B**: Infra 層の Row→Entity 変換共通化
- 対象: `backend/crates/infra/src/repository/` の 6 箇所の変換コード
- ラベル: `backend`, `enhancement`

**Issue C**: UseCase テストの Mock リポジトリ共通化
- 対象: `backend/apps/core-service/src/usecase/` の ~370 行の重複 Mock
- ラベル: `backend`, `enhancement`

#### テストリスト
- [ ] 3 つの Issue が作成されている
- [ ] 各 Issue に jscpd の検出結果が引用されている

## 変更対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.jscpd.json`（新規） | jscpd 設定ファイル |
| `justfile` | `check-duplicates` タスク追加、`check` に統合 |
| `.github/workflows/ci.yaml` | `code-quality` ジョブ追加 |
| `docs/70_ADR/042_コピペ検出ツールの選定.md`（新規） | ツール選定の ADR |
| `docs/60_手順書/04_開発フロー/02_TDD開発フロー.md` | 設計原則レンズ更新 |
| `.claude/rules/structural-review.md` | 重複検出セクション追加 |

## 検証方法

1. `just check-duplicates` がローカルで動作し、既知の重複を検出する
2. `just check-all` が通る
3. CI の `code-quality` ジョブが動作する（PR 作成後に確認）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | jscpd の Elm サポートが不確実 | 技術的前提 | Phase 1 でローカル検証する手順を明記。Haskell トークナイザーでの代用案も記載 |
| 2回目 | CI ジョブの配置が rust/elm どちらにも不適切 | アーキテクチャ不整合 | 言語横断の専用 `code-quality` ジョブを新設する判断を追加 |
| 3回目 | CI ブロック vs 警告の判断が未定義 | 曖昧 | 既存重複があるため初期は警告のみ。`ci-success` に追加しない判断を明記 |
| 4回目 | `check-tools` への追加要否が未検討 | 未定義 | `npx` 実行のためグローバルインストール不要、追加不要と判断 |
| 5回目 | テストファイル除外が `check-file-size.sh` と整合するか | 既存パターン整合 | Mock 重複も検出対象としたいため、テスト除外はしない方針に変更。false positive が多ければ後で調整 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の 3 つの対策方向がカバーされている | OK | ツール導入(Phase 1-2)、プロセス改善(Phase 4)、Issue作成(Phase 5) |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | Elm サポートは Phase 1 で検証と明記。他は具体的なファイルパス・設定値を記載 |
| 3 | 設計判断の完結性 | 全差異に判断が記載されている | OK | ツール選定(3候補比較)、CIジョブ配置、ci-success追加要否、check-tools追加要否 |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | 対象: 仕組みの導入。対象外: 既存重複のリファクタリング |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | npx の CI 利用パターン確認済み、GitHub Actions 許可設定の影響なし |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | ADR-038(同種選定)、structural-review.md、Issue #151 と整合 |
