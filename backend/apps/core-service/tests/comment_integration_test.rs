//! ワークフローコメント機能の統合テスト
//!
//! WorkflowTestBuilder を使用したテストの例

use ringiflow_core_service::{
    test_utils::WorkflowTestBuilder,
    usecase::workflow::PostCommentInput,
};
use ringiflow_domain::{
    user::UserId,
    value_objects::DisplayNumber,
    workflow::{NewWorkflowStep, WorkflowStep, WorkflowStepId},
};

#[tokio::test]
async fn test_post_comment_申請者がコメントを投稿できる() {
    // Arrange
    let builder = WorkflowTestBuilder::new();
    let instance = builder.build_submitted_instance("テスト申請", 100);
    let setup = builder.build_workflow_usecase_impl();

    setup.instance_repo.insert(&instance).await.unwrap();

    let input = PostCommentInput {
        body: "テストコメント".to_string(),
    };

    // Act
    let result = setup
        .sut
        .post_comment(
            input,
            DisplayNumber::new(100).unwrap(),
            builder.tenant_id().clone(),
            builder.user_id().clone(),
        )
        .await;

    // Assert
    assert!(result.is_ok());
    let comment = result.unwrap();
    assert_eq!(comment.body().as_str(), "テストコメント");
}

#[tokio::test]
async fn test_post_comment_承認者がコメントを投稿できる() {
    // Arrange
    let builder = WorkflowTestBuilder::new();
    let approver_id = UserId::new();
    let instance = builder.build_submitted_instance("テスト申請", 100);
    let setup = builder.build_workflow_usecase_impl();

    setup.instance_repo.insert(&instance).await.unwrap();

    // 承認者のステップを作成
    let step = WorkflowStep::new(NewWorkflowStep {
        id: WorkflowStepId::new(),
        instance_id: instance.id().clone(),
        display_number: DisplayNumber::new(1).unwrap(),
        step_id: "approval".to_string(),
        step_name: "承認".to_string(),
        step_type: "approval".to_string(),
        assigned_to: Some(approver_id.clone()),
        now: builder.now(),
    })
    .activated(builder.now());
    setup
        .step_repo
        .insert(&step, builder.tenant_id())
        .await
        .unwrap();

    let input = PostCommentInput {
        body: "承認者のコメント".to_string(),
    };

    // Act
    let result = setup
        .sut
        .post_comment(
            input,
            DisplayNumber::new(100).unwrap(),
            builder.tenant_id().clone(),
            approver_id.clone(),
        )
        .await;

    // Assert
    assert!(result.is_ok());
    let comment = result.unwrap();
    assert_eq!(comment.body().as_str(), "承認者のコメント");
}
