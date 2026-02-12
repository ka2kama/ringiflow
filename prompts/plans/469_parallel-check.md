# 計画: just check の並列実行による高速化 (#469)

## Context

`just check` が 14 タスクを逐次実行し、~54 秒かかっている。
40 秒以下に短縮する。2 つの施策を実施する:

1. **並列実行**: Rust レーンと Non-Rust レーンの 2 レーン並列化（期待 -17s）
2. **npm ツールのローカルインストール**: `npx --yes @latest` のオーバーヘッド排除（期待 -5s）

## スコープ

対象:
- `justfile` の `check` レシピを並列化
- ルートに `package.json` を新規作成（`@redocly/cli`, `jscpd`）
- `lint-openapi`, `check-duplicates` を `pnpm exec` に変更
- CI ワークフローに `pnpm install` ステップを追加
- 関連ドキュメント更新

対象外:
- `check-all` の並列化（`check` + `audit` + `test-api` + `test-e2e` の並列化は別 Issue）
- `lint`, `test` 単体レシピの並列化（スタンドアロンでの使用を維持）
- Non-Rust レーン内部の並列化（効果が小さくアウトプット管理が複雑化）

## 設計判断

### 1. 並列化手法: シェルレベル（`&` + `wait`）

`just` v1.46.0 にはビルトインの並列実行機能がない。bash スクリプトで 2 レーンを並列実行する。

代替案:
- GNU parallel → 外部依存が増える
- `just` の依存チェーン維持 → 並列化不可能
- **採用**: シェルの `&` + `wait`（依存なし、シンプル）

### 2. レーン分割

| レーン | タスク | 理由 |
|--------|--------|------|
| Rust（逐次） | lint-rust → test-rust → test-rust-integration → sqlx-check → openapi-check | Cargo ロック競合を回避 |
| Non-Rust（逐次） | lint-elm → test-elm → build-elm → lint-shell → lint-ci → lint-openapi → check-unused-deps → check-file-size → check-duplicates | Rust と独立 |

- `check-unused-deps`（`cargo machete`）: ファイル読み取りのみ、Cargo ビルドロック不要 → Non-Rust レーン
- Non-Rust レーン内は逐次: タスク個々が高速（~0.1-5s）なため、並列化の効果 < 出力管理の複雑さ

### 3. 出力ハンドリング

- Rust レーン: フォアグラウンド（リアルタイム出力）— 実行時間が長く進捗確認が重要
- Non-Rust レーン: バックグラウンド（一時ファイルにバッファ）— Rust レーン完了後に表示

### 4. エラーハンドリング

- 両レーンとも完了まで実行（1 回で全失敗を把握）
- Rust レーン内は早期終了（失敗したら後続スキップ、ビルド時間の節約）
- 終了コード: いずれかのレーンが失敗 → 非ゼロ

### 5. npm ツール管理: ルート `package.json`

ルートに `package.json` を作成し、`pnpm exec` で実行する。

代替案:
- バージョン固定 npx（`npx --yes @redocly/cli@2.18.0`）→ まだ npx のオーバーヘッドあり
- `frontend/package.json` に追加 → プロジェクトツールをフロントエンドに混在させるのは意味的に不適切
- **採用**: ルート `package.json`（npx オーバーヘッド排除、バージョン固定、再現性）

根拠:
- `.gitignore` は既に `node_modules/` をカバー
- `pnpm-workspace.yaml` 不在のため、ルートと `frontend/` は独立した pnpm プロジェクト

---

## Phase 1: ルート `package.json` の作成

### 確認事項
- [ ] パターン: `frontend/package.json` と `tests/e2e/package.json` の構造 → 既存パターンを参照
- [ ] ライブラリ: `@redocly/cli` 最新版 → `npm view` で確認済み: 2.18.0
- [ ] ライブラリ: `jscpd` 最新版 → `npm view` で確認済み: 4.0.8
- [ ] 型: `pnpm exec` コマンドの使用法 → Grep 既存使用パターン

### 変更内容

1. `/package.json` を新規作成:
   ```json
   {
     "private": true,
     "devDependencies": {
       "@redocly/cli": "^2.18.0",
       "jscpd": "^4.0.8"
     }
   }
   ```
2. `pnpm install` を実行して `pnpm-lock.yaml` を生成

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- [ ] `pnpm install` が成功する
- [ ] `pnpm exec redocly --version` が動作する
- [ ] `pnpm exec jscpd --version` が動作する

---

## Phase 2: justfile の npx → pnpm exec 変更

### 確認事項
- [ ] パターン: 現在の `lint-openapi` の引数パターン → justfile L257-258
- [ ] パターン: 現在の `check-duplicates` の引数パターン → justfile L342-347

### 変更内容

1. `lint-openapi` (L257-258): `npx --yes @redocly/cli@latest` → `pnpm exec redocly`
2. `check-duplicates` (L342-347): `npx --yes jscpd@latest` → `pnpm exec jscpd`
3. `setup` チェーンに `setup-root-deps` を追加
4. `setup-root-deps` レシピを新規追加

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- [ ] `just lint-openapi` が成功する
- [ ] `just check-duplicates` が成功する

---

## Phase 3: check レシピの並列化

### 確認事項
- [ ] パターン: 既存の bash shebang レシピ（`dev-deps` 等） → justfile L128-135

### 変更内容

