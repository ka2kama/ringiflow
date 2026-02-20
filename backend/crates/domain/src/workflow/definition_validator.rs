//! # ワークフロー定義バリデーション
//!
//! 定義 JSON の構造的整合性を検証する。
//! 公開時に自動実行され、バリデーション API からも呼び出される。

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use serde_json::Value as JsonValue;

/// バリデーション結果
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub valid:  bool,
    pub errors: Vec<ValidationError>,
}

/// バリデーションエラー
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ValidationError {
    pub code:    String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
}

impl ValidationError {
    fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code:    code.into(),
            message: message.into(),
            step_id: None,
        }
    }

    fn with_step_id(
        code: impl Into<String>,
        message: impl Into<String>,
        step_id: impl Into<String>,
    ) -> Self {
        Self {
            code:    code.into(),
            message: message.into(),
            step_id: Some(step_id.into()),
        }
    }
}

/// ワークフロー定義 JSON をバリデーションする
///
/// 10 のルールを順に検証し、すべてのエラーを収集して返す。
pub fn validate_definition(definition: &JsonValue) -> ValidationResult {
    let mut errors = Vec::new();

    validate_start_step(definition, &mut errors);
    validate_end_steps(definition, &mut errors);
    validate_approval_steps(definition, &mut errors);
    validate_step_ids_unique(definition, &mut errors);
    validate_transition_references(definition, &mut errors);
    validate_no_orphans(definition, &mut errors);
    validate_no_cycles(definition, &mut errors);
    validate_approval_transitions(definition, &mut errors);
    validate_form_fields(definition, &mut errors);

    ValidationResult {
        valid: errors.is_empty(),
        errors,
    }
}

/// steps 配列を安全に取得するヘルパー
fn get_steps(definition: &JsonValue) -> Option<&Vec<JsonValue>> {
    definition.get("steps").and_then(|v| v.as_array())
}

/// transitions 配列を安全に取得するヘルパー
fn get_transitions(definition: &JsonValue) -> Option<&Vec<JsonValue>> {
    definition.get("transitions").and_then(|v| v.as_array())
}

/// ステップの type を取得するヘルパー
fn step_type(step: &JsonValue) -> Option<&str> {
    step.get("type").and_then(|v| v.as_str())
}

/// ステップの id を取得するヘルパー
fn step_id(step: &JsonValue) -> Option<&str> {
    step.get("id").and_then(|v| v.as_str())
}

// --- バリデーションルール ---

/// ルール 1, 2: start ステップが正確に 1 つ
fn validate_start_step(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let start_count = steps
        .iter()
        .filter(|s| step_type(s) == Some("start"))
        .count();
    match start_count {
        0 => errors.push(ValidationError::new(
            "missing_start_step",
            "開始ステップが必要です",
        )),
        1 => {}
        _ => errors.push(ValidationError::new(
            "multiple_start_steps",
            "開始ステップは 1 つのみ許可されます",
        )),
    }
}

/// ルール 3: end ステップが 1 つ以上
fn validate_end_steps(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let end_count = steps.iter().filter(|s| step_type(s) == Some("end")).count();
    if end_count == 0 {
        errors.push(ValidationError::new(
            "missing_end_step",
            "終了ステップが必要です",
        ));
    }
}

/// ルール 4: approval ステップが 1 つ以上
fn validate_approval_steps(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let approval_count = steps
        .iter()
        .filter(|s| step_type(s) == Some("approval"))
        .count();
    if approval_count == 0 {
        errors.push(ValidationError::new(
            "missing_approval_step",
            "承認ステップが必要です",
        ));
    }
}

/// ルール 8: ステップ ID の重複チェック
fn validate_step_ids_unique(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let mut seen = HashSet::new();
    for step in steps {
        if let Some(id) = step_id(step) {
            if !seen.insert(id) {
                errors.push(ValidationError::with_step_id(
                    "duplicate_step_id",
                    format!("ステップ ID '{}' が重複しています", id),
                    id,
                ));
            }
        }
    }
}

/// ルール 9: 遷移が有効なステップ ID を参照しているか
fn validate_transition_references(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let Some(transitions) = get_transitions(definition) else {
        return;
    };

    let step_ids: HashSet<&str> = steps.iter().filter_map(step_id).collect();

    for transition in transitions {
        if let Some(from) = transition.get("from").and_then(|v| v.as_str()) {
            if !step_ids.contains(from) {
                errors.push(ValidationError::with_step_id(
                    "invalid_transition_ref",
                    format!("遷移元 '{}' は存在しないステップです", from),
                    from,
                ));
            }
        }
        if let Some(to) = transition.get("to").and_then(|v| v.as_str()) {
            if !step_ids.contains(to) {
                errors.push(ValidationError::with_step_id(
                    "invalid_transition_ref",
                    format!("遷移先 '{}' は存在しないステップです", to),
                    to,
                ));
            }
        }
    }
}

