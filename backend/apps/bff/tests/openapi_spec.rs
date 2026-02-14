//! # OpenAPI 仕様スナップショットテスト
//!
//! utoipa から生成される OpenAPI 仕様の整合性をスナップショットで検証する。

use ringiflow_bff::openapi::ApiDoc;
use utoipa::OpenApi;

#[test]
fn test_openapi仕様がパニックせず生成される() {
   let doc = ApiDoc::openapi();
   // パニックしなければ成功
   let _yaml = doc.to_yaml().unwrap();
}

#[test]
fn test_全パスが含まれている() {
   let doc = ApiDoc::openapi();
   let paths: Vec<&str> = doc.paths.paths.keys().map(|k| k.as_str()).collect();

   // 23 パス（29 ハンドラ、同一パスに複数メソッドがあるため 23 パス）
   // /health はインフラ用のため OpenAPI 仕様には含めない
   assert_eq!(paths.len(), 23, "パス数が 23 であること: {paths:?}");

   // 全パスの存在確認
   assert!(paths.contains(&"/api/v1/auth/login"));
   assert!(paths.contains(&"/api/v1/auth/logout"));
   assert!(paths.contains(&"/api/v1/auth/me"));
   assert!(paths.contains(&"/api/v1/auth/csrf"));
   assert!(paths.contains(&"/api/v1/workflow-definitions"));
   assert!(paths.contains(&"/api/v1/workflow-definitions/{id}"));
   assert!(paths.contains(&"/api/v1/workflows"));
   assert!(paths.contains(&"/api/v1/workflows/{display_number}"));
   assert!(paths.contains(&"/api/v1/workflows/{display_number}/submit"));
   assert!(
      paths.contains(&"/api/v1/workflows/{display_number}/steps/{step_display_number}/approve")
   );
   assert!(
      paths.contains(&"/api/v1/workflows/{display_number}/steps/{step_display_number}/reject")
   );
   assert!(
      paths.contains(
         &"/api/v1/workflows/{display_number}/steps/{step_display_number}/request-changes"
      )
   );
   assert!(paths.contains(&"/api/v1/workflows/{display_number}/resubmit"));
   assert!(paths.contains(&"/api/v1/tasks/my"));
   assert!(paths.contains(&"/api/v1/workflows/{display_number}/tasks/{step_display_number}"));
   assert!(paths.contains(&"/api/v1/users"));
   assert!(paths.contains(&"/api/v1/users/{display_number}"));
   assert!(paths.contains(&"/api/v1/users/{display_number}/status"));
   assert!(paths.contains(&"/api/v1/roles"));
   assert!(paths.contains(&"/api/v1/roles/{role_id}"));
   assert!(paths.contains(&"/api/v1/audit-logs"));
   assert!(paths.contains(&"/api/v1/workflows/{display_number}/comments"));
   assert!(paths.contains(&"/api/v1/dashboard/stats"));
}

#[test]
fn test_session_authセキュリティスキームが含まれている() {
   let doc = ApiDoc::openapi();
   let components = doc.components.as_ref().expect("components が存在すること");
   assert!(
      components.security_schemes.contains_key("session_auth"),
      "session_auth セキュリティスキームが存在すること"
   );
}

#[test]
fn test_全タグが含まれている() {
   let doc = ApiDoc::openapi();
   let tags: Vec<&str> = doc
      .tags
      .as_ref()
      .expect("tags が存在すること")
      .iter()
      .map(|t| t.name.as_str())
      .collect();

   assert!(tags.contains(&"auth"));
   assert!(tags.contains(&"workflows"));
   assert!(tags.contains(&"tasks"));
   assert!(tags.contains(&"users"));
   assert!(tags.contains(&"roles"));
   assert!(tags.contains(&"audit-logs"));
   assert!(tags.contains(&"dashboard"));
}

#[test]
fn test_problem_detailsスキーマが登録されている() {
   let doc = ApiDoc::openapi();
   let components = doc.components.as_ref().expect("components が存在すること");
   assert!(
      components.schemas.contains_key("ProblemDetails"),
      "ProblemDetails スキーマが存在すること: {:?}",
      components.schemas.keys().collect::<Vec<_>>()
   );
}

#[test]
fn test_openapi_json全体のスナップショット() {
   let doc = ApiDoc::openapi();
   let json = serde_json::to_string_pretty(&doc).unwrap();
   insta::assert_snapshot!("openapi_spec", json);
}
