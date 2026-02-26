//! # テンプレートレンダラー
//!
//! tera テンプレートエンジンで通知メールを HTML/plaintext 両形式で生成する。
//!
//! ## 設計方針
//!
//! - **`include_str!` によるコンパイル時埋め込み**: テンプレートはバイナリに埋め込まれる
//! - **件名パターン**: `[RingiFlow] {イベント種別}: {title} {display_id}`
//! - **ワークフロー詳細リンク**: `{base_url}/workflows/{display_id}` をテンプレートに渡す
//!
//! → 詳細設計: `docs/03_詳細設計書/16_通知機能設計.md`

use ringiflow_domain::notification::{EmailMessage, NotificationError, WorkflowNotification};
use tera::{Context, Tera};

/// テンプレートレンダラー
///
/// tera テンプレートエンジンをラップし、`WorkflowNotification` から
/// `EmailMessage` を生成する。
pub struct TemplateRenderer {
    engine: Tera,
}

impl TemplateRenderer {
    /// 新しいレンダラーインスタンスを作成
    ///
    /// `include_str!` で埋め込んだテンプレートを tera に登録する。
    pub fn new() -> Result<Self, NotificationError> {
        let mut engine = Tera::default();

        engine
            .add_raw_templates(vec![
                (
                    "approval_request.html",
                    include_str!("../../../templates/notifications/approval_request.html"),
                ),
                (
                    "approval_request.txt",
                    include_str!("../../../templates/notifications/approval_request.txt"),
                ),
                (
                    "step_approved.html",
                    include_str!("../../../templates/notifications/step_approved.html"),
                ),
                (
                    "step_approved.txt",
                    include_str!("../../../templates/notifications/step_approved.txt"),
                ),
                (
                    "approved.html",
                    include_str!("../../../templates/notifications/approved.html"),
                ),
                (
                    "approved.txt",
                    include_str!("../../../templates/notifications/approved.txt"),
                ),
                (
                    "rejected.html",
                    include_str!("../../../templates/notifications/rejected.html"),
                ),
                (
                    "rejected.txt",
                    include_str!("../../../templates/notifications/rejected.txt"),
                ),
                (
                    "changes_requested.html",
                    include_str!("../../../templates/notifications/changes_requested.html"),
                ),
                (
                    "changes_requested.txt",
                    include_str!("../../../templates/notifications/changes_requested.txt"),
                ),
            ])
            .map_err(|e| NotificationError::TemplateFailed(e.to_string()))?;

        Ok(Self { engine })
    }

    /// 通知イベントからメールメッセージを生成する
    ///
    /// # 引数
    ///
    /// - `notification`: ワークフロー通知イベント
    /// - `base_url`: アプリケーションのベース URL（例: `http://localhost:5173`）
    pub fn render(
        &self,
        notification: &WorkflowNotification,
        base_url: &str,
    ) -> Result<EmailMessage, NotificationError> {
        let (template_name, subject, context) = self.build_template_params(notification, base_url);

        let html_body = self
            .engine
            .render(&format!("{template_name}.html"), &context)
            .map_err(|e| NotificationError::TemplateFailed(e.to_string()))?;

        let text_body = self
            .engine
            .render(&format!("{template_name}.txt"), &context)
            .map_err(|e| NotificationError::TemplateFailed(e.to_string()))?;

        Ok(EmailMessage {
            to: notification.recipient_email().to_string(),
            subject,
            html_body,
            text_body,
        })
    }

    /// テンプレート名、件名、コンテキストを構築する
    fn build_template_params(
        &self,
        notification: &WorkflowNotification,
        base_url: &str,
    ) -> (String, String, Context) {
        let workflow_title = notification.workflow_title();
        let workflow_display_id = notification.workflow_display_id();
        let workflow_url = format!("{base_url}/workflows/{workflow_display_id}");

        let mut context = Context::new();
        context.insert("workflow_title", workflow_title);
        context.insert("workflow_display_id", workflow_display_id);
        context.insert("workflow_url", &workflow_url);

        let (template_name, subject) = match notification {
            WorkflowNotification::ApprovalRequest {
                applicant_name,
                step_name,
                ..
            } => {
                context.insert("applicant_name", applicant_name);
                context.insert("step_name", step_name);
                (
                    "approval_request".to_string(),
                    format!("[RingiFlow] 承認依頼: {workflow_title} {workflow_display_id}"),
                )
            }
            WorkflowNotification::StepApproved {
                step_name,
                approver_name,
                ..
            } => {
                context.insert("step_name", step_name);
                context.insert("approver_name", approver_name);
                (
                    "step_approved".to_string(),
                    format!("[RingiFlow] ステップ承認: {workflow_title} {workflow_display_id}"),
                )
            }
            WorkflowNotification::Approved { .. } => (
                "approved".to_string(),
                format!("[RingiFlow] 承認完了: {workflow_title} {workflow_display_id}"),
            ),
            WorkflowNotification::Rejected { comment, .. } => {
                context.insert("comment", &comment.as_deref().unwrap_or(""));
                (
                    "rejected".to_string(),
                    format!("[RingiFlow] 却下: {workflow_title} {workflow_display_id}"),
                )
            }
            WorkflowNotification::ChangesRequested { comment, .. } => {
                context.insert("comment", &comment.as_deref().unwrap_or(""));
                (
                    "changes_requested".to_string(),
                    format!("[RingiFlow] 要修正: {workflow_title} {workflow_display_id}"),
                )
            }
        };

        (template_name, subject, context)
    }
}

