# rust-script

## 概要

[rust-script](https://rust-script.org/) は、単一ファイルの Rust スクリプトを実行するツール。Cargo プロジェクト構造（`Cargo.toml` + `src/`）を用意せずに、スクリプト内にインラインで依存関係を宣言できる。

Python や Shell の手軽さと、Rust の型安全性・エコシステムを両立する。

## 基本的な使い方

### インライン依存宣言

```rust
#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! globset = "0.4"
//! serde = { version = "1", features = ["derive"] }
//! ```

use globset::Glob;

fn main() {
    // ...
}
```

`//!` ドキュメントコメント内に Cargo.toml のフォーマットで依存を記述する。

### 実行

```bash
# スクリプト実行
rust-script script.rs

# テスト実行
rust-script --test script.rs

# 引数付き実行
rust-script script.rs arg1 arg2
```

### 初回コンパイル

初回実行時にコンパイルが発生する（数秒〜十数秒）。コンパイル結果は `~/.cache/rust-script/` にキャッシュされ、ソースが変更されない限り再コンパイルは不要。

## プロジェクトでの使用箇所

### CI スクリプト

`.github/scripts/match-rules.rs` — PR の変更ファイルにマッチする `.claude/rules/` のルールを特定するスクリプト。

選定理由: ADR-015（開発スクリプトの品質担保方針）の移行基準に基づき、複雑なロジック（glob パターンマッチング）を含むスクリプトを Rust に移行した。詳細は ADR-056 を参照。

### 使い分け

| スコープ | 言語 | 根拠 |
|---------|------|------|
| `scripts/` | Shell | ローカル開発環境の依存を最小化（ADR-015） |
| `.github/scripts/` | Rust（rust-script）| 技術スタック一致、複雑なロジックに適する（ADR-056） |
| ワークフロー内のインライン `run` | Shell | ワークフロー YAML 内は Shell が自然 |

## 注意点

### テスト用依存

rust-script は `[dev-dependencies]` を区別しない。テスト専用の依存（`tempfile` 等）も `[dependencies]` に記述する。

```rust
//! ```cargo
//! [dependencies]
//! globset = "0.4"
//! tempfile = "3"   # テスト用だが [dependencies] に記述
//! ```
```

### globset との組み合わせ

`globset` のデフォルトでは `*` がパス区切り `/` を超えてマッチする。Shell や Python の glob と同じ挙動にするには `literal_separator(true)` が必要。

```rust
use globset::GlobBuilder;

// デフォルト: *.rs が src/main.rs にもマッチする（意図しない挙動の可能性）
let m = Glob::new("*.rs").unwrap().compile_matcher();
assert!(m.is_match("src/main.rs")); // true

// literal_separator: *.rs は main.rs のみにマッチ（Shell/Python 互換）
let m = GlobBuilder::new("*.rs")
    .literal_separator(true)
    .build()
    .unwrap()
    .compile_matcher();
assert!(!m.is_match("src/main.rs")); // false
assert!(m.is_match("main.rs"));      // true
```

## 将来の移行パス

`cargo script`（RFC 3424）が安定化されれば、`rust-script`（外部ツール）から `cargo` の組み込み機能に移行でき、追加ツールのインストールが不要になる。

移行手順（安定化後）:
1. `#!/usr/bin/env rust-script` → `#!/usr/bin/env cargo`
2. CI から `cargo install rust-script` ステップを削除

## 関連リソース

- [rust-script 公式サイト](https://rust-script.org/)
- [globset ドキュメント (docs.rs)](https://docs.rs/globset/)
- [RFC 3424: cargo script](https://rust-lang.github.io/rfcs/3424-cargo-script.html)
- ADR: [ADR-015 開発スクリプトの品質担保方針](../../05_ADR/015_開発スクリプトの品質担保方針.md)
- ADR: [ADR-056 CI スクリプトの言語選定方針](../../05_ADR/056_CIスクリプトの言語選定方針.md)
