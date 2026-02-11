//! # OpenAPI YAML 生成ツール
//!
//! BFF の Rust 型から OpenAPI 仕様を YAML 形式で標準出力に出力する。
//!
//! ## 使い方
//!
//! ```bash
//! cargo run --bin generate-openapi -p ringiflow-bff > openapi/openapi.yaml
//! ```

use ringiflow_bff::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
   let yaml = ApiDoc::openapi()
      .to_yaml()
      .expect("OpenAPI YAML 生成に失敗しました");
   print!("{yaml}");
}
