#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! globset = "0.4"
//! tempfile = "3"
//! pretty_assertions = "1"
//! ```

// PR の変更ファイルにマッチする .claude/rules/ のルールを特定し、内容を出力する。
//
// 使い方:
//     rust-script match-rules.rs <changed-files.txt>
//     rust-script match-rules.rs --groups N <changed-files.txt>
//
// 入力: 変更ファイル一覧（1 行 1 パス）
// 出力:
//   通常モード: マッチしたルールの名前リスト + 各ルールの本文（フロントマター除去済み）
//   グループモード: ルールを N グループに分割して出力（各グループは <!-- group:N --> で区切り）

use globset::{GlobBuilder, GlobMatcher};
use std::fs;
use std::path::Path;

/// glob パターンをコンパイルする。
///
/// `literal_separator(true)` により `*` がパス区切りを超えない
/// （Python 版の `[^/]*` と同等の挙動）。
fn compile_glob(pattern: &str) -> Option<GlobMatcher> {
    match GlobBuilder::new(pattern).literal_separator(true).build() {
        Ok(g) => Some(g.compile_matcher()),
        Err(e) => {
            eprintln!("警告: glob パターンのコンパイルに失敗: {pattern}: {e}");
            None
        }
    }
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
    // end_offset が content.len() を超える場合に備えて min で防御
    content[end_offset.min(content.len())..].trim_start_matches('\n')
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

/// マッチしたルールを N グループに分割する（LPT アルゴリズム）。
///
/// body サイズの大きいルールから順に、現在の合計サイズが最小のグループに割り当てる。
fn split_into_groups(rules: Vec<MatchedRule>, n: usize) -> Vec<Vec<MatchedRule>> {
    let mut groups: Vec<Vec<MatchedRule>> = (0..n).map(|_| Vec::new()).collect();
    let mut group_sizes: Vec<usize> = vec![0; n];

    // サイズ降順でソート（LPT: Longest Processing Time first）
    let mut sorted: Vec<_> = rules.into_iter().collect();
    sorted.sort_by(|a, b| b.body.len().cmp(&a.body.len()));

    for rule in sorted {
        // 最小サイズのグループを選択
        let min_idx = group_sizes
            .iter()
            .enumerate()
            .min_by_key(|&(_, &size)| size)
            .map(|(idx, _)| idx)
            .unwrap_or(0);

        group_sizes[min_idx] += rule.body.len();
        groups[min_idx].push(rule);
    }

    groups
}

/// ルール一覧のサマリーと本文を出力する。
fn format_rules(rules: &[MatchedRule]) -> String {
    let mut out = String::new();

    out.push_str(&format!("マッチしたルール: {} 件\n\n", rules.len()));
    for rule in rules {
        out.push_str(&format!("- `{}`\n", rule.path));
    }
    out.push('\n');

    for rule in rules {
        out.push_str(&format!("### {}\n\n", rule.path));
        out.push_str(&rule.body);
        out.push_str("\n\n");
    }

    out
}

/// CLI 引数をパースする。
struct CliArgs {
    groups: Option<usize>,
    changed_files_path: String,
}

fn parse_args() -> CliArgs {
    let args: Vec<String> = std::env::args().collect();
    let mut groups = None;
    let mut positional = Vec::new();

    let mut i = 1;
    while i < args.len() {
        if args[i] == "--groups" {
            i += 1;
            if i >= args.len() {
                eprintln!("エラー: --groups にはグループ数を指定してください");
                std::process::exit(1);
            }
            groups = Some(args[i].parse::<usize>().unwrap_or_else(|_| {
                eprintln!(
                    "エラー: --groups の値は正の整数を指定してください: {}",
                    args[i]
                );
                std::process::exit(1);
            }));
            if groups == Some(0) {
                eprintln!("エラー: --groups は 1 以上を指定してください");
                std::process::exit(1);
            }
        } else {
            positional.push(args[i].clone());
        }
        i += 1;
    }

    if positional.is_empty() {
        eprintln!("Usage: match-rules.rs [--groups N] <changed-files.txt>");
        std::process::exit(1);
    }

    CliArgs {
        groups,
        changed_files_path: positional[0].clone(),
    }
}

fn main() {
    let cli = parse_args();
    let rules_dir = Path::new(".claude/rules");

    let content = match fs::read_to_string(&cli.changed_files_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("エラー: {} の読み込みに失敗: {e}", cli.changed_files_path);
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

    match cli.groups {
        Some(n) => {
            // グループ分割モード
            let groups = split_into_groups(matched, n);
            for (i, group) in groups.iter().enumerate() {
                println!("<!-- group:{} -->", i + 1);
                if group.is_empty() {
                    println!("<!-- no-matching-rules -->");
                } else {
                    print!("{}", format_rules(group));
                }
            }
        }
        None => {
            // 通常モード（後方互換）
            print!("{}", format_rules(&matched));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
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

    // --- split_into_groups ---

    /// テスト用ヘルパー: MatchedRule を生成する
    fn make_rule(path: &str, body_size: usize) -> MatchedRule {
        MatchedRule {
            path: path.to_string(),
            body: "x".repeat(body_size),
        }
    }

    #[test]
    fn test_split_into_groups_2グループに均等分割() {
        // 100, 80, 60, 40 → グループ1: [100, 40]=140, グループ2: [80, 60]=140
        let rules = vec![
            make_rule("a.md", 100),
            make_rule("b.md", 80),
            make_rule("c.md", 60),
            make_rule("d.md", 40),
        ];
        let groups = split_into_groups(rules, 2);
        assert_eq!(groups.len(), 2);

        let size0: usize = groups[0].iter().map(|r| r.body.len()).sum();
        let size1: usize = groups[1].iter().map(|r| r.body.len()).sum();
        // LPT: 100→G1, 80→G2, 60→G2, 40→G1 → [140, 140]
        assert_eq!(size0, 140);
        assert_eq!(size1, 140);
    }

    #[test]
    fn test_split_into_groups_1件でグループ1のみ() {
        let rules = vec![make_rule("only.md", 100)];
        let groups = split_into_groups(rules, 2);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].len(), 1);
        assert!(groups[1].is_empty());
    }

    #[test]
    fn test_split_into_groups_0件で両グループ空() {
        let rules: Vec<MatchedRule> = vec![];
        let groups = split_into_groups(rules, 2);
        assert_eq!(groups.len(), 2);
        assert!(groups[0].is_empty());
        assert!(groups[1].is_empty());
    }

    #[test]
    fn test_split_into_groups_lptでサイズバランス() {
        // 200, 150, 100, 50, 30, 20 → LPT:
        // 200→G1(200), 150→G2(150), 100→G2(250), 50→G1(250), 30→G1(280), 20→G2(270)
        let rules = vec![
            make_rule("a.md", 200),
            make_rule("b.md", 150),
            make_rule("c.md", 100),
            make_rule("d.md", 50),
            make_rule("e.md", 30),
            make_rule("f.md", 20),
        ];
        let groups = split_into_groups(rules, 2);
        assert_eq!(groups.len(), 2);

        let size0: usize = groups[0].iter().map(|r| r.body.len()).sum();
        let size1: usize = groups[1].iter().map(|r| r.body.len()).sum();
        // 差が小さいことを確認（LPT の保証: 最適の 4/3 以内）
        let diff = size0.abs_diff(size1);
        let total = size0 + size1;
        assert!(
            diff * 3 <= total,
            "groups too imbalanced: {size0} vs {size1}"
        );
    }
}
