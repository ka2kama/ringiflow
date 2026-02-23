#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! globset = "0.4"
//! tempfile = "3"
//! ```

// PR の変更ファイルにマッチする .claude/rules/ のルールを特定し、内容を出力する。
//
// 使い方:
//     rust-script match-rules.rs <changed-files.txt>
//
// 入力: 変更ファイル一覧（1 行 1 パス）
// 出力: マッチしたルールの名前リスト + 各ルールの本文（フロントマター除去済み）

use globset::{GlobBuilder, GlobMatcher};
use std::fs;
use std::path::Path;

/// glob パターンをコンパイルする。
///
/// `literal_separator(true)` により `*` がパス区切りを超えない
/// （Python 版の `[^/]*` と同等の挙動）。
fn compile_glob(pattern: &str) -> Option<GlobMatcher> {
    GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .ok()
        .map(|g| g.compile_matcher())
}

/// YAML フロントマターから paths パターンを抽出する。
///
/// フォーマット:
/// ---
/// paths:
///   - "pattern1"
///   - "pattern2"
/// ---
fn parse_frontmatter_paths(content: &str) -> Vec<String> {
    let mut lines = content.lines();

    // 先頭が --- でなければフロントマターなし
    match lines.next() {
        Some(line) if line.trim() == "---" => {}
        _ => return Vec::new(),
    }

    let mut paths = Vec::new();
    let mut in_paths = false;

    for line in lines {
        let trimmed = line.trim();

        if trimmed == "---" {
            break;
        }

        if trimmed == "paths:" {
            in_paths = true;
            continue;
        }

        if in_paths {
            if let Some(value) = trimmed.strip_prefix("- ") {
                let pattern = value.trim().trim_matches('"').trim_matches('\'');
                paths.push(pattern.to_string());
            } else if !trimmed.is_empty() {
                // paths セクション以外のキーに遭遇 → paths 終了
                break;
            }
        }
    }

    paths
}

/// YAML フロントマターを除去してルール本文を返す。
fn strip_frontmatter(content: &str) -> &str {
    let mut lines = content.lines();

    // 先頭が --- でなければフロントマターなし
    match lines.next() {
        Some(line) if line.trim() == "---" => {}
        _ => return content,
    }

    // 2つ目の --- を探す
    let mut end_offset = 0;
    let mut found = false;
    for (i, line) in content.lines().enumerate() {
        if i > 0 && line.trim() == "---" {
            // i 行目の終わりまでのバイトオフセットを計算
            end_offset = content
                .lines()
                .take(i + 1)
                .map(|l| l.len() + 1) // +1 for \n
                .sum::<usize>();
            found = true;
            break;
        }
    }

    if !found {
        return content;
    }

    // フロントマター以降の本文を返す（先頭の空行を除去）
    content[end_offset..].trim_start_matches('\n')
}

/// マッチしたルールの情報
struct MatchedRule {
    /// ルールファイルの相対パス（例: ".claude/rules/rust.md"）
    path: String,
    /// フロントマター除去済みの本文
    body: String,
}