/// ルール 5: 孤立ステップなし（start 以外はすべて到達可能）
fn validate_no_orphans(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let Some(transitions) = get_transitions(definition) else {
        return;
    };

    let step_ids: HashSet<&str> = steps.iter().filter_map(step_id).collect();

    // 遷移で参照されているステップ ID を収集
    let mut connected: HashSet<&str> = HashSet::new();
    for transition in transitions {
        if let Some(to) = transition.get("to").and_then(|v| v.as_str()) {
            connected.insert(to);
        }
        if let Some(from) = transition.get("from").and_then(|v| v.as_str()) {
            connected.insert(from);
        }
    }

    // start ステップは遷移元として接続されていれば OK
    for step in steps {
        let Some(id) = step_id(step) else {
            continue;
        };
        // start ステップは孤立チェック対象外（遷移の起点として from に存在していれば OK）
        if step_type(step) == Some("start") {
            continue;
        }
        if !connected.contains(id) && step_ids.contains(id) {
            errors.push(ValidationError::with_step_id(
                "orphaned_step",
                format!("ステップ '{}' が接続されていません", id),
                id,
            ));
        }
    }
}

/// ルール 6: 循環（サイクル）がないこと（DAG 検証）
///
/// DFS でサイクルを検出する。白・灰・黒の3色アルゴリズムを使用。
fn validate_no_cycles(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let Some(transitions) = get_transitions(definition) else {
        return;
    };

    // 隣接リストを構築
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in steps {
        if let Some(id) = step_id(step) {
            adjacency.entry(id).or_default();
        }
    }
    for transition in transitions {
        let from = transition.get("from").and_then(|v| v.as_str());
        let to = transition.get("to").and_then(|v| v.as_str());
        if let (Some(from), Some(to)) = (from, to) {
            adjacency.entry(from).or_default().push(to);
        }
    }

    // 3色 DFS（White=未訪問, Gray=処理中, Black=完了）
    #[derive(Clone, Copy, PartialEq)]
    enum Color {
        White,
        Gray,
        Black,
    }

    let mut colors: HashMap<&str, Color> = adjacency.keys().map(|&k| (k, Color::White)).collect();
    let mut has_cycle = false;

    fn dfs<'a>(
        node: &'a str,
        adjacency: &HashMap<&'a str, Vec<&'a str>>,
        colors: &mut HashMap<&'a str, Color>,
        has_cycle: &mut bool,
    ) {
        if *has_cycle {
            return;
        }
        colors.insert(node, Color::Gray);
        if let Some(neighbors) = adjacency.get(node) {
            for &next in neighbors {
                match colors.get(next) {
                    Some(Color::Gray) => {
                        *has_cycle = true;
                        return;
                    }
                    Some(Color::White) => dfs(next, adjacency, colors, has_cycle),
                    _ => {}
                }
            }
        }
        colors.insert(node, Color::Black);
    }

    let nodes: Vec<&str> = adjacency.keys().copied().collect();
    for node in nodes {
        if colors.get(node) == Some(&Color::White) {
            dfs(node, &adjacency, &mut colors, &mut has_cycle);
        }
    }

    if has_cycle {
        errors.push(ValidationError::new(
            "cycle_detected",
            "ワークフローに循環が検出されました",
        ));
    }
}

/// ルール 7: approval ステップから approve/reject 両方の遷移が存在
fn validate_approval_transitions(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(steps) = get_steps(definition) else {
        return;
    };
    let Some(transitions) = get_transitions(definition) else {
        return;
    };

    let approval_ids: Vec<&str> = steps
        .iter()
        .filter(|s| step_type(s) == Some("approval"))
        .filter_map(step_id)
        .collect();

    for approval_id in approval_ids {
        let triggers: HashSet<&str> = transitions
            .iter()
            .filter(|t| t.get("from").and_then(|v| v.as_str()) == Some(approval_id))
            .filter_map(|t| t.get("trigger").and_then(|v| v.as_str()))
            .collect();

        if !triggers.contains("approve") || !triggers.contains("reject") {
            errors.push(ValidationError::with_step_id(
                "missing_approval_transition",
                format!(
                    "承認ステップ '{}' に approve/reject 両方の遷移が必要です",
                    approval_id
                ),
                approval_id,
            ));
        }
    }
}