#[cfg(test)]
mod tests {
    use ringiflow_domain::user::UserId;

    use super::*;

    fn make_base_url() -> &'static str {
        "http://localhost:5173"
    }

    #[test]
    fn newが正常に初期化される() {
        let renderer = TemplateRenderer::new();
        assert!(renderer.is_ok());
    }

    #[test]
    fn approval_requestのレンダリングが正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::ApprovalRequest {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            applicant_name:      "田中太郎".to_string(),
            step_name:           "上長承認".to_string(),
            approver_email:      "suzuki@example.com".to_string(),
            approver_user_id:    UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert_eq!(email.to, "suzuki@example.com");
        assert_eq!(email.subject, "[RingiFlow] 承認依頼: 経費精算申請 WF-0042");
        assert!(email.html_body.contains("田中太郎"));
        assert!(email.html_body.contains("上長承認"));
        assert!(
            email
                .html_body
                .contains("http://localhost:5173/workflows/WF-0042")
        );
        assert!(email.text_body.contains("田中太郎"));
        assert!(email.text_body.contains("上長承認"));
    }

    #[test]
    fn step_approvedのレンダリングが正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::StepApproved {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            step_name:           "上長承認".to_string(),
            approver_name:       "鈴木一郎".to_string(),
            applicant_email:     "tanaka@example.com".to_string(),
            applicant_user_id:   UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert_eq!(email.to, "tanaka@example.com");
        assert_eq!(
            email.subject,
            "[RingiFlow] ステップ承認: 経費精算申請 WF-0042"
        );
        assert!(email.html_body.contains("鈴木一郎"));
        assert!(email.html_body.contains("上長承認"));
    }

    #[test]
    fn approvedのレンダリングが正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::Approved {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            applicant_email:     "tanaka@example.com".to_string(),
            applicant_user_id:   UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert_eq!(email.to, "tanaka@example.com");
        assert_eq!(email.subject, "[RingiFlow] 承認完了: 経費精算申請 WF-0042");
        assert!(email.html_body.contains("経費精算申請"));
        assert!(email.html_body.contains("WF-0042"));
    }

    #[test]
    fn rejectedのレンダリングでcommentありの場合が正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::Rejected {
            workflow_title: "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            comment: Some("領収書が添付されていません".to_string()),
            applicant_email: "tanaka@example.com".to_string(),
            applicant_user_id: UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert_eq!(email.subject, "[RingiFlow] 却下: 経費精算申請 WF-0042");
        assert!(email.html_body.contains("領収書が添付されていません"));
        assert!(email.text_body.contains("領収書が添付されていません"));
    }

    #[test]
    fn rejectedのレンダリングでcommentなしの場合が正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::Rejected {
            workflow_title: "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            comment: None,
            applicant_email: "tanaka@example.com".to_string(),
            applicant_user_id: UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert_eq!(email.subject, "[RingiFlow] 却下: 経費精算申請 WF-0042");
        // comment なしの場合、コメント行が表示されないことを確認
        assert!(!email.html_body.contains("コメント:"));
    }

    #[test]
    fn changes_requestedのレンダリングでcommentありの場合が正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::ChangesRequested {
            workflow_title: "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            comment: Some("金額の内訳を追記してください".to_string()),
            applicant_email: "tanaka@example.com".to_string(),
            applicant_user_id: UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert_eq!(email.subject, "[RingiFlow] 要修正: 経費精算申請 WF-0042");
        assert!(email.html_body.contains("金額の内訳を追記してください"));
        assert!(email.text_body.contains("金額の内訳を追記してください"));
    }

    #[test]
    fn changes_requestedのレンダリングでcommentなしの場合が正しい() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::ChangesRequested {
            workflow_title: "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            comment: None,
            applicant_email: "tanaka@example.com".to_string(),
            applicant_user_id: UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert!(!email.html_body.contains("コメント:"));
    }

    #[test]
    fn htmlにワークフロー詳細リンクが含まれる() {
        let renderer = TemplateRenderer::new().unwrap();
        let notification = WorkflowNotification::Approved {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            applicant_email:     "tanaka@example.com".to_string(),
            applicant_user_id:   UserId::new(),
        };

        let email = renderer.render(&notification, make_base_url()).unwrap();

        assert!(
            email
                .html_body
                .contains("http://localhost:5173/workflows/WF-0042")
        );
    }
}
