//! # OpenAPI YAML 生成ツール
//!
//! BFF の Rust 型から OpenAPI 仕様を YAML 形式で標準出力に出力する。
//! 生成後、utoipa が自動登録する未使用コンポーネントスキーマを除去する。
//!
//! ## 使い方
//!
//! ```bash
//! cargo run --bin generate-openapi -p ringiflow-bff > openapi/openapi.yaml
//! ```

use std::collections::HashSet;

use ringiflow_bff::openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
   let mut openapi = ApiDoc::openapi();
   remove_unused_schemas(&mut openapi);
   let yaml = openapi.to_yaml().expect("OpenAPI YAML 生成に失敗しました");
   print!("{yaml}");
}

/// utoipa が自動登録する未使用コンポーネントスキーマを除去する
///
/// utoipa の `#[utoipa::path]` マクロは `body = ApiResponse<T>` を処理する際、
/// ジェネリック型パラメータ `T` の standalone スキーマも自動登録する。
/// `ApiResponse` は `T` のフィールドを inline 展開するため、standalone スキーマは
/// どこからも `$ref` されず未使用になる。この関数でそれらを除去する。
fn remove_unused_schemas(openapi: &mut utoipa::openapi::OpenApi) {
   // JSON にシリアライズして全 $ref ターゲットを収集する
   let json = serde_json::to_string(openapi).expect("JSON シリアライズに失敗しました");

   // $ref パターンから参照先スキーマ名を抽出する
   // JSON 形式: "$ref":"#/components/schemas/SchemaName"
   let prefix = "#/components/schemas/";
   let used_schemas: HashSet<&str> = json
      .match_indices(prefix)
      .filter_map(|(start, _)| {
         let rest = &json[start + prefix.len()..];
         // スキーマ名の終端は `"` （JSON 文字列の閉じ引用符）
         rest.find('"').map(|end| &rest[..end])
      })
      .collect();

   if let Some(components) = &mut openapi.components {
      components
         .schemas
         .retain(|name, _| used_schemas.contains(name.as_str()));
   }
}
