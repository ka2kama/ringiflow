#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! regex = "1"
//! pretty_assertions = "1"
//! ```

// Story Issue の親 Epic タスクリストを自動更新するスクリプト
//
// Story Issue の body から親 Epic 番号を検出し、Epic のタスクリストで
// 該当 Story のチェックボックスを [x] に更新する。
//
// Usage: rust-script ./scripts/issue/sync-epic.rs ISSUE_NUMBER

use regex::Regex;

/// Story Issue の body から親 Epic 番号を抽出する
///
/// 2つのフォーマットをサポート:
/// - フォーマット1（インライン）: `Epic: #123`
/// - フォーマット2（テンプレートレンダリング）: `### Epic\n\n#123`
fn extract_epic_number(issue_body: &str) -> Option<u32> {
    // フォーマット1: インライン（手動記載）
    let pattern1 = Regex::new(r"Epic:\s*#(\d+)").unwrap();
    if let Some(caps) = pattern1.captures(issue_body) {
        return caps[1].parse().ok();
    }

    // フォーマット2: テンプレートレンダリング（feature.yaml の type: input）
    // Rust regex の \s は \n を含むため、\s+ で改行を含むマッチが可能
    let pattern2 = Regex::new(r"###\s+Epic\s+#(\d+)").unwrap();
    if let Some(caps) = pattern2.captures(issue_body) {
        return caps[1].parse().ok();
    }

    None
}

/// Epic タスクリストで該当 Issue が既に [x] かチェックする（冪等性）
fn check_already_updated(epic_body: &str, issue_number: u32) -> bool {
    let pattern = Regex::new(&format!(r"(?m)^- \[x\] .*#{}(?:\D|$)", issue_number)).unwrap();
    pattern.is_match(epic_body)
}

/// Epic タスクリストに該当 Issue の未チェック行が存在するかチェックする
fn check_exists_unchecked(epic_body: &str, issue_number: u32) -> bool {
    let pattern = Regex::new(&format!(r"(?m)^- \[ \] .*#{}(?:\D|$)", issue_number)).unwrap();
    pattern.is_match(epic_body)
}

