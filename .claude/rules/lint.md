---
paths:
  - "**/*.rs"
  - "**/*.elm"
  - "**/*.ts"
  - "**/*.tsx"
  - "**/*.js"
  - "**/*.jsx"
---

# Lint ルール

lint（clippy, elm-review, ESLint 等）に関するルール。

## 禁止: lint エラーの無効化による回避

**AI エージェントは、lint エラーを無効化ディレクティブで回避することを禁止する。**

### 禁止される行為

lint エラーが発生した際に、以下のような無効化ディレクティブを追加してエラーを抑制すること:

| 言語 | 禁止される無効化 |
|------|------------------|
| Rust | `#[allow(...)]`, `#[expect(...)]`, `#![allow(...)]` |
| Elm | `-- elm-review: IGNORE` |
| TypeScript/JavaScript | `// eslint-disable-...`, `/* eslint-disable */` |

### 正しい対応

lint エラーが発生した場合は、**コードを修正してエラーを解消する**。

```rust
// Bad: lint を無効化して回避
#[allow(dead_code)]
fn unused_function() { ... }

// Good: 不要なら削除、必要なら使用する
// （削除した）
```

```rust
// Bad: clippy の警告を無効化
#[allow(clippy::unnecessary_wraps)]
fn always_ok() -> Result<(), Error> {
    Ok(())
}

// Good: 警告の意図を理解して修正
fn always_ok() {
    // Result でラップする必要がないなら Result を返さない
}
```

### 例外

以下の場合のみ、無効化が許容される:

1. **ユーザーの明示的な指示がある場合**
   - ユーザーが「この警告は無効化して」と明示的に指示した場合
2. **技術的に回避不可能な場合**
   - ライブラリの設計上どうしても必要な場合（例: FFI、マクロ生成コード）
   - この場合も、ユーザーに確認を取ってから実施する

例外に該当する場合は、必ず FIXME コメントを付ける（[code-annotations.md](code-annotations.md) 参照）:

```rust
// FIXME: #[allow(dead_code)] を解消する（〇〇の理由で一時的に抑制）
#[allow(dead_code)]
struct TemporarilyUnused { ... }
```

## 参照

- コードアノテーション規約: [code-annotations.md](code-annotations.md)
- Rust 実装ルール: [rust.md](rust.md)
