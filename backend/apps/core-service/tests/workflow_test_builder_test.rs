//! WorkflowTestBuilder の統合テスト

mod helpers;

use helpers::WorkflowTestBuilder;

#[test]
fn test_workflow_test_builder_can_be_imported() {
   let _builder = WorkflowTestBuilder::new();
}
