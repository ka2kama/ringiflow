# TODO/FIXME ライフサイクル管理の改善

## Context

ソースコード内の TODO/FIXME コメントに対する Issue 追跡の不備が発見された:

1. `FIXME(#688)` がクローズ済み Issue を参照したまま残存
2. Issue 未追跡の TODO が 2 件存在（保存→申請連続処理、KPI カードデザイン）
3. Issue クローズ時に関連 TODO/FIXME を棚卸しする仕組みがない

ハイブリッド方針（プロセス改善 + 自動 lint）で対処する。知識-実行乖離率が高い（45%）ため、手動プロセスのみでは不十分。プロジェクトの「改善記録 → lint 自動検証」パターン（#843, #845）に従う。

## 対象

- 未追跡 TODO/FIXME の Issue 化と参照更新
- `scripts/check/stale-annotations.sh` 新規作成
- `justfile` / `scripts/check/parallel.sh` への組み込み
- `.claude/rules/code-annotations.md` にライフサイクルルール追加
- `docs/04_手順書/04_開発フロー/01_Issue駆動開発.md` に棚卸しステップ追加
- 改善記録の作成

## 対象外

- Issue 番号なしの TODO/FIXME の自動検出（既存ルールで Issue 番号は任意）
- `frontend/review/src/ReviewConfig.elm` の TODO 4 件（Phase 3 / BFF 連携時に自然解消。Epic #405, #406 のスコープ）
- CI ワークフロー（`ci.yaml`）への直接追加（`parallel.sh` → `just check` 経由で十分。必要なら別 Issue で検討）

---

## Phase 1: Issue 化と改善記録

目的: 問題の記録と既存の陳腐化アノテーションの解消

### 1.1 改善記録の作成

`process/improvements/2026-02/2026-02-24_HHMM_TODO-FIXMEのライフサイクル管理不在.md`

