/// UUID v7 ベースの ID 型を定義する宣言型マクロ
///
/// 以下のボイラープレートを一括生成する:
/// - Newtype 構造体（`Uuid` をラップ）
/// - `derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Display)`
/// - `new()`: UUID v7 を生成
/// - `from_uuid()`: 既存 UUID から復元
/// - `as_uuid()`: 内部 UUID への参照
/// - `Default` impl（`new()` に委譲）
///
/// # 使用例
///
/// ```rust
/// use ringiflow_domain::tenant::TenantId;
/// use uuid::Uuid;
///
/// let id = TenantId::new();
/// let uuid = id.as_uuid();
/// let restored = TenantId::from_uuid(*uuid);
/// assert_eq!(id, restored);
/// ```
macro_rules! define_uuid_id {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident;
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash,
            serde::Serialize, serde::Deserialize,
            derive_more::Display,
        )]
        #[display("{_0}")]
        $vis struct $Name(uuid::Uuid);

        impl $Name {
            /// 新しい ID を生成する（UUID v7）
            pub fn new() -> Self {
                Self(uuid::Uuid::now_v7())
            }

            /// 既存の UUID から ID を作成する
            pub fn from_uuid(uuid: uuid::Uuid) -> Self {
                Self(uuid)
            }

            /// 内部の UUID 参照を取得する
            pub fn as_uuid(&self) -> &uuid::Uuid {
                &self.0
            }
        }

        impl Default for $Name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// バリデーション付き String Newtype の共通メソッドを生成する内部マクロ
///
/// `define_validated_string!` の PII / 非 PII 両アームで共有される
/// `new()`, `as_str()`, `into_string()` を一括生成する。
macro_rules! _validated_string_common {
    ($Name:ident, $label:expr, $max_length:expr) => {
        impl $Name {
            pub fn new(value: impl Into<String>) -> Result<Self, $crate::DomainError> {
                let value = value.into().trim().to_string();

                if value.is_empty() {
                    return Err($crate::DomainError::Validation(format!(
                        "{}は必須です",
                        $label
                    )));
                }

                if value.chars().count() > $max_length {
                    return Err($crate::DomainError::Validation(format!(
                        "{}は {} 文字以内である必要があります",
                        $label, $max_length
                    )));
                }

                Ok(Self(value))
            }

            /// 文字列参照を取得する
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 所有権を持つ文字列に変換する
            pub fn into_string(self) -> String {
                self.0
            }
        }
    };
}

/// バリデーション付き String Newtype を定義する宣言型マクロ
///
/// 以下のボイラープレートを一括生成する:
/// - Newtype 構造体（`String` をラップ）
/// - `new()`: trim + 空チェック + 最大長チェック
/// - `as_str()`: 文字列参照
/// - `into_string()`: 所有権を持つ文字列に変換
///
/// # PII モード
///
/// `pii: true` を指定すると PII 保護モードになる:
/// - `Debug` 出力を `[REDACTED]` にマスクする
/// - `Display` impl を生成しない（平文出力を防止）
///
/// `pii` を指定しない場合（デフォルト）:
/// - `derive(Debug)` で通常の Debug 出力
/// - `Display` impl を生成（平文出力）
///
/// # 引数
///
/// - `$label`: エラーメッセージに使うラベル（例: `"ユーザー名"`）
/// - `$max_length`: 最大文字数（`chars().count()` でカウント）
/// - `pii`: （任意）`true` を指定すると PII 保護モード
///
/// # 使用例
///
/// ```rust
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use ringiflow_domain::value_objects::UserName;
///
/// let name = UserName::new("山田太郎")?;
/// assert_eq!(name.as_str(), "山田太郎");
/// // Debug 出力はマスクされる（PII 保護）
/// assert!(format!("{:?}", name).contains("[REDACTED]"));
/// # Ok(())
/// # }
/// ```
macro_rules! define_validated_string {
    // PII アーム: Debug をマスク、Display を生成しない
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident {
            label: $label:expr,
            max_length: $max_length:expr,
            pii: true $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Clone, PartialEq, Eq,
            serde::Serialize, serde::Deserialize,
        )]
        $vis struct $Name(String);

        impl std::fmt::Debug for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_tuple(stringify!($Name)).field(&"[REDACTED]").finish()
            }
        }

        _validated_string_common!($Name, $label, $max_length);
    };
    // 非 PII アーム: derive(Debug) + Display 生成
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident {
            label: $label:expr,
            max_length: $max_length:expr $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq,
            serde::Serialize, serde::Deserialize,
        )]
        $vis struct $Name(String);

        _validated_string_common!($Name, $label, $max_length);

        impl std::fmt::Display for $Name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}
