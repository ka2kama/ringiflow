#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! syn = { version = "2", features = ["full", "parsing"] }
//! proc-macro2 = { version = "1", features = ["span-locations"] }
//! tempfile = "3"
//! ```

// ハンドラとリポジトリ実装に #[tracing::instrument] が付与されているかチェックする。
//
// チェック対象:
// - backend/apps/*/src/handler/**/*.rs の pub async fn（health_check を除く）
// - backend/crates/infra/src/repository/**/*.rs の async fn（impl メソッドのみ、trait 署名は除く）
//
// Usage: rust-script ./scripts/check/instrumentation.rs

use std::fmt;

/// 除外する関数名
const EXCLUDE_FUNCTIONS: &[&str] = &["health_check"];

/// チェック対象の種類
enum TargetKind {
    Handler,
    RepositoryImpl,
}

/// 計装漏れのエラー情報
struct InstrumentationError {
    file: String,
    line: usize,
    kind: TargetKind,
    fn_name: String,
}

impl fmt::Display for InstrumentationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self.kind {
            TargetKind::Handler => "ハンドラ",
            TargetKind::RepositoryImpl => "リポジトリ impl",
        };
        write!(
            f,
            "{}:{}: {} {} に #[tracing::instrument] がありません",
            self.file, self.line, label, self.fn_name
        )
    }
}

/// 属性リストに tracing::instrument が含まれるか判定する
fn has_instrument_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        let segments: Vec<String> = attr
            .path()
            .segments
            .iter()
            .map(|seg| seg.ident.to_string())
            .collect();
        matches!(
            segments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
            ["tracing", "instrument"] | ["instrument"]
        )
    })
}

/// 関数名が除外リストに含まれるか判定する
fn is_excluded(fn_name: &str) -> bool {
    EXCLUDE_FUNCTIONS.contains(&fn_name)
}

/// ハンドラファイルをチェックする（pub async fn に #[tracing::instrument] があるか）
fn check_handler_file(path: &str, content: &str) -> Vec<InstrumentationError> {
    let file = syn::parse_file(content).unwrap_or_else(|e| {
        panic!("{path} の構文解析に失敗: {e}");
    });
    let mut errors = Vec::new();

    for item in &file.items {
        if let syn::Item::Fn(item_fn) = item {
            if !matches!(item_fn.vis, syn::Visibility::Public(_)) {
                continue;
            }
            if item_fn.sig.asyncness.is_none() {
                continue;
            }

            let fn_name = item_fn.sig.ident.to_string();
            if is_excluded(&fn_name) {
                continue;
            }

            if !has_instrument_attr(&item_fn.attrs) {
                let line = item_fn.sig.fn_token.span.start().line;
                errors.push(InstrumentationError {
                    file: path.to_string(),
                    line,
                    kind: TargetKind::Handler,
                    fn_name,
                });
            }
        }
    }

    errors
}

/// リポジトリファイルをチェックする（impl メソッドの async fn に #[tracing::instrument] があるか）
fn check_repository_file(path: &str, content: &str) -> Vec<InstrumentationError> {
    let file = syn::parse_file(content).unwrap_or_else(|e| {
        panic!("{path} の構文解析に失敗: {e}");
    });
    let mut errors = Vec::new();

    for item in &file.items {
        if let syn::Item::Impl(item_impl) = item {
            for impl_item in &item_impl.items {
                if let syn::ImplItem::Fn(method) = impl_item {
                    if method.sig.asyncness.is_none() {
                        continue;
                    }

                    let fn_name = method.sig.ident.to_string();
                    if is_excluded(&fn_name) {
                        continue;
                    }

                    if !has_instrument_attr(&method.attrs) {
                        let line = method.sig.fn_token.span.start().line;
                        errors.push(InstrumentationError {
                            file: path.to_string(),
                            line,
                            kind: TargetKind::RepositoryImpl,
                            fn_name,
                        });
                    }
                }
            }
        }
    }

    errors
}