/// 変更ファイルにマッチするルールを返す。
///
/// ルールファイルはファイル名の辞書順でソートされる。
fn match_rules(changed_files: &[String], rules_dir: &Path) -> Vec<MatchedRule> {
    let mut rule_files: Vec<_> = fs::read_dir(rules_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
        .collect();

    rule_files.sort_by_key(|e| e.file_name());

    let mut matched = Vec::new();

    for entry in &rule_files {
        let content = match fs::read_to_string(entry.path()) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let patterns = parse_frontmatter_paths(&content);
        if patterns.is_empty() {
            continue;
        }

        // いずれかのパターンがいずれかの変更ファイルにマッチすれば OK
        let matchers: Vec<_> = patterns.iter().filter_map(|p| compile_glob(p)).collect();
        let rule_matched = matchers
            .iter()
            .any(|m| changed_files.iter().any(|f| m.is_match(f)));

        if rule_matched {
            let body = strip_frontmatter(&content);
            matched.push(MatchedRule {
                path: rules_dir.join(entry.file_name()).display().to_string(),
                body: body.to_string(),
            });
        }
    }

    matched
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: match-rules.rs <changed-files.txt>");
        std::process::exit(1);
    }

    let changed_files_path = &args[1];
    let rules_dir = Path::new(".claude/rules");

    let content = match fs::read_to_string(changed_files_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("エラー: {changed_files_path} の読み込みに失敗: {e}");
            std::process::exit(1);
        }
    };

    let changed_files: Vec<String> = content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    if changed_files.is_empty() {
        println!("<!-- no-matching-rules -->");
        return;
    }

    let matched = match_rules(&changed_files, rules_dir);

    if matched.is_empty() {
        println!("<!-- no-matching-rules -->");
        return;
    }

    // サマリー
    println!("マッチしたルール: {} 件\n", matched.len());
    for rule in &matched {
        println!("- `{}`", rule.path);
    }
    println!();

    // 各ルールの本文
    for rule in &matched {
        println!("### {}\n", rule.path);
        println!("{}", rule.body);
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // --- parse_frontmatter_paths ---

    #[test]
    fn test_parse_frontmatter_paths_標準的なフロントマターからpathsを抽出() {
        let content = r#"---
paths:
  - "**/*.rs"
  - "frontend/**"
---
# ルール本文
"#;
        let paths = parse_frontmatter_paths(content);
        assert_eq!(paths, vec!["**/*.rs", "frontend/**"]);
    }

    #[test]
    fn test_parse_frontmatter_paths_フロントマターがない場合は空() {
        let content = "# ルール本文\nただのテキスト";
        let paths = parse_frontmatter_paths(content);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_paths_pathsキーがない場合は空() {
        let content = "---\ntitle: ルール\n---\n# 本文";
        let paths = parse_frontmatter_paths(content);
        assert!(paths.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_paths_クォートが除去される() {
        let content = "---\npaths:\n  - \"**/*.rs\"\n  - '**/*.elm'\n---\n";
        let paths = parse_frontmatter_paths(content);
        assert_eq!(paths, vec!["**/*.rs", "**/*.elm"]);
    }

    // --- strip_frontmatter ---

    #[test]
    fn test_strip_frontmatter_フロントマターを除去() {
        let content = "---\npaths:\n  - \"**/*.rs\"\n---\n\n# ルール本文\n内容";
        let body = strip_frontmatter(content);
        assert_eq!(body, "# ルール本文\n内容");
    }

    #[test]
    fn test_strip_frontmatter_フロントマターがない場合はそのまま() {
        let content = "# ルール本文\n内容";
        let body = strip_frontmatter(content);
        assert_eq!(body, "# ルール本文\n内容");
    }

    // --- glob マッチ（compile_glob 使用） ---

    #[test]
    fn test_glob_recursive_wildcard_が深いパスにマッチ() {
        let matcher = compile_glob("**/*.rs").unwrap();
        assert!(matcher.is_match("backend/apps/bff/src/main.rs"));
        assert!(matcher.is_match("src/lib.rs"));
        assert!(!matcher.is_match("README.md"));
    }

    #[test]
    fn test_glob_single_wildcard_がディレクトリ区切りを超えない() {
        let matcher = compile_glob("*.rs").unwrap();
        assert!(matcher.is_match("main.rs"));
        assert!(!matcher.is_match("src/main.rs"));
    }

    // --- match_rules ---

    /// テスト用の一時ディレクトリにルールファイルを作成するヘルパー
    fn create_test_rules(dir: &Path, rules: &[(&str, &str)]) {
        fs::create_dir_all(dir).unwrap();
        for (name, content) in rules {
            let path = dir.join(name);
            let mut file = fs::File::create(&path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }
    }

    #[test]
    fn test_match_rules_マッチングが動作する() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join("rules");
        create_test_rules(
            &rules_dir,
            &[(
                "rust.md",
                "---\npaths:\n  - \"**/*.rs\"\n---\n# Rust ルール\n内容\n",
            )],
        );

        let changed = vec!["backend/src/main.rs".to_string()];
        let matched = match_rules(&changed, &rules_dir);
        assert_eq!(matched.len(), 1);
        assert!(matched[0].path.ends_with("rust.md"));
        assert_eq!(matched[0].body, "# Rust ルール\n内容\n");
    }

    #[test]
    fn test_match_rules_ファイル名順でソート() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join("rules");
        create_test_rules(
            &rules_dir,
            &[
                ("z_rule.md", "---\npaths:\n  - \"**/*\"\n---\n# Z ルール\n"),
                ("a_rule.md", "---\npaths:\n  - \"**/*\"\n---\n# A ルール\n"),
            ],
        );

        let changed = vec!["any_file.txt".to_string()];
        let matched = match_rules(&changed, &rules_dir);
        assert_eq!(matched.len(), 2);
        assert!(matched[0].path.ends_with("a_rule.md"));
        assert!(matched[1].path.ends_with("z_rule.md"));
    }

    #[test]
    fn test_match_rules_pathsなしのルールはスキップ() {
        let dir = tempfile::tempdir().unwrap();
        let rules_dir = dir.path().join("rules");
        create_test_rules(
            &rules_dir,
            &[
                (
                    "with_paths.md",
                    "---\npaths:\n  - \"**/*.rs\"\n---\n# ルール\n",
                ),
                ("no_paths.md", "---\ntitle: info\n---\n# 情報\n"),
                ("no_frontmatter.md", "# フロントマターなし\n"),
            ],
        );

        let changed = vec!["src/main.rs".to_string()];
        let matched = match_rules(&changed, &rules_dir);
        assert_eq!(matched.len(), 1);
        assert!(matched[0].path.ends_with("with_paths.md"));
    }
}
