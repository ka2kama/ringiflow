//! # 通知
//!
//! メール通知に関するドメインモデルを定義する。
//!
//! ## ドメイン用語
//!
//! | 型 | ドメイン用語 | 要件 |
//! |---|------------|------|
//! | [`WorkflowNotification`] | ワークフロー通知イベント | NOTIFY-001: メール通知基盤 |
//! | [`NotificationEventType`] | 通知イベント種別 | 5 種類: 承認依頼、ステップ承認、承認完了、却下、差し戻し |
//!
//! ## 設計方針
//!
//! - **enum による通知イベント**: 各バリアントが機能仕様書の通知イベントに対応
//! - **fire-and-forget**: 通知送信の失敗はワークフロー操作に影響しない
//! - **テンプレート分離**: 通知イベントとメール生成は分離（TemplateRenderer は core-service）
//!
//! → 詳細設計: `docs/03_詳細設計書/16_通知機能設計.md`

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use thiserror::Error;

use crate::user::UserId;

define_uuid_id! {
    /// 通知ログ ID（一意識別子）
    ///
    /// notification_logs テーブルの主キー。UUID v7 を使用。
    pub struct NotificationLogId;
}

/// 通知送信エラー
#[derive(Debug, Error)]
pub enum NotificationError {
    /// メール送信に失敗
    #[error("メール送信に失敗: {0}")]
    SendFailed(String),

    /// テンプレートレンダリングに失敗
    #[error("テンプレートレンダリングに失敗: {0}")]
    TemplateFailed(String),

    /// 通知ログの記録に失敗
    #[error("通知ログの記録に失敗: {0}")]
    LogFailed(String),
}

/// 通知イベント種別
///
/// notification_logs テーブルの `event_type` カラムに格納される値。
/// snake_case でシリアライズされる。
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    IntoStaticStr,
    strum::Display,
    strum::EnumString,
)]
#[strum(serialize_all = "snake_case")]
pub enum NotificationEventType {
    /// 承認依頼: ステップが active になったとき → 承認者に送信
    ApprovalRequest,
    /// ステップ承認（中間）: 多段階承認の中間ステップ承認 → 申請者に送信
    StepApproved,
    /// 承認完了: 最終ステップ承認でインスタンスが Approved → 申請者に送信
    Approved,
    /// 却下: ステップ却下でインスタンスが Rejected → 申請者に送信
    Rejected,
    /// 差し戻し: ステップ差し戻しでインスタンスが ChangesRequested → 申請者に送信
    ChangesRequested,
}

/// メールメッセージ
///
/// テンプレートレンダリングの出力。NotificationSender に渡される。
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// 送信先メールアドレス
    pub to:        String,
    /// 件名
    pub subject:   String,
    /// HTML 本文
    pub html_body: String,
    /// プレーンテキスト本文
    pub text_body: String,
}

/// ワークフロー通知イベント
///
/// 各バリアントが機能仕様書の通知イベント（5 種類）に対応する。
/// → 機能仕様書: `docs/01_要件定義書/機能仕様書/05_通知機能.md`
#[derive(Debug, Clone)]
pub enum WorkflowNotification {
    /// 承認依頼: ステップが active になったとき → 承認者に送信
    ApprovalRequest {
        workflow_title:      String,
        workflow_display_id: String,
        applicant_name:      String,
        step_name:           String,
        approver_email:      String,
        approver_user_id:    UserId,
    },
    /// ステップ承認（中間）: 多段階承認の中間ステップ承認 → 申請者に送信
    StepApproved {
        workflow_title:      String,
        workflow_display_id: String,
        step_name:           String,
        approver_name:       String,
        applicant_email:     String,
        applicant_user_id:   UserId,
    },
    /// 承認完了: 最終ステップ承認でインスタンスが Approved → 申請者に送信
    Approved {
        workflow_title:      String,
        workflow_display_id: String,
        applicant_email:     String,
        applicant_user_id:   UserId,
    },
    /// 却下: ステップ却下でインスタンスが Rejected → 申請者に送信
    Rejected {
        workflow_title: String,
        workflow_display_id: String,
        comment: Option<String>,
        applicant_email: String,
        applicant_user_id: UserId,
    },
    /// 差し戻し: ステップ差し戻しでインスタンスが ChangesRequested → 申請者に送信
    ChangesRequested {
        workflow_title: String,
        workflow_display_id: String,
        comment: Option<String>,
        applicant_email: String,
        applicant_user_id: UserId,
    },
}

