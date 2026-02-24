#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! glob = "0.3"
//! pretty_assertions = "1"
//! ```

// 改善記録が標準フォーマットに準拠しているかバリデーションする。
//
// チェック内容:
// - 「## 分類」セクションが存在する（エラー）
// - 「- カテゴリ: <有効値>」が存在する（エラー）
// - 「- 失敗タイプ: <有効値>」が存在する（エラー）
// - 「- 問題の性質: <有効値>」が存在する（警告のみ — 2026-02-15 導入のため遡及は別 Issue）
//
// 有効値の定義: [改善記録フォーマット](../../process/improvements/README.md)
//
// Usage: rust-script ./scripts/check/improvement-records.rs

use std::str::FromStr;

/// 改善記録のカテゴリ
#[derive(Debug, PartialEq)]
enum Category {
    ReferenceOmission,
    SinglePathVerification,
    ImmediateAction,
    LackOfPerspective,
    ContextCarryover,
    KnowledgeExecutionGap,
}

impl Category {
    fn all_values() -> &'static [&'static str] {
        &[
            "参照漏れ",
            "単一パス検証",
            "即座の対策",
            "視点不足",
            "コンテキスト引きずり",
            "知識-実行乖離",
        ]
    }
}

impl FromStr for Category {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "参照漏れ" => Ok(Self::ReferenceOmission),
            "単一パス検証" => Ok(Self::SinglePathVerification),
            "即座の対策" => Ok(Self::ImmediateAction),
            "視点不足" => Ok(Self::LackOfPerspective),
            "コンテキスト引きずり" => Ok(Self::ContextCarryover),
            "知識-実行乖離" => Ok(Self::KnowledgeExecutionGap),
            _ => Err(format!(
                "カテゴリ '{}' は定義済みカテゴリに含まれません（有効値: {}）",
                s,
                Self::all_values().join("|")
            )),
        }
    }
}

/// 失敗タイプ
#[derive(Debug, PartialEq)]
enum FailureType {
    KnowledgeGap,
    ExecutionGap,
    ProcessGap,
}

impl FailureType {
    fn all_values() -> &'static [&'static str] {
        &["知識ギャップ", "実行ギャップ", "プロセスギャップ"]
    }
}

impl FromStr for FailureType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "知識ギャップ" => Ok(Self::KnowledgeGap),
            "実行ギャップ" => Ok(Self::ExecutionGap),
            "プロセスギャップ" => Ok(Self::ProcessGap),
            _ => Err(format!(
                "失敗タイプ '{}' は定義済み失敗タイプに含まれません（有効値: {}）",
                s,
                Self::all_values().join("|")
            )),
        }
    }
}

/// 問題の性質
#[derive(Debug, PartialEq)]
enum Nature {
    Technical,
    Process,
    Cognitive,
}

impl Nature {
    fn all_values() -> &'static [&'static str] {
        &["技術的", "プロセス的", "思考的"]
    }
}

impl FromStr for Nature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "技術的" => Ok(Self::Technical),
            "プロセス的" => Ok(Self::Process),
            "思考的" => Ok(Self::Cognitive),
            _ => Err(format!(
                "問題の性質 '{}' は定義済み値に含まれません（有効値: {}）",
                s,
                Self::all_values().join("|")
            )),
        }
    }
}

/// バリデーション結果
struct ValidationResult {
    errors: Vec<String>,
    warnings: Vec<String>,
}

/// 行からプレフィックスを除去し、括弧以降を除去し、末尾空白を除去して値を抽出する
///
/// 例: "- カテゴリ: 知識-実行乖離（検証の仕組みは...）" → "知識-実行乖離"
fn extract_value(line: &str, prefix: &str) -> String {
    let value = line.strip_prefix(prefix).unwrap_or(line);
    let value = match value.find(&['（', '('][..]) {
        Some(pos) => &value[..pos],
        None => value,
    };
    value.trim().to_string()
}

