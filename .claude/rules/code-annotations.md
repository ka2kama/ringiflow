---
paths:
  - "**/*.rs"
  - "**/*.elm"
---

# コードアノテーション規約

コード内で「後から対応すべき箇所」を示すためのコメント規約。

## 使用するタグ

| タグ | 用途 | 緊急度 |
|------|------|--------|
| **TODO** | やるべきこと（機能追加、改善、削除） | 低〜中 |
| **FIXME** | 修正すべき問題（バグ、一時的な回避策、警告抑制） | 高 |

### TODO

将来やるべきこと（動作に問題はないが改善の余地がある箇所）。

```rust
// TODO(#80): Phase 4 で password_hash フィールドを削除する
```

### FIXME

修正すべき問題（一時的な回避策、警告の抑制など）。

```rust
// FIXME: #[allow(dead_code)] を解消する（使うか削除する）
#[allow(dead_code)]
struct UnusedStruct { ... }
```

## 形式

```
// <TAG>(<ISSUE>): <やること>
```

- `<TAG>`: `TODO` または `FIXME`
- `<ISSUE>`: Issue 番号（任意）。関連 Issue がある場合は `#123` 形式で記載
- `<やること>`: 具体的なアクション。「〜する」で終わる

注意: 行コメント（`//`）を使用する。ドキュメントコメント（`///`）には書かない。

### 良い例

```rust
// TODO(#80): password_hash カラムを削除する
// FIXME: #[allow(dead_code)] を解消する
// TODO: get_user API でロール詳細を返す
```

### 悪い例

```rust
// TODO: 後で直す                    // 何をするか不明
// TODO: パフォーマンス改善が必要     // 具体的なアクションがない
// FIXME                             // 説明がない
```

## 使い分けの判断基準

| 状況 | タグ |
|------|------|
| 機能追加・改善 | TODO |
| コードの削除 | TODO |
| `#[allow(...)]` で警告を抑制中 | FIXME |
| 一時的な回避策（HACK） | FIXME |
| バグ・問題のあるコード | FIXME |

## 検索方法

```bash
# 全ての TODO/FIXME を一覧
grep -rn "TODO\|FIXME" backend --include="*.rs"

# FIXME のみ（優先度高）
grep -rn "FIXME" backend --include="*.rs"
```