/// 指定パターンのファイル一覧を git ls-files で取得する
fn git_ls_files(pattern: &str) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["ls-files", pattern])
        .output()
        .expect("git ls-files の実行に失敗");

    if !output.status.success() {
        eprintln!("git ls-files が失敗しました");
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

fn run() -> i32 {
    let mut errors = Vec::new();

    // ハンドラチェック
    let handler_files: Vec<String> = git_ls_files("backend/apps/*/src/handler/**/*.rs")
        .into_iter()
        .filter(|f| !f.ends_with("tests.rs"))
        .collect();

    for file in &handler_files {
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("{file} の読み込みに失敗: {e}"));
        errors.extend(check_handler_file(file, &content));
    }

    // リポジトリチェック
    let repo_files = git_ls_files("backend/crates/infra/src/repository/**/*.rs");

    for file in &repo_files {
        let content = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("{file} の読み込みに失敗: {e}"));
        errors.extend(check_repository_file(file, &content));
    }

    // 結果出力
    if errors.is_empty() {
        println!("✅ すべてのハンドラ・リポジトリに計装が設定されています");
        0
    } else {
        println!("❌ 計装漏れが見つかりました ({} 件):", errors.len());
        for error in &errors {
            println!("  - {error}");
        }
        1
    }
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- has_instrument_attr ---

    #[test]
    fn test_has_instrument_attr_tracing_instrument属性がある場合にtrueを返す() {
        let code = r#"
            #[tracing::instrument(skip_all)]
            pub async fn login() {}
        "#;
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Fn(item_fn) = &file.items[0] {
            assert!(has_instrument_attr(&item_fn.attrs));
        } else {
            panic!("Item::Fn が期待される");
        }
    }

    #[test]
    fn test_has_instrument_attr_instrument属性の短縮形でもtrueを返す() {
        let code = r#"
            #[instrument(skip_all)]
            pub async fn login() {}
        "#;
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Fn(item_fn) = &file.items[0] {
            assert!(has_instrument_attr(&item_fn.attrs));
        } else {
            panic!("Item::Fn が期待される");
        }
    }

    #[test]
    fn test_has_instrument_attr_他の属性のみの場合にfalseを返す() {
        let code = r#"
            #[allow(unused)]
            pub async fn login() {}
        "#;
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Fn(item_fn) = &file.items[0] {
            assert!(!has_instrument_attr(&item_fn.attrs));
        } else {
            panic!("Item::Fn が期待される");
        }
    }

    #[test]
    fn test_has_instrument_attr_属性なしの場合にfalseを返す() {
        let code = r#"
            pub async fn login() {}
        "#;
        let file = syn::parse_file(code).unwrap();
        if let syn::Item::Fn(item_fn) = &file.items[0] {
            assert!(!has_instrument_attr(&item_fn.attrs));
        } else {
            panic!("Item::Fn が期待される");
        }
    }

    // --- is_excluded ---

    #[test]
    fn test_is_excluded_health_checkは除外される() {
        assert!(is_excluded("health_check"));
    }

    #[test]
    fn test_is_excluded_通常の関数名は除外されない() {
        assert!(!is_excluded("login"));
        assert!(!is_excluded("find_by_email"));
    }

    // --- check_handler_file ---

    #[test]
    fn test_check_handler_file_pub_async_fnに計装ありでエラーなし() {
        let code = r#"
            #[tracing::instrument(skip_all)]
            pub async fn login() {}
        "#;
        let errors = check_handler_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_handler_file_pub_async_fnに計装なしでエラーあり() {
        let code = r#"
            pub async fn login() {}
        "#;
        let errors = check_handler_file("test.rs", code);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].fn_name, "login");
    }

    #[test]
    fn test_check_handler_file_health_checkは計装なしでもエラーなし() {
        let code = r#"
            pub async fn health_check() {}
        "#;
        let errors = check_handler_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_handler_file_非pubのasync_fnはチェック対象外() {
        let code = r#"
            async fn helper() {}
        "#;
        let errors = check_handler_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_handler_file_pubだが非asyncのfnはチェック対象外() {
        let code = r#"
            pub fn sync_helper() {}
        "#;
        let errors = check_handler_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_handler_file_複数の関数で漏れのある関数のみエラー() {
        let code = r#"
            #[tracing::instrument(skip_all)]
            pub async fn login() {}

            pub async fn logout() {}

            #[tracing::instrument(skip_all)]
            pub async fn session() {}
        "#;
        let errors = check_handler_file("test.rs", code);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].fn_name, "logout");
    }

    // --- check_repository_file ---

    #[test]
    fn test_check_repository_file_impl内のasync_fnに計装ありでエラーなし() {
        let code = r#"
            struct Repo;
            impl Repo {
                #[tracing::instrument(skip_all)]
                async fn find_by_id(&self) -> Result<(), ()> {
                    Ok(())
                }
            }
        "#;
        let errors = check_repository_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_repository_file_impl内のasync_fnに計装なしでエラーあり() {
        let code = r#"
            struct Repo;
            impl Repo {
                async fn find_by_id(&self) -> Result<(), ()> {
                    Ok(())
                }
            }
        "#;
        let errors = check_repository_file("test.rs", code);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].fn_name, "find_by_id");
    }

    #[test]
    fn test_check_repository_file_trait定義のasync_fnはチェック対象外() {
        let code = r#"
            trait UserRepository {
                async fn find_by_email(&self) -> Result<(), ()>;
            }
        "#;
        let errors = check_repository_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_repository_file_impl内の非async_fnはチェック対象外() {
        let code = r#"
            struct Repo;
            impl Repo {
                fn new() -> Self {
                    Repo
                }
            }
        "#;
        let errors = check_repository_file("test.rs", code);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_repository_file_traitとimplが混在する場合にimplのみチェック() {
        let code = r#"
            trait UserRepository {
                async fn find_by_email(&self) -> Result<(), ()>;
            }

            struct Repo;
            impl UserRepository for Repo {
                async fn find_by_email(&self) -> Result<(), ()> {
                    Ok(())
                }
            }
        "#;
        let errors = check_repository_file("test.rs", code);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].fn_name, "find_by_email");
    }

    #[test]
    fn test_check_repository_file_除外関数はimplメソッドでもスキップ() {
        let code = r#"
            struct Repo;
            impl Repo {
                async fn health_check(&self) {}
            }
        "#;
        let errors = check_repository_file("test.rs", code);
        assert!(errors.is_empty());
    }

    // --- 出力フォーマット ---

    #[test]
    fn test_ハンドラのエラーメッセージフォーマットが正しい() {
        let error = InstrumentationError {
            file: "backend/apps/bff/src/handler/auth/login.rs".to_string(),
            line: 10,
            kind: TargetKind::Handler,
            fn_name: "login".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "backend/apps/bff/src/handler/auth/login.rs:10: ハンドラ login に #[tracing::instrument] がありません"
        );
    }

    #[test]
    fn test_リポジトリのエラーメッセージフォーマットが正しい() {
        let error = InstrumentationError {
            file: "backend/crates/infra/src/repository/user_repository.rs".to_string(),
            line: 25,
            kind: TargetKind::RepositoryImpl,
            fn_name: "find_by_email".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "backend/crates/infra/src/repository/user_repository.rs:25: リポジトリ impl find_by_email に #[tracing::instrument] がありません"
        );
    }
}