- カテゴリ: プロセスギャップ
- 事象: FIXME(#688) がクローズ済み Issue を参照、未追跡 TODO が複数存在
- 原因: Issue クローズ時の TODO/FIXME 棚卸しフローが未定義
- 対策: 自動 lint（Phase 2）+ プロセス改善（Phase 3）

### 1.2 Issue 作成（3 件）

| # | 内容 | ラベル | 元のアノテーション |
|---|------|-------|------------------|
| A | WorkflowUseCaseImpl の deps 構造体統合（too_many_arguments 解消） | `backend`, `enhancement` | `FIXME(#688)` in `workflow.rs:127` |
| B | ワークフロー申請の保存→申請連続処理 | `frontend`, `enhancement` | `TODO` in `New.elm:807` |
| C | ダッシュボード KPI カードのデザイン実装 | `frontend`, `enhancement` | `TODO(human)` in `Home.elm:114` |

### 1.3 アノテーション更新

- `backend/apps/core-service/src/usecase/workflow.rs:127`: `FIXME(#688)` → `FIXME(#<A>)` に更新
- `frontend/src/Page/Workflow/New.elm:807`: `TODO` → `TODO(#<B>)` に更新
- `frontend/src/Page/Home.elm:114`: `TODO(human)` はそのまま維持（人間実装の目印 + Issue で追跡）

#### 確認事項
- パターン: 改善記録のフォーマット → `process/improvements/README.md`
- パターン: Issue 作成のフォーマット → `docs/04_手順書/04_開発フロー/01_Issue駆動開発.md`

#### 操作パス
該当なし（ドキュメント・Issue 管理のみ）

#### テストリスト
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 2: 自動 lint スクリプト作成

目的: クローズ済み Issue を参照する TODO/FIXME を自動検出する

### 2.1 `scripts/check/stale-annotations.sh` の作成

アルゴリズム:
1. `git ls-files` で VCS 管理下の `*.rs` / `*.elm` を取得
2. `grep -nE '(TODO|FIXME)\(#[0-9]+\)'` でアノテーションを抽出
3. 一意な Issue 番号を収集
4. `gh issue view <N> --json state` で各 Issue の状態を取得
5. CLOSED/NOT_FOUND な Issue を参照するアノテーションをエラーとして報告

設計判断:
- **言語: bash** — `doc-links.sh`, `file-size.sh` と同パターン。grep + gh で十分
- **API 呼び出し**: `gh issue view` の個別呼び出し。アノテーション数は 10 件未満が想定され、バッチ不要
- **`gh` 未インストール/未認証時**: 警告を出してスキップ（exit 0）。ローカル未設定環境をブロックしない
- **出力**: 既存スクリプトの `✅` / `❌` パターンに統一
- **exit code**: クローズ済み参照あり → exit 1（lint としてブロック）

### 2.2 justfile にレシピ追加

`justfile` の「構造品質チェック」セクション（`check-doc-links` の後、L424 付近）に追加:

```just
# クローズ済み Issue を参照する TODO/FIXME を検出
check-stale-annotations:
    ./scripts/check/stale-annotations.sh
```

### 2.3 `parallel.sh` の Non-Rust レーンに追加

`just check-doc-links` の後に `just check-stale-annotations` を追加（L42 付近）。

#### 確認事項
- パターン: `file-size.sh` の `git ls-files` 使用 → `scripts/check/file-size.sh`
- パターン: `doc-links.sh` のエラー報告 → `scripts/check/doc-links.sh`
- ライブラリ: `gh issue view --json state --jq '.state'` の出力形式 → 実行確認

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | クローズ済み Issue 参照を検出してエラー報告 | 正常系 | 手動テスト |
| 2 | 全参照先 Issue が OPEN の場合に正常終了 | 正常系 | 手動テスト |
| 3 | Issue 番号付き TODO/FIXME が存在しない場合に正常終了 | 正常系 | 手動テスト |
| 4 | `gh` 未インストール時にスキップ | 準正常系 | 手動テスト |

#### テストリスト

ユニットテスト:
- [ ] クローズ済み Issue 参照の検出（exit 1）
- [ ] 全 OPEN 時の正常終了（exit 0）
- [ ] アノテーションなし時の正常終了（exit 0）
- [ ] `gh` 未利用時のスキップ（exit 0）
- [ ] 同一 Issue の重複 API 呼び出し防止

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 3: ドキュメント更新

目的: ルールとフローにライフサイクル管理を組み込む

### 3.1 `.claude/rules/code-annotations.md` に「ライフサイクル」セクション追加

末尾に追加:

```markdown
## ライフサイクル

### Issue 参照の維持

TODO/FIXME に Issue 番号を付けた場合、参照先 Issue のクローズ時に棚卸しが必要:

| 状態 | 対応 |
|------|------|
| アノテーションの内容が解消済み | アノテーションを削除 |
| 未解消（Issue のスコープ外だった） | 新 Issue を作成し、参照番号を更新 |

### 自動検出

`just check-stale-annotations` でクローズ済み Issue を参照する TODO/FIXME を検出する。`just check` に含まれるため、プッシュ前に自動実行される。
```

### 3.2 Issue 駆動開発フロー Step 9 に棚卸しステップ追加

`docs/04_手順書/04_開発フロー/01_Issue駆動開発.md` の「改善記録の検証」セクション（L713）の後に追加:

```markdown
#### TODO/FIXME の棚卸し

Issue クローズ時に、クローズした Issue を参照する TODO/FIXME がないか確認する。

\```bash
grep -rn "TODO(#<Issue番号>)\|FIXME(#<Issue番号>)" backend/ frontend/
\```

| 状態 | 対応 |
|------|------|
| アノテーションなし | 追加アクション不要 |
| 解消済み | アノテーションを削除 |
| 未解消 | 新 Issue を作成し、参照番号を更新 |

注: 自動 lint（`just check-stale-annotations`）が補完するため、手動ステップの漏れはプッシュ前に検出される。
```

#### 確認事項
- パターン: `code-annotations.md` の現在のセクション構造 → 既読（L1-83）
- パターン: Issue 駆動開発 Step 9 の構造 → 既読（L655-742）

#### 操作パス
該当なし（ドキュメント修正のみ）

#### テストリスト
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## 検証

1. `just check-stale-annotations` を実行し、Phase 1 でアノテーション更新前は FIXME(#688) が検出されることを確認
2. アノテーション更新後、再実行してエラーがゼロになることを確認
3. `just check` を実行し、スクリプトが `parallel.sh` 経由で正常に動作することを確認
4. `just check-doc-links` でドキュメント内リンク切れがないことを確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ReviewConfig.elm の TODO 4 件の扱いが未定義 | 曖昧 | Phase 3/BFF 連携で自然解消するため対象外に明記 |
| 2回目 | GraphQL vs REST の API 呼び出し方式が未確定 | 技術的前提 | アノテーション数 <10 のため個別 REST 呼び出しで十分。シンプルさ優先 |
| 3回目 | TODO(human) の Issue 化方針が曖昧 | 曖昧 | アノテーションは維持（人間実装の目印）、Issue で追跡と明記 |
| 4回目 | CI 直接追加の判断根拠が不足 | アーキテクチャ不整合 | `parallel.sh` → `just check` 経由で十分。必要なら別 Issue で対応と対象外に明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Issue 化 3 件、lint スクリプト、justfile、parallel.sh、code-annotations.md、Issue 駆動開発フロー、改善記録 — すべて Phase 1-3 に配置 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | TODO(human) の扱い、ReviewConfig.elm の除外理由、CI 非対象の理由を明記 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | bash vs rust-script、REST vs GraphQL、exit code の方針に理由付き |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象セクションと対象外セクションで明確に区分 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `gh` CLI の可用性（CI: GITHUB_TOKEN、ローカル: gh auth）を確認事項に含む |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | code-annotations.md の Issue 番号任意ルールと整合（番号付きのみ検出） |