/// ファイル内容をバリデーションし、エラーと警告を返す
fn validate_file(file_path: &str, content: &str) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 「## 分類」セクションの存在チェック
    if !content.lines().any(|line| line == "## 分類") {
        errors.push(format!("{file_path}: '## 分類' セクションがありません"));
        return ValidationResult { errors, warnings };
    }

    // カテゴリのチェック
    let category_line = content
        .lines()
        .find(|line| line.starts_with("- カテゴリ: "));
    match category_line {
        None => {
            errors.push(format!(
                "{file_path}: '- カテゴリ: ' が標準フォーマットで記載されていません"
            ));
        }
        Some(line) => {
            let value = extract_value(line, "- カテゴリ: ");
            if let Err(msg) = Category::from_str(&value) {
                errors.push(format!("{file_path}: {msg}"));
            }
        }
    }

    // 失敗タイプのチェック
    let failure_line = content
        .lines()
        .find(|line| line.starts_with("- 失敗タイプ: "));
    match failure_line {
        None => {
            errors.push(format!(
                "{file_path}: '- 失敗タイプ: ' が標準フォーマットで記載されていません"
            ));
        }
        Some(line) => {
            let value = extract_value(line, "- 失敗タイプ: ");
            if let Err(msg) = FailureType::from_str(&value) {
                errors.push(format!("{file_path}: {msg}"));
            }
        }
    }

    // 問題の性質のチェック（警告のみ）
    let nature_line = content
        .lines()
        .find(|line| line.starts_with("- 問題の性質: "));
    match nature_line {
        None => {
            warnings.push(format!("{file_path}: '- 問題の性質: ' が未記載です"));
        }
        Some(line) => {
            let value = extract_value(line, "- 問題の性質: ");
            if let Err(msg) = Nature::from_str(&value) {
                errors.push(format!("{file_path}: {msg}"));
            }
        }
    }

    ValidationResult { errors, warnings }
}

