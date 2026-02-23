---
paths:
  - "openapi/**"
  - "backend/.sqlx/**"
  - "backend/schema.sql"
  - "backend/apps/bff/tests/snapshots/**"
---

# VCS 管理の生成物

VCS にコミットされているが、手動編集してはならない生成物の一覧。

## 対応表

| 生成物 | ソースオブトゥルース | 生成コマンド |
|--------|-------------------|-------------|
| `openapi/openapi.yaml` | Rust 構造体の utoipa アノテーション | `just openapi-generate` |
| `backend/.sqlx/` | SQL クエリマクロ | `just sqlx-prepare` |
| `backend/schema.sql` | PostgreSQL スキーマ（マイグレーション） | `just db-dump-schema` |
| `backend/apps/bff/tests/snapshots/*.snap` | テスト実行結果 | テスト実行 → snapshot 更新 |

## ルール

- 対応表のファイルを直接編集しない。ソースオブトゥルース側を変更してから生成コマンドを実行する
- 新しい生成パイプラインを追加した場合は対応表を更新する

改善の経緯: [生成物の直接編集](../../process/improvements/2026-02/2026-02-22_2108_生成物の直接編集.md)