`check` レシピ（L378-379）を bash スクリプトに変更:

```just
check:
    #!/usr/bin/env bash
    set -euo pipefail

    non_rust_log=$(mktemp)
    trap 'rm -f "$non_rust_log"' EXIT

    # Non-Rust レーン（バックグラウンド）
    (
        set -e
        just lint-elm
        just test-elm
        just build-elm
        just lint-shell
        just lint-ci
        just lint-openapi
        just check-unused-deps
        just check-file-size
        just check-duplicates
    ) > "$non_rust_log" 2>&1 &
    non_rust_pid=$!

    # Rust レーン（フォアグラウンド）
    rust_ok=true
    just lint-rust &&
    just test-rust &&
    just test-rust-integration &&
    just sqlx-check &&
    just openapi-check || rust_ok=false

    # Non-Rust レーンの完了待ち
    non_rust_ok=true
    wait $non_rust_pid || non_rust_ok=false

    echo ""
    echo "=== Non-Rust チェック ==="
    cat "$non_rust_log"

    # 結果判定
    if ! $rust_ok || ! $non_rust_ok; then
        echo ""
        ! $rust_ok && echo "✗ Rust レーン: 失敗"
        ! $non_rust_ok && echo "✗ Non-Rust レーン: 失敗"
        exit 1
    fi
    echo ""
    echo "✓ 全チェック完了"
```

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- [ ] `just check` が成功する
- [ ] `time just check` が 40 秒以下
- [ ] Rust レーン失敗時: Non-Rust レーンも完了し、両方の結果が表示される
- [ ] Non-Rust レーン失敗時: Rust レーンも完了し、両方の結果が表示される
- [ ] `just check-all` が成功する（check を前提とするため）

---

## Phase 4: CI ワークフローの更新

### 確認事項
- [ ] パターン: CI の既存 pnpm セットアップ → ci.yaml の elm ジョブ等

### 変更内容

CI の `openapi` ジョブ（L305-324）と `code-quality` ジョブ（L591-610）に pnpm セットアップ + install を追加:

1. `openapi` ジョブ: `Setup pnpm` + `Install root dev tools` ステップを追加（`Setup Node.js` の後）
2. `code-quality` ジョブ: 同上

追加するステップ:
```yaml
      - name: Setup pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 10

      - name: Install root dev tools
        run: pnpm install --frozen-lockfile
```

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- [ ] CI の `openapi` ジョブが成功する
- [ ] CI の `code-quality` ジョブが成功する

---

## Phase 5: ドキュメント更新

### 確認事項
- なし（既知のパターンのみ）

### 変更内容

1. `docs/06_ナレッジベース/devtools/RedoclyCLI.md` L23, L164: `npx` コマンド例を `pnpm exec redocly` に更新
2. `docs/05_ADR/042_コピペ検出ツールの選定.md` L51, L61: `npx` の記述を `pnpm exec` に更新
3. 開発環境構築手順に「ルート npm ツールのインストール」を追加（`just setup` で自動化されるため最小限の記述）

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- [ ] ドキュメント内のリンク切れがないこと

---

## 予測パフォーマンス

| レーン | タスク | 時間 |
|--------|--------|------|
| Rust（逐次） | lint-rust → test-rust → test-rust-integration → sqlx-check → openapi-check | ~34s |
| Non-Rust（逐次） | lint-elm → test-elm → build-elm → lint-shell → lint-ci → lint-openapi → check-unused-deps → check-file-size → check-duplicates | ~10s（npx 排除後） |
| **合計** | max(Rust, Non-Rust) | **~34s** |

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `check-unused-deps` のレーン配置が未定義 | 未定義 | `cargo machete` はファイル読み取りのみでビルドロック不要 → Non-Rust レーンに配置 |
| 2回目 | `@redocly/cli` のバージョンが Plan agent の想定（1.31.0）と実際（2.18.0）で不一致 | 技術的前提 | `npm view` で最新版を確認し修正: @redocly/cli@2.18.0, jscpd@4.0.8 |
| 3回目 | Rust レーン内のエラーハンドリング方針が未定義 | 未定義 | 早期終了（失敗したら後続スキップ）を採用。ビルド時間の節約が理由 |
| 4回目 | CI ワークフローに `pnpm/action-setup` が必要だが、GitHub Actions の許可設定への影響が未検討 | 既存手段の見落とし | 確認済み: `pnpm/action-setup@v4` は既に `elm` ジョブ(L244)と `e2e-test` ジョブ(L465)で使用済み。許可設定テーブルにも登録済み（GitHubActions.md L70）。追加対応不要 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | justfile の `check` 依存タスク 14 個すべてがいずれかのレーンに配置されていることを確認 |
| 2 | 曖昧さ排除 | OK | 各レーンのタスク順序、出力ハンドリング、エラーハンドリングを明示 |
| 3 | 設計判断の完結性 | OK | 並列化手法、レーン分割、npm 管理方式、出力・エラー処理にそれぞれ代替案と理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明記（`check-all` の並列化、Non-Rust 内部並列化は対象外） |
| 5 | 技術的前提 | OK | just v1.46.0 に並列機能なし、pnpm-workspace.yaml 不在で独立動作を確認 |
| 6 | 既存ドキュメント整合 | OK | ADR-042（jscpd 選定）、ナレッジベース（Redocly CLI）の更新を計画に含む |
