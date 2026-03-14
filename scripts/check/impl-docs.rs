#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! glob = "0.3"
//! regex = "1"
//! pretty_assertions = "1"
//! ```

// 実装解説ドキュメントのファイル命名規則をチェックする。
//
// チェック内容:
// - ディレクトリ名: 機能ドメイン名であること（旧 PR プレフィックス形式を拒否）
// - ファイル名: NN_<トピック>.md パターンに合致すること
//
// 命名規則の定義: [命名規則](../../docs/90_実装解説/README.md)
//
// Usage: rust-script ./scripts/check/impl-docs.rs

use regex::Regex;

/// ディレクトリ内のファイル情報
struct DirFiles {
    dir_name: String,
    dir_path: String,
    file_names: Vec<String>,
}

/// バリデーション結果
struct ValidationResult {
    errors: Vec<String>,
}

/// ディレクトリ単位のバリデーション（純粋関数）
fn validate_dir(dir: &DirFiles) -> ValidationResult {
    let mut errors = Vec::new();

    let old_numbered = Regex::new(r"^[0-9]+_").unwrap();
    let pr_prefix = Regex::new(r"^PR[0-9]+").unwrap();
    let file_pattern = Regex::new(r"^[0-9]{2}_.+\.md$").unwrap();
    let old_file_pattern = Regex::new(r"_(機能解説|コード解説)\.md$").unwrap();

    // ディレクトリ名チェック: 旧形式を拒否
    if old_numbered.is_match(&dir.dir_name) {
        errors.push(format!(
            "旧形式の連番ディレクトリ名です。機能ドメイン名に変更してください: {}/",
            dir.dir_path
        ));
        return ValidationResult { errors };
    }

    if pr_prefix.is_match(&dir.dir_name) {
        errors.push(format!(
            "旧形式の PR プレフィックスです。機能ドメイン名に変更してください: {}/",
            dir.dir_path
        ));
        return ValidationResult { errors };
    }

    // ファイル名チェック
    for file_name in &dir.file_names {
        // 旧形式のサフィックスを拒否
        if old_file_pattern.is_match(file_name) {
            errors.push(format!(
                "旧形式のファイル名です（_{} サフィックス）。NN_<トピック>.md に変更してください: {}/{}",
                if file_name.contains("機能解説") {
                    "機能解説"
                } else {
                    "コード解説"
                },
                dir.dir_path,
                file_name
            ));
            continue;
        }

        if !file_pattern.is_match(file_name) {
            errors.push(format!(
                "ファイル名が NN_<トピック>.md パターンに合致しません: {}/{}",
                dir.dir_path, file_name
            ));
        }
    }

    ValidationResult { errors }
}

fn run() -> i32 {
    let mut all_errors = Vec::new();

    let pattern = "docs/90_実装解説/*/";
    let paths = glob::glob(pattern).unwrap_or_else(|e| {
        panic!("glob パターンの解析に失敗: {e}");
    });

    for entry in paths {
        let path = entry.unwrap_or_else(|e| {
            panic!("パスの取得に失敗: {e}");
        });
        let dir_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let dir_path = path.to_string_lossy().to_string();

        // ディレクトリ内の .md ファイルを収集
        let file_pattern = format!("{}/*.md", dir_path);
        let file_names: Vec<String> = glob::glob(&file_pattern)
            .unwrap_or_else(|e| panic!("glob パターンの解析に失敗: {e}"))
            .filter_map(|entry| {
                let p = entry.ok()?;
                if p.is_file() {
                    Some(p.file_name()?.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect();

        // .gitkeep のみのディレクトリはスキップ
        if file_names.is_empty() {
            continue;
        }

        let dir_files = DirFiles {
            dir_name,
            dir_path,
            file_names,
        };

        let result = validate_dir(&dir_files);
        all_errors.extend(result.errors);
    }

    if !all_errors.is_empty() {
        println!(
            "⚠️  実装解説の命名規則違反が見つかりました ({} 件):",
            all_errors.len()
        );
        for error in &all_errors {
            println!("  - {error}");
        }
        return 1;
    }

    println!("✅ 実装解説のファイル命名規則に準拠しています");
    0
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn make_dir(dir_name: &str, file_names: &[&str]) -> DirFiles {
        DirFiles {
            dir_name: dir_name.to_string(),
            dir_path: format!("docs/90_実装解説/{}", dir_name),
            file_names: file_names.iter().map(|s| s.to_string()).collect(),
        }
    }

    // --- 正常系 ---

    #[test]
    fn test_機能ドメインディレクトリと正しいファイル名でエラーなし() {
        let dir = make_dir(
            "ワークフロー",
            &["01_申請フロー.md", "02_承認・却下フロー.md"],
        );
        let result = validate_dir(&dir);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_横断的関心事ディレクトリでエラーなし() {
        let dir = make_dir(
            "横断的関心事",
            &["01_マルチテナントRLS.md", "02_Observability.md"],
        );
        let result = validate_dir(&dir);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    // --- 異常系: ディレクトリ名 ---

    #[test]
    fn test_旧形式の連番ディレクトリ名でエラー() {
        let dir = make_dir("01_認証機能", &["01_認証フロー.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("旧形式の連番ディレクトリ名です"));
    }

    #[test]
    fn test_旧形式のprプレフィックスでエラー() {
        let dir = make_dir("PR123_認証機能", &["01_認証フロー.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("旧形式の PR プレフィックス"));
    }

    // --- 異常系: ファイル名 ---

    #[test]
    fn test_不正なファイル名でエラー() {
        let dir = make_dir("認証", &["README.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("NN_<トピック>.md パターンに合致しません"));
    }

    #[test]
    fn test_旧形式の機能解説サフィックスでエラー() {
        let dir = make_dir("ワークフロー", &["01_申請_機能解説.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("旧形式のファイル名です"));
        assert!(result.errors[0].contains("_機能解説"));
    }

    #[test]
    fn test_旧形式のコード解説サフィックスでエラー() {
        let dir = make_dir("ワークフロー", &["01_申請_コード解説.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("旧形式のファイル名です"));
        assert!(result.errors[0].contains("_コード解説"));
    }

    // --- 異常系: 複数エラー ---

    #[test]
    fn test_複数のエラーを同時に検出() {
        let dir = make_dir(
            "ワークフロー",
            &[
                "README.md",           // 不正なファイル名
                "01_申請_機能解説.md", // 旧形式
            ],
        );
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 2, "errors: {:?}", result.errors);
    }
}
