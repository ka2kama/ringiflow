# GitHub Ruleset

GitHub Ruleset は、ブランチ保護ルールをより柔軟に管理するための機能。
従来の Branch Protection Rules の後継として推奨されている。

## CLI での操作

```bash
# 一覧取得
gh api repos/{owner}/{repo}/rulesets

# 詳細取得
gh api repos/{owner}/{repo}/rulesets/{ruleset_id}

# 作成
gh api repos/{owner}/{repo}/rulesets -X POST --input ruleset.json

# 更新
gh api repos/{owner}/{repo}/rulesets/{ruleset_id} -X PUT --input ruleset.json

# 削除
gh api repos/{owner}/{repo}/rulesets/{ruleset_id} -X DELETE
```

## JSON 構造とパラメータ解説

### 基本構造

```json
{
  "name": "main-protection",
  "target": "branch",
  "enforcement": "active",
  "conditions": { ... },
  "rules": [ ... ],
  "bypass_actors": [ ... ]
}
```

| パラメータ | 説明 |
|-----------|------|
| `name` | Ruleset の名前 |
| `target` | `branch` または `tag` |
| `enforcement` | `active`（有効）, `disabled`（無効）, `evaluate`（評価のみ、ブロックしない） |
| `conditions` | 適用対象のブランチ/タグパターン |
| `rules` | 適用するルールの配列 |
| `bypass_actors` | ルールをバイパスできるユーザー/チーム |

### conditions

```json
{
  "conditions": {
    "ref_name": {
      "include": ["refs/heads/main"],
      "exclude": []
    }
  }
}
```

| パラメータ | 説明 |
|-----------|------|
| `include` | 対象に含めるパターン（例: `refs/heads/main`, `refs/heads/release/*`） |
| `exclude` | 対象から除外するパターン |

### rules

#### deletion（ブランチ削除禁止）

```json
{"type": "deletion"}
```

対象ブランチの削除を禁止する。

#### non_fast_forward（強制プッシュ禁止）

```json
{"type": "non_fast_forward"}
```

`git push --force` を禁止する。履歴の改変を防ぐ。

#### required_signatures（署名必須）

```json
{"type": "required_signatures"}
```

署名付きコミットを必須にする。GPG または SSH 署名が必要。

#### pull_request（PR 必須）

```json
{
  "type": "pull_request",
  "parameters": {
    "required_approving_review_count": 1,
    "dismiss_stale_reviews_on_push": true,
    "required_reviewers": [],
    "require_code_owner_review": false,
    "require_last_push_approval": false,
    "required_review_thread_resolution": true,
    "allowed_merge_methods": ["squash"]
  }
}
```

| パラメータ | 説明 |
|-----------|------|
| `required_approving_review_count` | 必要な承認数（0 = 承認不要、1以上 = その数の承認が必要） |
| `dismiss_stale_reviews_on_push` | `true`: 新しいコミットがプッシュされたら既存の承認を取り消す |
| `required_reviewers` | 必須レビュワーのリスト |
| `require_code_owner_review` | `true`: CODEOWNERS で指定された人の承認が必要 |
| `require_last_push_approval` | `true`: 最後にプッシュした人以外の承認が必要（自己承認防止） |
| `required_review_thread_resolution` | `true`: すべてのレビューコメントが解決されないとマージ不可 |
| `allowed_merge_methods` | 許可するマージ方法: `merge`, `squash`, `rebase` |

#### required_status_checks（CI 必須）

```json
{
  "type": "required_status_checks",
  "parameters": {
    "strict_required_status_checks_policy": true,
    "do_not_enforce_on_create": false,
    "required_status_checks": [
      {"context": "CI Success", "integration_id": 15368}
    ]
  }
}
```

| パラメータ | 説明 |
|-----------|------|
| `strict_required_status_checks_policy` | `true`: ブランチが最新（main と同期済み）でないとマージ不可 |
| `do_not_enforce_on_create` | `true`: 新規ブランチ作成時はチェックをスキップ |
| `required_status_checks` | 必須の status check リスト |
| `required_status_checks[].context` | status check の名前（ワークフローのジョブ名） |
| `required_status_checks[].integration_id` | status check を提供するアプリの ID |

**integration_id の値:**

| ID | サービス |
|----|---------|
| 15368 | GitHub Actions |

他のサービス（CircleCI, Jenkins 等）を使う場合は、そのサービスの integration_id を指定する。

### bypass_actors

```json
{
  "bypass_actors": [
    {
      "actor_id": 1,
      "actor_type": "OrganizationAdmin",
      "bypass_mode": "always"
    }
  ]
}
```

| パラメータ | 説明 |
|-----------|------|
| `actor_id` | ユーザー/チーム/アプリの ID |
| `actor_type` | `OrganizationAdmin`, `RepositoryRole`, `Team`, `Integration` |
| `bypass_mode` | `always`（常にバイパス可）, `pull_request`（PR のみバイパス可） |

## 完全な設定例

```json
{
  "name": "main-protection",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "exclude": [],
      "include": ["refs/heads/main"]
    }
  },
  "rules": [
    {"type": "deletion"},
    {"type": "required_signatures"},
    {
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 1,
        "dismiss_stale_reviews_on_push": true,
        "required_reviewers": [],
        "require_code_owner_review": false,
        "require_last_push_approval": false,
        "required_review_thread_resolution": true,
        "allowed_merge_methods": ["squash"]
      }
    },
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "do_not_enforce_on_create": false,
        "required_status_checks": [
          {"context": "CI Success", "integration_id": 15368},
          {"context": "Auto Review", "integration_id": 15368}
        ]
      }
    },
    {"type": "non_fast_forward"}
  ],
  "bypass_actors": []
}
```

## 参考リンク

- [GitHub Docs: About rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/about-rulesets)
- [GitHub REST API: Repository rulesets](https://docs.github.com/en/rest/repos/rules)
