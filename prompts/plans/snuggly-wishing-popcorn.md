# 計画: CI/justfile シェルスクリプトチェック強化

## 目的

CI YAML と justfile 内のインラインシェルスクリプトに対して静的解析を適用し、品質を担保する。

## 実装内容

### 1. actionlint の導入

**actionlint**: GitHub Actions の YAML ファイル専用リンター。構文チェックに加え、内部で shellcheck を使ってシェルスクリプトもチェックする。

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `justfile` | `check-tools` に actionlint 追加、`lint-ci` タスク新設 |
| `.github/workflows/ci.yml` | `shell` ジョブで actionlint も実行 |
| `docs/04_手順書/01_開発参画/01_開発環境構築.md` | actionlint インストール手順追加 |

### 2. justfile 内シェルスクリプトの外部化

複雑なシェルブロックを `scripts/` に切り出す。

#### 対象の選定基準

| 基準 | 外部化する | 外部化しない |
|------|-----------|-------------|
| 複雑さ | ループ、関数、trap 等を含む | 単純な条件分岐のみ |
| 行数 | 15行以上 | 数行 |
| 再利用性 | CI でも使う可能性あり | justfile 専用 |

#### 対象ブロック

| タスク | 行数 | 複雑さ | 判定 |
|--------|------|--------|------|
| `test-api` | 30行 | trap, ループ, ヘルスチェック | ✓ 外部化 |
| `worktree-add` | 50行 | 配列, ループ, ポート計算 | ✓ 外部化 |
| `dev-deps` | 5行 | Docker起動のみ | ✗ 維持 |
| `clean` | 5行 | Docker停止のみ | ✗ 維持 |
| `fmt-rust`, `fmt-elm`, `lint-shell` | 5行 | 単純な条件分岐 | ✗ 維持 |
| `worktree-remove` | 10行 | Docker停止 + worktree削除 | ✗ 維持 |

#### 作成するファイル

| ファイル | 元タスク | 内容 |
|---------|---------|------|
| `scripts/run-api-tests.sh` | `test-api` | サービス起動・ヘルスチェック・テスト実行 |
| `scripts/worktree-add.sh` | `worktree-add` | ポートオフセット計算・worktree作成 |

### 3. CI ワークフロー更新

#### ci.yml の変更

```yaml
# shell ジョブに actionlint を追加
shell:
  steps:
    - name: Lint shell scripts
      run: just lint-shell
    - name: Lint GitHub Actions
      run: actionlint
```

### 4. paths-filter 更新

CI YAML の変更検出を追加:

```yaml
ci:
  - '.github/workflows/**'
```

## 実装順序

1. actionlint をツールチェーンに追加（check-tools, ドキュメント）
2. `scripts/run-api-tests.sh` を作成
3. `scripts/worktree-add.sh` を作成
4. justfile を更新（外部スクリプト呼び出し、lint-ci タスク追加）
5. ci.yml を更新（actionlint 実行、paths-filter 追加）

## 検証方法

```bash
# 1. ローカルで actionlint を実行
just lint-ci

# 2. shellcheck で新スクリプトをチェック
just lint-shell

# 3. API テストが動作することを確認
just test-api

# 4. worktree 作成が動作することを確認（オプション）
just worktree-add test-wt feature/test
just worktree-remove test-wt
```
