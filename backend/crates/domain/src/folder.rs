//! # フォルダ
//!
//! ドキュメント管理のフォルダ階層構造を表現するドメインモデル。
//!
//! ## Materialized Path パターン
//!
//! フォルダの階層関係を `path` カラムに文字列として格納する方式。
//! 例: `/2026年度/経費精算/` はルート直下の「2026年度」フォルダの
//! 子フォルダ「経費精算」を表す。
//!
//! サブツリーの取得が `LIKE '{path}%'` で高速に行える（読み取り最適化）。
//! 移動時はサブツリー全体の path 更新が必要だが、5 階層制限で影響は限定的。
//!
//! → 設計判断: [ドキュメント管理設計](../../../../docs/03_詳細設計書/17_ドキュメント管理設計.md)
//!
//! ## 使用例
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ringiflow_domain::{
//!     folder::{Folder, FolderId, FolderName, MAX_FOLDER_DEPTH},
//!     tenant::TenantId,
//!     user::UserId,
//! };
//!
//! // ルート直下にフォルダを作成
//! let name = FolderName::new("2026年度")?;
//! let folder = Folder::new(
//!     FolderId::new(),
//!     TenantId::new(),
//!     name,
//!     None, // parent_id
//!     None, // parent_path
//!     None, // parent_depth
//!     Some(UserId::new()),
//!     chrono::Utc::now(),
//! )?;
//!
//! assert_eq!(folder.path(), "/2026年度/");
//! assert_eq!(folder.depth(), 1);
//! # Ok(())
//! # }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{DomainError, tenant::TenantId, user::UserId};

define_uuid_id! {
    /// フォルダの一意識別子
    pub struct FolderId;
}

// =========================================================================
// FolderName（フォルダ名）
// =========================================================================

/// ファイルシステムとの互換性のため使用を禁止する文字
const FORBIDDEN_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// フォルダ名の最大文字数（DB: `VARCHAR(255)`）
const MAX_FOLDER_NAME_LENGTH: usize = 255;

/// フォルダ名（値オブジェクト）
///
/// 1〜255 文字。ファイルシステム互換の禁止文字チェック付き。
///
/// # 不変条件
///
/// - 空文字列ではない
/// - 最大 255 文字
/// - 禁止文字（`/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`）を含まない
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderName(String);

impl FolderName {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into().trim().to_string();

        if value.is_empty() {
            return Err(DomainError::Validation(
                "フォルダ名を入力してください".to_string(),
            ));
        }

        if value.chars().count() > MAX_FOLDER_NAME_LENGTH {
            return Err(DomainError::Validation(
                "フォルダ名は 255 文字以内で入力してください".to_string(),
            ));
        }

        if value.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) {
            return Err(DomainError::Validation(
                "フォルダ名に使用できない文字が含まれています".to_string(),
            ));
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

impl std::fmt::Display for FolderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// =========================================================================
// Folder（フォルダエンティティ）
// =========================================================================

/// フォルダの最大階層数
pub const MAX_FOLDER_DEPTH: i32 = 5;

/// フォルダエンティティ
///
/// materialized path による階層構造を持つ。
///
/// # 不変条件
///
/// - `depth` は 1 以上 5 以下
/// - `path` はルートフォルダなら `"/{name}/"`、子フォルダなら `"{parent.path}{name}/"`
/// - ルートフォルダの `parent_id` は `None`、`depth` は 1
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    id:         FolderId,
    tenant_id:  TenantId,
    name:       FolderName,
    parent_id:  Option<FolderId>,
    path:       String,
    depth:      i32,
    created_by: Option<UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Folder {
    /// 新しいフォルダを作成する
    ///
    /// # 引数
    ///
    /// - `parent_path`: 親フォルダの path（ルート直下なら `None`）
    /// - `parent_depth`: 親フォルダの depth（ルート直下なら `None`）
    // FIXME: 引数が多い。親フォルダ情報を値オブジェクトにまとめて引数を削減する
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: FolderId,
        tenant_id: TenantId,
        name: FolderName,
        parent_id: Option<FolderId>,
        parent_path: Option<&str>,
        parent_depth: Option<i32>,
        created_by: Option<UserId>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let (path, depth) = match (parent_path, parent_depth) {
            (Some(pp), Some(pd)) => {
                let depth = pd + 1;
                if depth > MAX_FOLDER_DEPTH {
                    return Err(DomainError::Validation(
                        "フォルダの階層が上限（5 階層）を超えています".to_string(),
                    ));
                }
                let path = format!("{}{}/", pp, name.as_str());
                (path, depth)
            }
            _ => {
                let path = format!("/{}/", name.as_str());
                (path, 1)
            }
        };

        Ok(Self {
            id,
            tenant_id,
            name,
            parent_id,
            path,
            depth,
            created_by,
            created_at: now,
            updated_at: now,
        })
    }

    /// データベースからフォルダを復元する
    // FIXME: 引数が多い。DB 行データの中間構造体を経由して引数を削減する
    #[allow(clippy::too_many_arguments)]
    pub fn from_db(
        id: FolderId,
        tenant_id: TenantId,
        name: FolderName,
        parent_id: Option<FolderId>,
        path: String,
        depth: i32,
        created_by: Option<UserId>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            name,
            parent_id,
            path,
            depth,
            created_by,
            created_at,
            updated_at,
        }
    }

    /// フォルダ名を変更する
    ///
    /// 自身の path を再計算した新インスタンスを返す。
    /// サブツリーの path 更新はリポジトリ層で実施する。
    pub fn rename(&self, new_name: FolderName, now: DateTime<Utc>) -> Self {
        // path の末尾のフォルダ名部分を置換
        // 現在の path: "/.../old_name/" → "/.../new_name/"
        let parent_path = self.parent_path();
        let new_path = format!("{}{}/", parent_path, new_name.as_str());

        Self {
            id:         self.id.clone(),
            tenant_id:  self.tenant_id.clone(),
            name:       new_name,
            parent_id:  self.parent_id.clone(),
            path:       new_path,
            depth:      self.depth,
            created_by: self.created_by.clone(),
            created_at: self.created_at,
            updated_at: now,
        }
    }

    /// フォルダを別の親に移動する
    ///
    /// 新しい parent_id、path、depth を計算した新インスタンスを返す。
    /// サブツリーの path/depth 更新はリポジトリ層で実施する。
    pub fn move_to(
        &self,
        new_parent_id: Option<FolderId>,
        new_parent_path: Option<&str>,
        new_parent_depth: Option<i32>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let (new_path, new_depth) = match (new_parent_path, new_parent_depth) {
            (Some(pp), Some(pd)) => {
                let depth = pd + 1;
                if depth > MAX_FOLDER_DEPTH {
                    return Err(DomainError::Validation(
                        "フォルダの階層が上限（5 階層）を超えています".to_string(),
                    ));
                }
                let path = format!("{}{}/", pp, self.name.as_str());
                (path, depth)
            }
            _ => {
                let path = format!("/{}/", self.name.as_str());
                (path, 1)
            }
        };

        Ok(Self {
            id:         self.id.clone(),
            tenant_id:  self.tenant_id.clone(),
            name:       self.name.clone(),
            parent_id:  new_parent_id,
            path:       new_path,
            depth:      new_depth,
            created_by: self.created_by.clone(),
            created_at: self.created_at,
            updated_at: now,
        })
    }

    /// 子フォルダの path を計算する
    pub fn child_path(&self, child_name: &str) -> String {
        format!("{}{}/", self.path, child_name)
    }

    /// 子フォルダの depth を計算する
    pub fn child_depth(&self) -> Result<i32, DomainError> {
        let new_depth = self.depth + 1;
        if new_depth > MAX_FOLDER_DEPTH {
            return Err(DomainError::Validation(
                "フォルダの階層が上限（5 階層）を超えています".to_string(),
            ));
        }
        Ok(new_depth)
    }

    /// 親フォルダの path 部分を取得する
    ///
    /// 例: path = "/a/b/c/" → parent_path = "/a/b/"
    /// 例: path = "/root/" → parent_path = "/"
    fn parent_path(&self) -> &str {
        // path の末尾の "name/" を除去
        let without_trailing_slash = &self.path[..self.path.len() - 1]; // "/a/b/c"
        match without_trailing_slash.rfind('/') {
            Some(idx) => &self.path[..idx + 1], // "/a/b/"
            None => "/",
        }
    }

    // --- ゲッター ---

    pub fn id(&self) -> &FolderId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn name(&self) -> &FolderName {
        &self.name
    }

    pub fn parent_id(&self) -> Option<&FolderId> {
        self.parent_id.as_ref()
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn depth(&self) -> i32 {
        self.depth
    }

    pub fn created_by(&self) -> Option<&UserId> {
        self.created_by.as_ref()
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    // =========================================================================
    // FolderName のテスト
    // =========================================================================

    #[test]
    fn test_フォルダ名は正常な名前を受け入れる() {
        let name = FolderName::new("2026年度");
        assert!(name.is_ok());
        assert_eq!(name.unwrap().as_str(), "2026年度");
    }

    #[test]
    fn test_フォルダ名は空文字列を拒否する() {
        assert!(FolderName::new("").is_err());
    }

    #[test]
    fn test_フォルダ名は空白のみを拒否する() {
        assert!(FolderName::new("   ").is_err());
    }

    #[test]
    fn test_フォルダ名は前後の空白をトリミングする() {
        let name = FolderName::new("  経費精算  ").unwrap();
        assert_eq!(name.as_str(), "経費精算");
    }

    #[test]
    fn test_フォルダ名は255文字以内を受け入れる() {
        let name = "a".repeat(255);
        assert!(FolderName::new(name).is_ok());
    }

    #[test]
    fn test_フォルダ名は255文字超を拒否する() {
        let name = "a".repeat(256);
        assert!(FolderName::new(name).is_err());
    }

    #[rstest]
    #[case('/', "スラッシュ")]
    #[case('\\', "バックスラッシュ")]
    #[case(':', "コロン")]
    #[case('*', "アスタリスク")]
    #[case('?', "クエスチョン")]
    #[case('"', "ダブルクォート")]
    #[case('<', "小なり")]
    #[case('>', "大なり")]
    #[case('|', "パイプ")]
    fn test_フォルダ名は禁止文字を拒否する(
        #[case] ch: char,
        #[case] _description: &str,
    ) {
        let name = format!("folder{}name", ch);
        assert!(FolderName::new(name).is_err());
    }

    // =========================================================================
    // Folder のテスト
    // =========================================================================

    fn fixed_now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    #[test]
    fn test_ルート直下にフォルダを作成する() {
        let name = FolderName::new("2026年度").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            None,
            None,
            None,
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap();

        assert_eq!(folder.path(), "/2026年度/");
        assert_eq!(folder.depth(), 1);
        assert!(folder.parent_id().is_none());
    }

    #[test]
    fn test_子フォルダを作成する() {
        let parent_id = FolderId::new();
        let name = FolderName::new("経費精算").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(parent_id),
            Some("/2026年度/"),
            Some(1),
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap();

        assert_eq!(folder.path(), "/2026年度/経費精算/");
        assert_eq!(folder.depth(), 2);
        assert!(folder.parent_id().is_some());
    }

    #[test]
    fn test_5階層を超える作成を拒否する() {
        let name = FolderName::new("too-deep").unwrap();
        let result = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(FolderId::new()),
            Some("/a/b/c/d/e/"),
            Some(5), // parent depth = 5 → child depth = 6 → エラー
            Some(UserId::new()),
            fixed_now(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_5階層ちょうどは許可される() {
        let name = FolderName::new("level5").unwrap();
        let result = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(FolderId::new()),
            Some("/a/b/c/d/"),
            Some(4), // parent depth = 4 → child depth = 5 → OK
            Some(UserId::new()),
            fixed_now(),
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap().depth(), 5);
    }

    #[test]
    fn test_renameで名前とpathが更新される() {
        let name = FolderName::new("old-name").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(FolderId::new()),
            Some("/parent/"),
            Some(1),
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap();

        let new_name = FolderName::new("new-name").unwrap();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let renamed = folder.rename(new_name, later);

        assert_eq!(renamed.name().as_str(), "new-name");
        assert_eq!(renamed.path(), "/parent/new-name/");
        assert_eq!(renamed.depth(), 2); // depth は変わらない
        assert_eq!(renamed.updated_at(), later);
    }

    #[test]
    fn test_move_toでparent_idとpathとdepthが更新される() {
        let name = FolderName::new("moving").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(FolderId::new()),
            Some("/old-parent/"),
            Some(1),
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap();

        let new_parent_id = FolderId::new();
        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let moved = folder
            .move_to(
                Some(new_parent_id.clone()),
                Some("/new-parent/sub/"),
                Some(2),
                later,
            )
            .unwrap();

        assert_eq!(moved.parent_id(), Some(&new_parent_id));
        assert_eq!(moved.path(), "/new-parent/sub/moving/");
        assert_eq!(moved.depth(), 3);
        assert_eq!(moved.updated_at(), later);
    }

    #[test]
    fn test_move_toでルートに移動する() {
        let name = FolderName::new("to-root").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(FolderId::new()),
            Some("/parent/"),
            Some(1),
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap();

        let later = DateTime::from_timestamp(1_700_001_000, 0).unwrap();
        let moved = folder.move_to(None, None, None, later).unwrap();

        assert!(moved.parent_id().is_none());
        assert_eq!(moved.path(), "/to-root/");
        assert_eq!(moved.depth(), 1);
    }

    #[test]
    fn test_from_dbでフォルダを復元できる() {
        let id = FolderId::new();
        let tenant_id = TenantId::new();
        let name = FolderName::new("test").unwrap();
        let now = fixed_now();

        let sut = Folder::from_db(
            id.clone(),
            tenant_id.clone(),
            name.clone(),
            None,
            "/test/".to_string(),
            1,
            None,
            now,
            now,
        );

        assert_eq!(sut.id(), &id);
        assert_eq!(sut.tenant_id(), &tenant_id);
        assert_eq!(sut.name(), &name);
        assert_eq!(sut.path(), "/test/");
        assert_eq!(sut.depth(), 1);
    }

    #[test]
    fn test_child_pathで子フォルダのパスを計算する() {
        let name = FolderName::new("parent").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            None,
            None,
            None,
            None,
            fixed_now(),
        )
        .unwrap();

        assert_eq!(folder.child_path("child"), "/parent/child/");
    }

    #[test]
    fn test_child_depthで子フォルダのdepthを計算する() {
        let name = FolderName::new("parent").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            None,
            None,
            None,
            None,
            fixed_now(),
        )
        .unwrap();

        assert_eq!(folder.child_depth().unwrap(), 2);
    }

    #[test]
    fn test_child_depthで5階層の親はエラーを返す() {
        let name = FolderName::new("level5").unwrap();
        let folder = Folder::new(
            FolderId::new(),
            TenantId::new(),
            name,
            Some(FolderId::new()),
            Some("/a/b/c/d/"),
            Some(4),
            None,
            fixed_now(),
        )
        .unwrap();

        assert_eq!(folder.depth(), 5);
        assert!(folder.child_depth().is_err());
    }
}
