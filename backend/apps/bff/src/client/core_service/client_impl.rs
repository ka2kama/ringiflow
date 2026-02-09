//! CoreServiceClient スーパートレイトとクライアント実装の構造体

use super::{
   task_client::CoreServiceTaskClient,
   user_client::CoreServiceUserClient,
   workflow_client::CoreServiceWorkflowClient,
};

/// Core Service クライアントトレイト（スーパートレイト）
///
/// User / Workflow / Task の各サブトレイトを束ねるスーパートレイト。
/// テスト時にはサブトレイト単位でスタブを使用できる。
///
/// `dyn CoreServiceClient` はオブジェクトセーフであり、従来通り
/// `Arc<dyn CoreServiceClient>` として使用可能。
pub trait CoreServiceClient:
   CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient
{
}

/// ブランケット impl: 3 つのサブトレイトをすべて実装する型は
/// 自動的に `CoreServiceClient` を実装する。
impl<T> CoreServiceClient for T where
   T: CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient
{
}

/// Core Service クライアント実装
#[derive(Clone)]
pub struct CoreServiceClientImpl {
   pub(super) base_url: String,
   pub(super) client:   reqwest::Client,
}

impl CoreServiceClientImpl {
   /// 新しい CoreServiceClient を作成する
   ///
   /// # 引数
   ///
   /// - `base_url`: Core Service のベース URL（例: `http://localhost:13001`）
   pub fn new(base_url: &str) -> Self {
      Self {
         base_url: base_url.trim_end_matches('/').to_string(),
         client:   reqwest::Client::new(),
      }
   }
}