impl WorkflowNotification {
    /// 通知イベント種別を返す
    pub fn event_type(&self) -> NotificationEventType {
        match self {
            Self::ApprovalRequest { .. } => NotificationEventType::ApprovalRequest,
            Self::StepApproved { .. } => NotificationEventType::StepApproved,
            Self::Approved { .. } => NotificationEventType::Approved,
            Self::Rejected { .. } => NotificationEventType::Rejected,
            Self::ChangesRequested { .. } => NotificationEventType::ChangesRequested,
        }
    }

    /// 受信者のメールアドレスを返す
    pub fn recipient_email(&self) -> &str {
        match self {
            Self::ApprovalRequest { approver_email, .. } => approver_email,
            Self::StepApproved {
                applicant_email, ..
            }
            | Self::Approved {
                applicant_email, ..
            }
            | Self::Rejected {
                applicant_email, ..
            }
            | Self::ChangesRequested {
                applicant_email, ..
            } => applicant_email,
        }
    }

    /// 受信者のユーザー ID を返す
    pub fn recipient_user_id(&self) -> &UserId {
        match self {
            Self::ApprovalRequest {
                approver_user_id, ..
            } => approver_user_id,
            Self::StepApproved {
                applicant_user_id, ..
            }
            | Self::Approved {
                applicant_user_id, ..
            }
            | Self::Rejected {
                applicant_user_id, ..
            }
            | Self::ChangesRequested {
                applicant_user_id, ..
            } => applicant_user_id,
        }
    }

    /// ワークフロータイトルを返す
    pub fn workflow_title(&self) -> &str {
        match self {
            Self::ApprovalRequest { workflow_title, .. }
            | Self::StepApproved { workflow_title, .. }
            | Self::Approved { workflow_title, .. }
            | Self::Rejected { workflow_title, .. }
            | Self::ChangesRequested { workflow_title, .. } => workflow_title,
        }
    }