/// ルール 10: フォームフィールドの整合性チェック
fn validate_form_fields(definition: &JsonValue, errors: &mut Vec<ValidationError>) {
    let Some(fields) = definition
        .get("form")
        .and_then(|f| f.get("fields"))
        .and_then(|f| f.as_array())
    else {
        // form がない場合はバリデーションスキップ（form は任意）
        return;
    };

    let valid_types = ["text", "textarea", "number", "select", "date"];
    let mut seen_ids = HashSet::new();

    for field in fields {
        let field_id = field.get("id").and_then(|v| v.as_str());
        let field_type = field.get("type").and_then(|v| v.as_str());

        // id の存在チェック
        let Some(id) = field_id else {
            errors.push(ValidationError::new(
                "invalid_form_field",
                "フォームフィールドに id が必要です",
            ));
            continue;
        };

        // id の重複チェック
        if !seen_ids.insert(id) {
            errors.push(ValidationError::new(
                "invalid_form_field",
                format!("フォームフィールド ID '{}' が重複しています", id),
            ));
        }

        // type の存在と有効性チェック
        match field_type {
            None => {
                errors.push(ValidationError::new(
                    "invalid_form_field",
                    format!("フォームフィールド '{}' に type が必要です", id),
                ));
            }
            Some(t) if !valid_types.contains(&t) => {
                errors.push(ValidationError::new(
                    "invalid_form_field",
                    format!("フォームフィールド '{}' の type '{}' は無効です", id, t),
                ));
            }
            _ => {}
        }

        // label の存在チェック
        if field.get("label").and_then(|v| v.as_str()).is_none() {
            errors.push(ValidationError::new(
                "invalid_form_field",
                format!("フォームフィールド '{}' に label が必要です", id),
            ));
        }

        // select の options チェック
        if field_type == Some("select") {
            let has_options = field
                .get("options")
                .and_then(|v| v.as_array())
                .is_some_and(|arr| !arr.is_empty());
            if !has_options {
                errors.push(ValidationError::new(
                    "invalid_form_field",
                    format!("フォームフィールド '{}' (select) に options が必要です", id),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use serde_json::json;

    use super::*;

    /// テスト用の有効な定義 JSON を生成するヘルパー
    fn valid_definition() -> JsonValue {
        json!({
            "form": {
                "fields": [
                    {"id": "title", "type": "text", "label": "件名", "required": true},
                    {"id": "amount", "type": "number", "label": "金額", "required": true}
                ]
            },
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "上長承認"},
                {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        })
    }

    #[test]
    fn test_有効な定義でバリデーション成功() {
        let result = validate_definition(&valid_definition());

        assert!(result.valid, "errors: {:?}", result.errors);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_複数エラーが同時に返される() {
        let definition = json!({
            "steps": [],
            "transitions": []
        });

        let result = validate_definition(&definition);

        assert!(!result.valid);
        let codes: Vec<&str> = result.errors.iter().map(|e| e.code.as_str()).collect();
        assert!(codes.contains(&"missing_start_step"));
        assert!(codes.contains(&"missing_end_step"));
        assert!(codes.contains(&"missing_approval_step"));
    }

    // --- ルール 1: missing_start_step ---

    #[test]
    fn test_startステップがない場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end", "type": "end", "name": "完了"}
            ],
            "transitions": [
                {"from": "approval_1", "to": "end", "trigger": "approve"},
                {"from": "approval_1", "to": "end", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "missing_start_step"));
    }

    #[test]
    fn test_startステップが1つなら正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "missing_start_step"));
        assert!(!has_error(&result, "multiple_start_steps"));
    }

    // --- ルール 2: multiple_start_steps ---

    #[test]
    fn test_startステップが複数ある場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start1", "type": "start", "name": "開始1"},
                {"id": "start2", "type": "start", "name": "開始2"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start1", "to": "approval_1"},
                {"from": "start2", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "multiple_start_steps"));
    }

    // --- ルール 3: missing_end_step ---

    #[test]
    fn test_endステップがない場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "missing_end_step"));
    }

    #[test]
    fn test_endステップが存在すれば正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "missing_end_step"));
    }

    // --- ルール 4: missing_approval_step ---

    #[test]
    fn test_approvalステップがない場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "end", "type": "end", "name": "完了"}
            ],
            "transitions": [
                {"from": "start", "to": "end"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "missing_approval_step"));
    }

    #[test]
    fn test_approvalステップが存在すれば正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "missing_approval_step"));
    }

    // --- ルール 5: orphaned_step ---

    #[test]
    fn test_孤立ステップがある場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "orphan", "type": "approval", "name": "孤立"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "orphaned_step"));
        let orphan_error = result
            .errors
            .iter()
            .find(|e| e.code == "orphaned_step")
            .unwrap();
        assert_eq!(orphan_error.step_id.as_deref(), Some("orphan"));
    }

    #[test]
    fn test_全ステップが接続されていれば正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "orphaned_step"));
    }

    // --- ルール 6: cycle_detected ---

    #[test]
    fn test_循環がある場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認1"},
                {"id": "approval_2", "type": "approval", "name": "承認2"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "approval_2", "trigger": "approve"},
                {"from": "approval_2", "to": "approval_1", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"},
                {"from": "approval_2", "to": "end_approved", "trigger": "reject"},
                {"from": "approval_2", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "cycle_detected"));
    }

    #[test]
    fn test_循環がなければ正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "cycle_detected"));
    }

    // --- ルール 7: missing_approval_transition ---

    #[test]
    fn test_approvalにapprove遷移がない場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "missing_approval_transition"));
    }

    #[test]
    fn test_approvalにreject遷移がない場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "missing_approval_transition"));
    }

    #[test]
    fn test_approvalに両方の遷移があれば正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "missing_approval_transition"));
    }

    // --- ルール 8: duplicate_step_id ---

    #[test]
    fn test_ステップIDが重複している場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認1"},
                {"id": "approval_1", "type": "approval", "name": "承認2"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "duplicate_step_id"));
    }

    #[test]
    fn test_ステップIDが一意なら正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "duplicate_step_id"));
    }

    // --- ルール 9: invalid_transition_ref ---

    #[test]
    fn test_遷移が存在しないステップを参照している場合エラー() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"},
                {"from": "approval_1", "to": "nonexistent"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "invalid_transition_ref"));
    }

    #[test]
    fn test_全遷移が有効なステップを参照していれば正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "invalid_transition_ref"));
    }

    // --- ルール 10: invalid_form_field ---

    #[test]
    fn test_フォームフィールドにidがない場合エラー() {
        let definition = json!({
            "form": {
                "fields": [{"type": "text", "label": "名前", "required": true}]
            },
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "invalid_form_field"));
    }

    #[test]
    fn test_フォームフィールドのtypeが無効な場合エラー() {
        let definition = json!({
            "form": {
                "fields": [{"id": "f1", "type": "invalid_type", "label": "名前", "required": true}]
            },
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "invalid_form_field"));
    }

    #[test]
    fn test_selectフィールドにoptionsがない場合エラー() {
        let definition = json!({
            "form": {
                "fields": [{"id": "category", "type": "select", "label": "分類", "required": true}]
            },
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "invalid_form_field"));
    }

    #[test]
    fn test_フォームフィールドIDが重複している場合エラー() {
        let definition = json!({
            "form": {
                "fields": [
                    {"id": "name", "type": "text", "label": "名前1", "required": true},
                    {"id": "name", "type": "text", "label": "名前2", "required": true}
                ]
            },
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "invalid_form_field"));
    }

    #[test]
    fn test_フォームフィールドにlabelがない場合エラー() {
        let definition = json!({
            "form": {
                "fields": [{"id": "f1", "type": "text", "required": true}]
            },
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(has_error(&result, "invalid_form_field"));
    }

    #[test]
    fn test_有効なフォームフィールドなら正常() {
        let result = validate_definition(&valid_definition());

        assert!(!has_error(&result, "invalid_form_field"));
    }

    #[test]
    fn test_formがない定義でもバリデーション成功() {
        let definition = json!({
            "steps": [
                {"id": "start", "type": "start", "name": "開始"},
                {"id": "approval_1", "type": "approval", "name": "承認"},
                {"id": "end_approved", "type": "end", "name": "完了", "status": "approved"},
                {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
            ],
            "transitions": [
                {"from": "start", "to": "approval_1"},
                {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
                {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
            ]
        });

        let result = validate_definition(&definition);

        assert!(result.valid, "errors: {:?}", result.errors);
    }

    // --- テストヘルパー ---

    fn has_error(result: &ValidationResult, code: &str) -> bool {
        result.errors.iter().any(|e| e.code == code)
    }
}