fn run() -> i32 {
    let mut all_errors = Vec::new();
    let mut all_warnings = Vec::new();

    let pattern = "process/improvements/????-??/*.md";
    let paths = glob::glob(pattern).unwrap_or_else(|e| {
        panic!("glob パターンの解析に失敗: {e}");
    });

    for entry in paths {
        let path = entry.unwrap_or_else(|e| {
            panic!("ファイルパスの取得に失敗: {e}");
        });
        let file_path = path.to_string_lossy().to_string();

        // README.md はバリデーション対象外
        if file_path.ends_with("README.md") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("{file_path} の読み込みに失敗: {e}"));

        let result = validate_file(&file_path, &content);
        all_errors.extend(result.errors);
        all_warnings.extend(result.warnings);
    }

    // 警告の表示
    if !all_warnings.is_empty() {
        println!(
            "⚠ 以下の改善記録に '- 問題の性質: ' が未記載です（{} 件）:",
            all_warnings.len()
        );
        for warning in &all_warnings {
            println!("  - {warning}");
        }
    }

    // エラーの表示
    if !all_errors.is_empty() {
        println!("❌ 以下の改善記録が標準フォーマットに準拠していません:");
        for error in &all_errors {
            println!("  - {error}");
        }
        println!();
        println!("標準フォーマット:");
        println!("  ## 分類");
        println!(
            "  - カテゴリ: <参照漏れ|単一パス検証|即座の対策|視点不足|コンテキスト引きずり|知識-実行乖離>"
        );
        println!("  - 失敗タイプ: <知識ギャップ|実行ギャップ|プロセスギャップ>");
        println!("  - 問題の性質: <技術的|プロセス的|思考的>");
        println!();
        println!("詳細: process/improvements/README.md");
        return 1;
    }

    println!("✅ すべての改善記録が標準フォーマットに準拠しています");
    0
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // --- extract_value ---

    #[test]
    fn test_extract_value_プレフィックスと値を正しく分離する() {
        assert_eq!(
            extract_value("- カテゴリ: 参照漏れ", "- カテゴリ: "),
            "参照漏れ"
        );
    }

    #[test]
    fn test_extract_value_全角括弧以降を除去する() {
        assert_eq!(
            extract_value(
                "- カテゴリ: 知識-実行乖離（検証の仕組みは知っていたが実行しなかった）",
                "- カテゴリ: "
            ),
            "知識-実行乖離"
        );
    }

    #[test]
    fn test_extract_value_半角括弧以降を除去する() {
        assert_eq!(
            extract_value("- カテゴリ: 知識-実行乖離(some note)", "- カテゴリ: "),
            "知識-実行乖離"
        );
    }

    #[test]
    fn test_extract_value_括弧がない場合は値全体を返す() {
        assert_eq!(
            extract_value("- カテゴリ: 視点不足", "- カテゴリ: "),
            "視点不足"
        );
    }

    #[test]
    fn test_extract_value_末尾の空白を除去する() {
        assert_eq!(
            extract_value("- カテゴリ: 参照漏れ  ", "- カテゴリ: "),
            "参照漏れ"
        );
    }

    // --- Category::from_str ---

    #[test]
    fn test_category_from_str_有効なカテゴリ値を受け入れる() {
        assert_eq!(
            Category::from_str("参照漏れ"),
            Ok(Category::ReferenceOmission)
        );
        assert_eq!(
            Category::from_str("単一パス検証"),
            Ok(Category::SinglePathVerification)
        );
        assert_eq!(
            Category::from_str("即座の対策"),
            Ok(Category::ImmediateAction)
        );
        assert_eq!(
            Category::from_str("視点不足"),
            Ok(Category::LackOfPerspective)
        );
        assert_eq!(
            Category::from_str("コンテキスト引きずり"),
            Ok(Category::ContextCarryover)
        );
        assert_eq!(
            Category::from_str("知識-実行乖離"),
            Ok(Category::KnowledgeExecutionGap)
        );
    }

    #[test]
    fn test_category_from_str_無効な値でエラーを返す() {
        let result = Category::from_str("不明なカテゴリ");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不明なカテゴリ"));
    }

    // --- FailureType::from_str ---

    #[test]
    fn test_failure_type_from_str_有効な失敗タイプ値を受け入れる() {
        assert_eq!(
            FailureType::from_str("知識ギャップ"),
            Ok(FailureType::KnowledgeGap)
        );
        assert_eq!(
            FailureType::from_str("実行ギャップ"),
            Ok(FailureType::ExecutionGap)
        );
        assert_eq!(
            FailureType::from_str("プロセスギャップ"),
            Ok(FailureType::ProcessGap)
        );
    }

    #[test]
    fn test_failure_type_from_str_無効な値でエラーを返す() {
        let result = FailureType::from_str("不明なタイプ");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不明なタイプ"));
    }

    // --- Nature::from_str ---

    #[test]
    fn test_nature_from_str_有効な問題の性質値を受け入れる() {
        assert_eq!(Nature::from_str("技術的"), Ok(Nature::Technical));
        assert_eq!(Nature::from_str("プロセス的"), Ok(Nature::Process));
        assert_eq!(Nature::from_str("思考的"), Ok(Nature::Cognitive));
    }

    #[test]
    fn test_nature_from_str_無効な値でエラーを返す() {
        let result = Nature::from_str("不明な性質");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("不明な性質"));
    }

    // --- validate_file ---

    #[test]
    fn test_validate_file_正常なファイルでエラーなし() {
        let content = "\
# タイトル

## 分類

- カテゴリ: 参照漏れ
- 失敗タイプ: 知識ギャップ
- 問題の性質: 技術的
";
        let result = validate_file("test.md", content);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert!(
            result.warnings.is_empty(),
            "warnings: {:?}",
            result.warnings
        );
    }

    #[test]
    fn test_validate_file_分類セクションがない場合にエラー() {
        let content = "\
# タイトル

- カテゴリ: 参照漏れ
- 失敗タイプ: 知識ギャップ
";
        let result = validate_file("test.md", content);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("'## 分類' セクションがありません"));
    }

    #[test]
    fn test_validate_file_カテゴリ行がない場合にエラー() {
        let content = "\
## 分類

- 失敗タイプ: 知識ギャップ
- 問題の性質: 技術的
";
        let result = validate_file("test.md", content);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("'- カテゴリ: '"));
    }

    #[test]
    fn test_validate_file_無効なカテゴリ値でエラー() {
        let content = "\
## 分類

- カテゴリ: 不明なカテゴリ
- 失敗タイプ: 知識ギャップ
- 問題の性質: 技術的
";
        let result = validate_file("test.md", content);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("不明なカテゴリ"));
    }

    #[test]
    fn test_validate_file_失敗タイプ行がない場合にエラー() {
        let content = "\
## 分類

- カテゴリ: 参照漏れ
- 問題の性質: 技術的
";
        let result = validate_file("test.md", content);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("'- 失敗タイプ: '"));
    }

    #[test]
    fn test_validate_file_無効な失敗タイプ値でエラー() {
        let content = "\
## 分類

- カテゴリ: 参照漏れ
- 失敗タイプ: 不明なタイプ
- 問題の性質: 技術的
";
        let result = validate_file("test.md", content);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("不明なタイプ"));
    }

    #[test]
    fn test_validate_file_問題の性質が未記載の場合に警告() {
        let content = "\
## 分類

- カテゴリ: 参照漏れ
- 失敗タイプ: 知識ギャップ
";
        let result = validate_file("test.md", content);
        assert!(result.errors.is_empty(), "errors: {:?}", result.errors);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("'- 問題の性質: ' が未記載です"));
    }

    #[test]
    fn test_validate_file_無効な問題の性質値でエラー() {
        let content = "\
## 分類

- カテゴリ: 参照漏れ
- 失敗タイプ: 知識ギャップ
- 問題の性質: 不明な性質
";
        let result = validate_file("test.md", content);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("不明な性質"));
    }
}