    /// ワークフロー表示 ID を返す
    pub fn workflow_display_id(&self) -> &str {
        match self {
            Self::ApprovalRequest {
                workflow_display_id,
                ..
            }
            | Self::StepApproved {
                workflow_display_id,
                ..
            }
            | Self::Approved {
                workflow_display_id,
                ..
            }
            | Self::Rejected {
                workflow_display_id,
                ..
            }
            | Self::ChangesRequested {
                workflow_display_id,
                ..
            } => workflow_display_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn notification_event_type_の文字列変換が正しい() {
        // Display (snake_case)
        assert_eq!(
            NotificationEventType::ApprovalRequest.to_string(),
            "approval_request"
        );
        assert_eq!(
            NotificationEventType::StepApproved.to_string(),
            "step_approved"
        );
        assert_eq!(NotificationEventType::Approved.to_string(), "approved");
        assert_eq!(NotificationEventType::Rejected.to_string(), "rejected");
        assert_eq!(
            NotificationEventType::ChangesRequested.to_string(),
            "changes_requested"
        );

        // FromStr (snake_case)
        assert_eq!(
            NotificationEventType::from_str("approval_request").unwrap(),
            NotificationEventType::ApprovalRequest
        );
        assert_eq!(
            NotificationEventType::from_str("step_approved").unwrap(),
            NotificationEventType::StepApproved
        );
        assert_eq!(
            NotificationEventType::from_str("approved").unwrap(),
            NotificationEventType::Approved
        );
        assert_eq!(
            NotificationEventType::from_str("rejected").unwrap(),
            NotificationEventType::Rejected
        );
        assert_eq!(
            NotificationEventType::from_str("changes_requested").unwrap(),
            NotificationEventType::ChangesRequested
        );
    }

    fn make_approval_request() -> WorkflowNotification {
        WorkflowNotification::ApprovalRequest {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            applicant_name:      "田中太郎".to_string(),
            step_name:           "上長承認".to_string(),
            approver_email:      "suzuki@example.com".to_string(),
            approver_user_id:    UserId::new(),
        }
    }

    fn make_step_approved() -> WorkflowNotification {
        WorkflowNotification::StepApproved {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            step_name:           "上長承認".to_string(),
            approver_name:       "鈴木一郎".to_string(),
            applicant_email:     "tanaka@example.com".to_string(),
            applicant_user_id:   UserId::new(),
        }
    }

    fn make_approved() -> WorkflowNotification {
        WorkflowNotification::Approved {
            workflow_title:      "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            applicant_email:     "tanaka@example.com".to_string(),
            applicant_user_id:   UserId::new(),
        }
    }

    fn make_rejected() -> WorkflowNotification {
        WorkflowNotification::Rejected {
            workflow_title: "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            comment: Some("領収書が添付されていません".to_string()),
            applicant_email: "tanaka@example.com".to_string(),
            applicant_user_id: UserId::new(),
        }
    }

    fn make_changes_requested() -> WorkflowNotification {
        WorkflowNotification::ChangesRequested {
            workflow_title: "経費精算申請".to_string(),
            workflow_display_id: "WF-0042".to_string(),
            comment: Some("金額の内訳を追記してください".to_string()),
            applicant_email: "tanaka@example.com".to_string(),
            applicant_user_id: UserId::new(),
        }
    }

    #[test]
    fn event_typeが各バリアントで正しい値を返す() {
        assert_eq!(
            make_approval_request().event_type(),
            NotificationEventType::ApprovalRequest
        );
        assert_eq!(
            make_step_approved().event_type(),
            NotificationEventType::StepApproved
        );
        assert_eq!(
            make_approved().event_type(),
            NotificationEventType::Approved
        );
        assert_eq!(
            make_rejected().event_type(),
            NotificationEventType::Rejected
        );
        assert_eq!(
            make_changes_requested().event_type(),
            NotificationEventType::ChangesRequested
        );
    }

    #[test]
    fn recipient_emailが各バリアントで正しいメールアドレスを返す() {
        // ApprovalRequest → 承認者のメールアドレス
        assert_eq!(
            make_approval_request().recipient_email(),
            "suzuki@example.com"
        );

        // その他 → 申請者のメールアドレス
        assert_eq!(make_step_approved().recipient_email(), "tanaka@example.com");
        assert_eq!(make_approved().recipient_email(), "tanaka@example.com");
        assert_eq!(make_rejected().recipient_email(), "tanaka@example.com");
        assert_eq!(
            make_changes_requested().recipient_email(),
            "tanaka@example.com"
        );
    }

    #[test]
    fn recipient_user_idが各バリアントで正しいユーザーidを返す() {
        let approver_id = UserId::new();
        let applicant_id = UserId::new();

        let approval_request = WorkflowNotification::ApprovalRequest {
            workflow_title:      "テスト".to_string(),
            workflow_display_id: "WF-0001".to_string(),
            applicant_name:      "田中".to_string(),
            step_name:           "承認".to_string(),
            approver_email:      "approver@example.com".to_string(),
            approver_user_id:    approver_id.clone(),
        };
        assert_eq!(approval_request.recipient_user_id(), &approver_id);

        let approved = WorkflowNotification::Approved {
            workflow_title:      "テスト".to_string(),
            workflow_display_id: "WF-0001".to_string(),
            applicant_email:     "applicant@example.com".to_string(),
            applicant_user_id:   applicant_id.clone(),
        };
        assert_eq!(approved.recipient_user_id(), &applicant_id);
    }
}
