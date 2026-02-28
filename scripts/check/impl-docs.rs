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
// - ディレクトリ名: 旧形式（連番プレフィックス）を拒否
// - ディレクトリ名: PR プレフィックスがある場合は PR<番号>_ 形式を検証
// - ファイル名: NN_<トピック>_{機能解説,コード解説}.md パターンに合致すること
// - ペアチェック: トピック単位で機能解説とコード解説がペアで存在すること
//
// 命名規則の定義: [命名規則](../../docs/90_実装解説/README.md)
//
// Usage: rust-script ./scripts/check/impl-docs.rs

use regex::Regex;
use std::collections::{HashMap, HashSet};

/// ドキュメントの種類
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DocType {
    Feature, // 機能解説
    Code,    // コード解説
}

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

    let old_format = Regex::new(r"^[0-9]+_").unwrap();
    let pr_prefix = Regex::new(r"^PR[0-9]+_").unwrap();
    let file_pattern = Regex::new(r"^[0-9]{2}_.+_(機能解説|コード解説)\.md$").unwrap();
    let topic_pattern = Regex::new(r"^[0-9]{2}_(.+)_(機能解説|コード解説)\.md$").unwrap();

    // ディレクトリ名チェック: 旧形式（連番プレフィックス）を拒否
    if old_format.is_match(&dir.dir_name) {
        errors.push(format!(
            "旧形式の連番ディレクトリ名です。PR<番号>_<機能名> に変更してください: {}/",
            dir.dir_path
        ));
        return ValidationResult { errors };
    }

    // PR プレフィックスがある場合は PR<数字>_ 形式を検証
    if dir.dir_name.starts_with("PR") && !pr_prefix.is_match(&dir.dir_name) {
        errors.push(format!(
            "ディレクトリ名が PR<番号>_<機能名> パターンに合致しません: {}/",
            dir.dir_path
        ));
        return ValidationResult { errors };
    }

    // ファイル名チェック
    for file_name in &dir.file_names {
        if !file_pattern.is_match(file_name) {
            errors.push(format!(
                "ファイル名が NN_<トピック>_{{機能解説,コード解説}}.md パターンに合致しません: {}/{}",
                dir.dir_path, file_name
            ));
        }
    }

    // ペアチェック: トピック単位で機能解説とコード解説がペアで存在するか
    let mut topics: HashMap<String, HashSet<DocType>> = HashMap::new();
    for file_name in &dir.file_names {
        if let Some(caps) = topic_pattern.captures(file_name) {
            let topic = caps[1].to_string();
            let doc_type = match &caps[2] {
                "機能解説" => DocType::Feature,
                "コード解説" => DocType::Code,
                _ => unreachable!(),
            };
            topics.entry(topic).or_default().insert(doc_type);
        }
    }

    // ペアのチェック — トピック名をソートして出力を安定化
    let mut sorted_topics: Vec<_> = topics.iter().collect();
    sorted_topics.sort_by_key(|(topic, _)| *topic);

    for (topic, doc_types) in sorted_topics {
        if !doc_types.contains(&DocType::Code) {
            errors.push(format!(
                "コード解説が欠如しています: {}/ トピック「{}」に機能解説はあるがコード解説がない",
                dir.dir_path, topic
            ));
        }
        if !doc_types.contains(&DocType::Feature) {
            errors.push(format!(
                "機能解説が欠如しています: {}/ トピック「{}」にコード解説はあるが機能解説がない",
                dir.dir_path, topic
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
    fn test_pr形式ディレクトリと正しいファイルペアでエラーなし() {
        let dir = make_dir(
            "PR123_認証機能",
            &["01_ログイン_機能解説.md", "01_ログイン_コード解説.md"],
        );
        let result = validate_dir(&dir);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    #[test]
    fn test_feature形式ディレクトリでエラーなし() {
        let dir = make_dir(
            "認証機能",
            &["01_ログイン_機能解説.md", "01_ログイン_コード解説.md"],
        );
        let result = validate_dir(&dir);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
    }

    // --- 異常系: ディレクトリ名 ---

    #[test]
    fn test_旧形式の連番ディレクトリ名でエラー() {
        let dir = make_dir("01_認証機能", &["01_ログイン_機能解説.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("旧形式の連番ディレクトリ名です"));
    }

    #[test]
    fn test_不正なprプレフィックスでエラー() {
        let dir = make_dir("PRx_認証機能", &["01_ログイン_機能解説.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("PR<番号>_<機能名> パターンに合致しません"));
    }

    // --- 異常系: ファイル名 ---

    #[test]
    fn test_不正なファイル名でエラー() {
        let dir = make_dir("PR123_認証機能", &["README.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0]
                .contains("NN_<トピック>_{機能解説,コード解説}.md パターンに合致しません")
        );
    }

    // --- 異常系: ペアチェック ---

    #[test]
    fn test_コード解説が欠如しているトピックでエラー() {
        let dir = make_dir("PR123_認証機能", &["01_ログイン_機能解説.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("コード解説が欠如しています"));
        assert!(result.errors[0].contains("ログイン"));
    }

    #[test]
    fn test_機能解説が欠如しているトピックでエラー() {
        let dir = make_dir("PR123_認証機能", &["01_ログイン_コード解説.md"]);
        let result = validate_dir(&dir);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("機能解説が欠如しています"));
        assert!(result.errors[0].contains("ログイン"));
    }

    // --- 異常系: 複数エラー ---

    #[test]
    fn test_複数のエラーを同時に検出() {
        let dir = make_dir(
            "PR123_認証機能",
            &[
                "README.md",               // 不正なファイル名
                "01_ログイン_機能解説.md", // コード解説が欠如
                "02_認可_コード解説.md",   // 機能解説が欠如
            ],
        );
        let result = validate_dir(&dir);
        // 不正ファイル名(1) + コード解説欠如(1) + 機能解説欠如(1) = 3 件
        assert_eq!(result.errors.len(), 3, "errors: {:?}", result.errors);
    }
}