/// Epic タスクリストのチェックボックスを更新する（[ ] → [x]）
fn update_checkbox(epic_body: &str, issue_number: u32) -> String {
    let pattern = Regex::new(&format!(r"(?m)^- \[ \] .*#{}(?:\D|$)", issue_number)).unwrap();
    epic_body
        .lines()
        .map(|line| {
            if pattern.is_match(line) {
                line.replacen("[ ]", "[x]", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// gh issue view --json body で Issue body を取得する
fn gh_issue_view_body(issue_number: u32) -> Result<String, String> {
    let output = std::process::Command::new("gh")
        .args([
            "issue",
            "view",
            &issue_number.to_string(),
            "--json",
            "body",
            "--jq",
            ".body",
        ])
        .output()
        .map_err(|e| format!("gh コマンドの実行に失敗: {e}"))?;

    if !output.status.success() {
        return Err("gh issue view が失敗しました".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if body.is_empty() {
        return Err("body が空です".to_string());
    }
    Ok(body)
}

/// gh issue edit --body で Issue body を更新する
fn gh_issue_edit_body(issue_number: u32, body: &str) -> Result<(), String> {
    let output = std::process::Command::new("gh")
        .args(["issue", "edit", &issue_number.to_string(), "--body", body])
        .output()
        .map_err(|e| format!("gh コマンドの実行に失敗: {e}"))?;

    if !output.status.success() {
        return Err("gh issue edit が失敗しました".to_string());
    }
    Ok(())
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("使い方: sync-epic.rs ISSUE_NUMBER");
        return 1;
    }

    let issue_number: u32 = match args[1].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("エラー: ISSUE_NUMBER は数値で指定してください");
            return 1;
        }
    };

    // 1. Story Issue の body を取得
    let issue_body = match gh_issue_view_body(issue_number) {
        Ok(body) => body,
        Err(_) => {
            eprintln!("エラー: Issue #{issue_number} が見つからないか、body が空です");
            return 1;
        }
    };

    // 2. Epic 番号を抽出
    let epic_number = match extract_epic_number(&issue_body) {
        Some(n) => n,
        None => {
            println!("ℹ️ Issue #{issue_number} に親 Epic が設定されていません（スキップ）");
            return 0;
        }
    };

    println!("Issue #{issue_number} → Epic #{epic_number}");

    // 3. Epic の body を取得
    let epic_body = match gh_issue_view_body(epic_number) {
        Ok(body) => body,
        Err(_) => {
            eprintln!("エラー: Epic #{epic_number} が見つからないか、body が空です");
            return 1;
        }
    };

    // 4. 冪等性チェック
    if check_already_updated(&epic_body, issue_number) {
        println!("✓ Epic #{epic_number} のタスクリストは既に更新済みです");
        return 0;
    }

    // 5. 存在チェック
    if !check_exists_unchecked(&epic_body, issue_number) {
        eprintln!("⚠️ Epic #{epic_number} のタスクリストに #{issue_number} が見つかりません");
        return 0;
    }

    // 6. チェックボックス更新
    let updated_body = update_checkbox(&epic_body, issue_number);

    // 7. Epic を更新
    if let Err(e) = gh_issue_edit_body(epic_number, &updated_body) {
        eprintln!("エラー: Epic #{epic_number} の更新に失敗しました: {e}");
        return 1;
    }

    println!("✓ Epic #{epic_number} のタスクリストを更新しました（#{issue_number} → [x]）");
    0
}

fn main() {
    std::process::exit(run());
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // --- extract_epic_number ---

    #[test]
    fn test_extract_epic_number_インライン形式から抽出する() {
        let body = "## Issue\n\nEpic: #841\n\nRelated to #833";
        assert_eq!(extract_epic_number(body), Some(841));
    }

    #[test]
    fn test_extract_epic_number_スペースなしでも抽出できる() {
        let body = "Epic:#123";
        assert_eq!(extract_epic_number(body), Some(123));
    }

    #[test]
    fn test_extract_epic_number_複数スペースでも抽出できる() {
        let body = "Epic:  #456";
        assert_eq!(extract_epic_number(body), Some(456));
    }

    #[test]
    fn test_extract_epic_number_テンプレート形式から抽出する() {
        let body = "### Epic\n\n#789\n\n### 関連ドキュメント";
        assert_eq!(extract_epic_number(body), Some(789));
    }

    #[test]
    fn test_extract_epic_number_epic未設定でnoneを返す() {
        let body = "## Issue\n\nこれは Epic に属さない Issue です";
        assert_eq!(extract_epic_number(body), None);
    }

    #[test]
    fn test_extract_epic_number_本文中のissue番号だけではマッチしない() {
        let body = "## Issue\n\nRelated to #123\n\nSee also #456";
        assert_eq!(extract_epic_number(body), None);
    }

    // --- check_already_updated ---

    #[test]
    fn test_check_already_updated_チェック済み行がある場合にtrueを返す() {
        let body = "## タスク\n\n- [x] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        assert!(check_already_updated(body, 123));
    }

    #[test]
    fn test_check_already_updated_未チェック行のみの場合にfalseを返す() {
        let body = "## タスク\n\n- [ ] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        assert!(!check_already_updated(body, 123));
    }

    #[test]
    fn test_check_already_updated_issue番号が含まれない場合にfalseを返す() {
        let body = "## タスク\n\n- [x] Story 1: 説明 #456\n- [ ] Story 2: 説明 #789";
        assert!(!check_already_updated(body, 123));
    }

    #[test]
    fn test_check_already_updated_部分一致を防ぐ_短い番号() {
        let body = "## タスク\n\n- [x] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        // #12 で検索しても #123 にはマッチしない
        assert!(!check_already_updated(body, 12));
    }

    #[test]
    fn test_check_already_updated_部分一致を防ぐ_長い番号() {
        let body = "## タスク\n\n- [x] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        // #1234 で検索しても #123 にはマッチしない
        assert!(!check_already_updated(body, 1234));
    }

    #[test]
    fn test_check_already_updated_行末のissue番号にマッチする() {
        let body = "- [x] Story 1: 説明 #123";
        assert!(check_already_updated(body, 123));
    }

    // --- check_exists_unchecked ---

    #[test]
    fn test_check_exists_unchecked_未チェック行がある場合にtrueを返す() {
        let body = "## タスク\n\n- [ ] Story 1: 説明 #123\n- [x] Story 2: 説明 #456";
        assert!(check_exists_unchecked(body, 123));
    }

    #[test]
    fn test_check_exists_unchecked_チェック済み行のみの場合にfalseを返す() {
        let body = "## タスク\n\n- [x] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        assert!(!check_exists_unchecked(body, 123));
    }

    #[test]
    fn test_check_exists_unchecked_issue番号が含まれない場合にfalseを返す() {
        let body = "## タスク\n\n- [ ] Story 1: 説明 #456\n- [ ] Story 2: 説明 #789";
        assert!(!check_exists_unchecked(body, 123));
    }

    #[test]
    fn test_check_exists_unchecked_部分一致を防ぐ() {
        let body = "## タスク\n\n- [ ] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        // #12 で検索しても #123 にはマッチしない
        assert!(!check_exists_unchecked(body, 12));
    }

    // --- update_checkbox ---

    #[test]
    fn test_update_checkbox_該当行のチェックボックスを更新する() {
        let body = "- [ ] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456";
        let result = update_checkbox(body, 123);
        assert_eq!(result, "- [x] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456");
    }

    #[test]
    fn test_update_checkbox_他の行はそのまま保持する() {
        let body = "## タスク\n\n- [ ] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456\n\n## 備考";
        let result = update_checkbox(body, 123);
        assert_eq!(
            result,
            "## タスク\n\n- [x] Story 1: 説明 #123\n- [ ] Story 2: 説明 #456\n\n## 備考"
        );
    }

    #[test]
    fn test_update_checkbox_部分一致を防ぐ() {
        let body = "- [ ] Story 1: 説明 #123\n- [ ] Story 2: 説明 #1234";
        let result = update_checkbox(body, 12);
        // #12 の行は存在しないため、どちらも変更されない
        assert_eq!(
            result,
            "- [ ] Story 1: 説明 #123\n- [ ] Story 2: 説明 #1234"
        );
    }

    #[test]
    fn test_update_checkbox_複数行がある場合に該当行のみ更新する() {
        let body = "- [ ] #834 instrumentation.sh\n- [ ] #835 improvement-records.sh\n- [ ] #836 impl-docs.sh\n- [ ] #837 sync-epic.sh";
        let result = update_checkbox(body, 835);
        assert_eq!(
            result,
            "- [ ] #834 instrumentation.sh\n- [x] #835 improvement-records.sh\n- [ ] #836 impl-docs.sh\n- [ ] #837 sync-epic.sh"
        );
    }
}
