//! 合成データ用のメモリ内保存部（QA-GDPR-02 の仮置き。04・05・10 が使う前提の置き場）。
//!
//! - workspace・channel・message・明示メモリ・根拠記録・利用方針・削除/制限記録を
//!   **メモリだけ** に持つ。ディスクには書かない（07 のログが `target/demo-logs/` に
//!   出る既存経路以外、この crate はファイルを作らない）。
//! - 保存する記録の ID・時刻・版・epoch は **サーバーが付ける**。要求や画面から来た
//!   値を正本にしない（12 §3.2 の `communication_basis_records` /
//!   `communication_use_policies` の列名は論理単位で、確定 DDL ではない）。
//! - `scope` と参照列（`*_ref`、`source_scope`、`retention_policy_ref`）は **参照だけ**
//!   にする。保存前に合成の参照形式（ASCII 小文字・数字・`-`・`_` の識別子。
//!   参照列は `ref:` で始まる）の検証を通し、形式に合わない値＝本文や人物情報の
//!   自由文の複製は [`StoreError::InvalidReference`] として **保存しない**
//!   （12 §3.2「人物情報や本文の不要な複製を避ける」）。
//! - `client_msg_id` はこの保存部に入れない。メッセージも既存の
//!   [`crate::message::Message`] ではなく `client_msg_id` の無い [`StoredMessage`]
//!   を使う。
//! - 保存物には `purpose_id`（[`PurposeId`]、P01〜P08 のみ）が必須。未設定・不正値は
//!   serde の脱直列化エラーとして拒否する。学習・広告・会社横断分析の経路は作らない。
//! - 根拠記録の status は 12 §3.2/§3.3 に従い、`pending`・欠落・不明・期限切れ・
//!   撤回済みを同意根拠として使わない。この実装には有効（Active）な根拠記録を
//!   作る経路が無く、同意を使う機能（P03/P04/P06）は初期オフ（他の目的も同様）。
//! - 取消P・同意撤回（根拠失効）・削除の利用停止は別の状態として持てる
//!   （[`HaltKind`]/[`HaltState`]）。この実装が遷移を実装するのは撤回だけ
//!   （撤回後の読取・検索・AI 入力の拒否は QA-GDPR-03 の範囲で、ここでは実装しない）。
//! - 削除の処理そのもの（05）、本人 JSON 書き出し（04）、所在一覧 API（10）は
//!   実装しない。所在は [`StorageRegistry`] に各保存部が自分を登録できる形のみ用意する。

use std::fmt;

use crate::ids::{
    BasisId, ChannelId, DeletionRestrictionId, MemberId, MemoryId, MessageId, WorkspaceId,
};
use crate::member::MemberKind;
use crate::message::Channel;

/// サーバーが付けた時刻（UNIX エポックからのミリ秒）。ログの `ts_ms` と同じ単位。
pub type TsMs = u64;

/// 目的 ID（P01〜P08 のみ。12 §3 の表）。
/// どの根拠の種類がどの目的に当てはまるかは **この crate では決めない**（U03 未決）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum PurposeId {
    P01,
    P02,
    P03,
    P04,
    P05,
    P06,
    P07,
    P08,
}

impl PurposeId {
    /// 全目的（テスト・表示用）。
    pub const ALL: [Self; 8] = [
        Self::P01,
        Self::P02,
        Self::P03,
        Self::P04,
        Self::P05,
        Self::P06,
        Self::P07,
        Self::P08,
    ];
}

/// 根拠の種類（`basis_type` 列）。**合成の値** で、実際の根拠分類との対応は未決
/// （U03。この crate では決めない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BasisType {
    /// 同意のような合成値。
    SyntheticConsent,
    /// 同意以外のような合成値。
    SyntheticNonConsent,
}

/// 根拠記録の status（12 §3.2/§3.3）。`pending`・欠落・不明・期限切れ・撤回済みは
/// 同意根拠として使わない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BasisStatus {
    /// 未確定（作成直後）。同意根拠として使わない。
    Pending,
    /// 有効な根拠が確認できた状態。**この実装はこの状態を作る経路を持たない**
    /// （法的な当てはめは U03 未決。同意を使う機能は初期オフのまま）。
    Active,
    /// 撤回済み（根拠失効）。
    Withdrawn,
}

/// 保存部の workspace（合成）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoredWorkspace {
    pub workspace_id: WorkspaceId,
    /// サーバーが付けた登録時刻。
    pub created_at: TsMs,
}

/// 保存部のメッセージ。既存の [`crate::message::Message`] と違い `client_msg_id` を
/// 持たない（この保存部には `client_msg_id` を入れない）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StoredMessage {
    /// サーバー採番。
    pub message_id: MessageId,
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    /// チャンネル内の発言順（サーバー採番）。
    pub seq: u64,
    pub author_id: MemberId,
    pub member_kind: MemberKind,
    /// 保存に必要な目的（必須・P01〜P08 のみ）。
    pub purpose_id: PurposeId,
    pub body: String,
    pub thread_root: Option<MessageId>,
    /// サーバーが付けた保存時刻。
    pub created_at: TsMs,
}

/// 明示メモリ（合成）。本文（content）を持つが、根拠記録・利用方針はこれを複製しない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExplicitMemory {
    /// サーバー採番。
    pub memory_id: MemoryId,
    pub workspace_id: WorkspaceId,
    /// 将来再利用の目的（必須・P01〜P08 のみ）。
    pub purpose_id: PurposeId,
    /// 合成の内容。
    pub content: String,
    /// source 参照（参照のみ）。
    pub source_ref: String,
    /// サーバーが付けた保存時刻。
    pub created_at: TsMs,
}

/// 利用方針（12 §3.2 `communication_use_policies`。列名は論理単位で確定 DDL ではない）。
/// 変更前の版は証拠参照として [`DataStore`] が残す（削除しない）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct UsePolicy {
    pub workspace_id: WorkspaceId,
    pub purpose_id: PurposeId,
    /// 版（サーバー採番。同じ workspace × purpose で 1 から振る）。
    pub version: u64,
    /// データ分類（合成）。
    pub data_classes: Vec<String>,
    /// source の範囲（合成の参照形式のみ。本文や人物情報の自由文は入れない）。
    pub source_scope: String,
    /// 取扱主体（合成）。
    pub processors: Vec<String>,
    /// 許可される出力（合成）。
    pub allowed_outputs: Vec<String>,
    /// 説明版。
    pub explanation_version: String,
    /// 保持方針への参照（参照のみ。`ref:` で始まる合成の参照形式）。
    pub retention_policy_ref: String,
    /// サーバーが付けた作成時刻（12 §3.2 の列に加えた追加列。時刻はサーバー付けの約束のため）。
    pub created_at: TsMs,
}

/// 根拠記録（12 §3.2 `communication_basis_records`。列名は論理単位で確定 DDL ではない）。
/// 参照列（`*_ref`）は参照のみで、本文や人物情報の不要な複製を持たない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BasisRecord {
    pub workspace_id: WorkspaceId,
    /// サーバー採番。
    pub basis_id: BasisId,
    /// サーバー採番（この実装では作成時に 1）。
    pub basis_version: u64,
    /// 合成の根拠の種類（実際の当てはめは未決・U03）。
    pub basis_type: BasisType,
    pub purpose_id: PurposeId,
    /// 対象 scope（合成の参照形式のみ。`ch-synthetic-02` 等の識別子で、本文や
    /// 人物情報の自由文は入れない）。
    pub scope: String,
    /// 通信当事者の充足状況への参照（参照のみ。`ref:` で始まる合成の参照形式）。
    pub party_coverage_ref: String,
    /// 同意取得者・根拠確認担当への参照（参照のみ。`ref:` で始まる合成の参照形式）。
    pub grantor_ref: String,
    /// 代理権の確認方法への参照（参照のみ。`ref:` で始まる合成の参照形式）。
    pub representative_authority_ref: Option<String>,
    pub explanation_version: String,
    /// 証拠への参照（参照のみ。`ref:` で始まる合成の参照形式）。
    pub evidence_ref: String,
    /// サーバーが付けた取得時刻。
    pub collected_at: TsMs,
    /// 発効時刻（未確定のうちは `None`）。
    pub effective_at: Option<TsMs>,
    /// 期限。
    pub expires_at: Option<TsMs>,
    /// 撤回時刻（サーバーが付けた。未撤回なら `None`）。
    pub withdrawn_at: Option<TsMs>,
    /// 撤回方法への参照（参照のみ。`ref:` で始まる合成の参照形式）。
    pub withdrawal_method_ref: Option<String>,
    pub status: BasisStatus,
    /// 失効の世代（撤回で進む）。
    pub epoch: u64,
}

/// 削除・制限記録（12 §3.2 の「削除・制限記録」行。対象ID・種類・発効順序・保全理由・処理結果）。
/// 削除の処理そのもの（05）は実装しない。ここは記録の構造と状態だけ。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeletionRestriction {
    pub workspace_id: WorkspaceId,
    /// サーバー採番。
    pub record_id: DeletionRestrictionId,
    /// 対象 ID（参照のみ）。
    pub target_ref: String,
    /// 操作の種類（取消P・根拠失効・削除の利用停止の区別）。
    pub kind: HaltKind,
    /// 記録の状態。
    pub state: HaltState,
    /// 保全の理由への参照（あれば）。
    pub preservation_ref: Option<String>,
    /// 処理結果（合成）。
    pub outcome: String,
    /// 発効順序（サーバー採番）。
    pub effective_seq: u64,
    /// サーバーが付けた記録時刻。
    pub created_at: TsMs,
}

/// 利用停止系の操作の区別（12 §5: 通常取消・integration grant 取消・通信利用の同意撤回/
/// 法的根拠失効は別の事実として記録する）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum HaltKind {
    /// task/operation の通常取消（取消P）。03 の 9 状態には **足さない**
    /// （この実装では遷移を作らない。台帳側は既存のまま）。
    OperationCancellation,
    /// 同意撤回・法的根拠失効（[`DataStore::withdraw_basis_record`] の遷移）。
    BasisInvalidation,
    /// 削除に伴う利用停止（05 の範囲。この実装では遷移を作らない）。
    DeletionSuspension,
}

/// 利用停止系記録の状態（受付と実効の停止は別の事実）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum HaltState {
    /// 受付済み（処理の完了ではない）。
    Received,
    /// 利用停止済み（読取・検索・AI 入力の拒否は QA-GDPR-03 の範囲）。
    Suspended,
}

/// 保存先の登録簿（10 の所在一覧用の型のみ。応答 API は作らない）。
/// [`DataStore`] の各保存部に加え、外の保存部（07 のログ出力先など）も登録できる。
#[derive(Debug, Default)]
pub struct StorageRegistry {
    entries: Vec<StorageLocation>,
}

/// 保存先 1 件。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageLocation {
    /// 保存部の名前。
    pub name: String,
    /// メモリか、ワークスペース内のファイルか。
    pub medium: StorageMedium,
}

/// 保存の媒体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StorageMedium {
    /// メモリのみ（ディスクに書かない）。
    Memory,
    /// ファイル（07 の既存ログは `target/demo-logs/` の下）。
    File,
}

impl StorageLocation {
    /// メモリ保存部の登録情報を作る。
    #[must_use]
    pub fn memory(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            medium: StorageMedium::Memory,
        }
    }

    /// ファイル保存部の登録情報を作る（`name` は置き場所の名前）。
    #[must_use]
    pub fn file(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            medium: StorageMedium::File,
        }
    }
}

impl StorageRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 保存部を登録する（同一 name の再登録も許す。単なる登録簿）。
    pub fn register(&mut self, location: StorageLocation) {
        self.entries.push(location);
    }

    /// 登録済みの保存先一覧。
    #[must_use]
    pub fn locations(&self) -> &[StorageLocation] {
        &self.entries
    }
}

/// メッセージ保存の入力（ID・時刻・seq はサーバー付けのため入力に無い）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewMessage {
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    pub author_id: MemberId,
    pub member_kind: MemberKind,
    pub purpose_id: PurposeId,
    pub body: String,
    pub thread_root: Option<MessageId>,
}

/// 明示メモリ保存の入力（ID・時刻はサーバー付け）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewExplicitMemory {
    pub workspace_id: WorkspaceId,
    pub purpose_id: PurposeId,
    pub content: String,
    pub source_ref: String,
}

/// 根拠記録作成の入力。ID・時刻・status・epoch はサーバー付けのため入力に無い
/// （要求側から指定できない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewBasisRecord {
    pub basis_type: BasisType,
    pub purpose_id: PurposeId,
    pub scope: String,
    pub party_coverage_ref: String,
    pub grantor_ref: String,
    pub representative_authority_ref: Option<String>,
    pub explanation_version: String,
    pub evidence_ref: String,
}

impl NewBasisRecord {
    /// `scope` と参照列（`*_ref`）が合成の参照形式か（本文や人物情報の複製を
    /// 参照欄に入れさせない。[`create_basis_record`](DataStore::create_basis_record)
    /// が保存前に必ずこの検証を通す）。
    fn validate(&self) -> Result<(), StoreError> {
        validate_scope("scope", &self.scope)?;
        validate_ref("party_coverage_ref", &self.party_coverage_ref)?;
        validate_ref("grantor_ref", &self.grantor_ref)?;
        validate_opt_ref(
            "representative_authority_ref",
            self.representative_authority_ref.as_deref(),
        )?;
        validate_ref("evidence_ref", &self.evidence_ref)
    }
}

/// 利用方針作成の入力（version はサーバー付け）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewUsePolicy {
    pub purpose_id: PurposeId,
    pub data_classes: Vec<String>,
    pub source_scope: String,
    pub processors: Vec<String>,
    pub allowed_outputs: Vec<String>,
    pub explanation_version: String,
    pub retention_policy_ref: String,
}

impl NewUsePolicy {
    /// `source_scope` と `retention_policy_ref` が合成の参照形式か
    /// （[`create_use_policy`](DataStore::create_use_policy) が保存前に必ず通す）。
    fn validate(&self) -> Result<(), StoreError> {
        validate_scope("source_scope", &self.source_scope)?;
        validate_ref("retention_policy_ref", &self.retention_policy_ref)
    }
}

/// 削除・制限記録の入力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewDeletionRestriction {
    pub target_ref: String,
    pub kind: HaltKind,
    pub preservation_ref: Option<String>,
    pub outcome: String,
}

/// `POST /v1/workspaces/{workspace_id}/communication-basis-records`（**仮の経路**）の要求。
/// `basis_id`・時刻・`status`・`epoch` 等の名前のフィールドは持たず、未知フィールドは
/// 拒否する（`deny_unknown_fields`）。偽の ID・時刻を送っても保存されない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CreateBasisRecordRequest {
    /// 要求者の合成利用者 ID（引数で渡す。開発リードの仮の決め方 2026-11-10）。
    pub actor: MemberId,
    pub basis_type: BasisType,
    /// P01〜p08 のみ。未設定・不正値は拒否。
    pub purpose_id: PurposeId,
    pub scope: String,
    pub party_coverage_ref: String,
    pub grantor_ref: String,
    pub representative_authority_ref: Option<String>,
    pub explanation_version: String,
    pub evidence_ref: String,
}

impl From<CreateBasisRecordRequest> for NewBasisRecord {
    fn from(req: CreateBasisRecordRequest) -> Self {
        Self {
            basis_type: req.basis_type,
            purpose_id: req.purpose_id,
            scope: req.scope,
            party_coverage_ref: req.party_coverage_ref,
            grantor_ref: req.grantor_ref,
            representative_authority_ref: req.representative_authority_ref,
            explanation_version: req.explanation_version,
            evidence_ref: req.evidence_ref,
        }
    }
}

/// `POST /v1/workspaces/{workspace_id}/communication-basis-records/{basis_id}/withdraw`
/// （**仮の経路**）の要求。撤回は取得と同じ 1 操作（別の難しい手順を課さない）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct WithdrawBasisRecordRequest {
    /// 要求者の合成利用者 ID。
    pub actor: MemberId,
    /// 撤回方法への参照（あれば）。
    pub withdrawal_method_ref: Option<String>,
}

/// `POST /v1/workspaces/{workspace_id}/communication-use-policies`（**仮の経路**）の要求。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CreateUsePolicyRequest {
    /// 要求者の合成利用者 ID。
    pub actor: MemberId,
    /// P01〜p08 のみ。未設定・不正値は拒否。
    pub purpose_id: PurposeId,
    pub data_classes: Vec<String>,
    pub source_scope: String,
    pub processors: Vec<String>,
    pub allowed_outputs: Vec<String>,
    pub explanation_version: String,
    pub retention_policy_ref: String,
}

impl From<CreateUsePolicyRequest> for NewUsePolicy {
    fn from(req: CreateUsePolicyRequest) -> Self {
        Self {
            purpose_id: req.purpose_id,
            data_classes: req.data_classes,
            source_scope: req.source_scope,
            processors: req.processors,
            allowed_outputs: req.allowed_outputs,
            explanation_version: req.explanation_version,
            retention_policy_ref: req.retention_policy_ref,
        }
    }
}

/// 保存部の失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreError {
    /// 対象の記録が無い。
    NotFound,
    /// 参照列（`*_ref`）または `scope`/`source_scope` の値が合成の参照形式でない
    /// （本文や人物情報など、参照以外の複製を入れさせない検証。どの列かは `column`）。
    InvalidReference {
        /// 検証に失敗した列の名前（ログ・デバッグ用。HTTP 応答には出さない）。
        column: &'static str,
    },
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => f.write_str("target record not found"),
            Self::InvalidReference { column } => {
                write!(f, "column {column} is not a synthetic reference form")
            }
        }
    }
}

impl std::error::Error for StoreError {}

/// 合成の参照列（`*_ref`）の形式: `ref:` の後に ASCII 小文字・数字・`-`・`_` だけが
/// 1 文字以上並び、全体（`ref:` を含む）が [`MAX_REF_BYTES`] バイト以内。
/// この形式は **合成データ用の仮の規則**（12 §3.2 は論理単位のみを定め、確定 DDL
/// ではない）。大文字・空白・記号・多バイト文字を含む値＝本文や人物情報のような
/// 自由文は、参照列への複製として **受け付けない**。
const REF_PREFIX: &str = "ref:";

/// 参照列・scope の長さの上限（バイト）。参照識別子には十分で、本文の複製には短い。
const MAX_REF_BYTES: usize = 64;

fn is_allowed_ref_char(b: u8) -> bool {
    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'
}

/// 参照列（`*_ref`）の値が合成の参照形式か。
fn is_synthetic_ref(value: &str) -> bool {
    let Some(rest) = value.strip_prefix(REF_PREFIX) else {
        return false;
    };
    !rest.is_empty() && value.len() <= MAX_REF_BYTES && rest.bytes().all(is_allowed_ref_char)
}

/// scope / source_scope の値が合成の参照形式か（`ref:` 無しの識別子形式。
/// `ch-synthetic-02` 等、参照先の種別 + 識別子）。
fn is_synthetic_scope(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_REF_BYTES && value.bytes().all(is_allowed_ref_char)
}

fn validate_ref(column: &'static str, value: &str) -> Result<(), StoreError> {
    if is_synthetic_ref(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidReference { column })
    }
}

fn validate_opt_ref(column: &'static str, value: Option<&str>) -> Result<(), StoreError> {
    match value {
        Some(v) => validate_ref(column, v),
        None => Ok(()),
    }
}

fn validate_scope(column: &'static str, value: &str) -> Result<(), StoreError> {
    if is_synthetic_scope(value) {
        Ok(())
    } else {
        Err(StoreError::InvalidReference { column })
    }
}

/// 合成データ用のメモリ内保存部。ディスクには書かない。
#[derive(Debug, Default)]
pub struct DataStore {
    workspaces: Vec<StoredWorkspace>,
    channels: Vec<Channel>,
    messages: Vec<StoredMessage>,
    explicit_memories: Vec<ExplicitMemory>,
    basis_records: Vec<BasisRecord>,
    use_policies: Vec<UsePolicy>,
    deletion_restrictions: Vec<DeletionRestriction>,
    registry: StorageRegistry,
    next_message: u64,
    next_memory: u64,
    next_basis: u64,
    next_restriction: u64,
}

impl DataStore {
    #[must_use]
    pub fn new() -> Self {
        // 各保存部が自分を登録簿に登録する（10 の所在一覧の種。API は未実装）。
        let mut registry = StorageRegistry::new();
        for name in [
            "workspaces",
            "channels",
            "messages",
            "explicit_memories",
            "communication_basis_records",
            "communication_use_policies",
            "deletion_restrictions",
        ] {
            registry.register(StorageLocation::memory(name));
        }
        Self {
            registry,
            ..Self::default()
        }
    }

    /// workspace を登録する（冪等。既にあれば何もしない）。時刻はサーバー付け。
    pub fn ensure_workspace(&mut self, workspace_id: &WorkspaceId, now: TsMs) -> StoredWorkspace {
        if let Some(w) = self
            .workspaces
            .iter()
            .find(|w| &w.workspace_id == workspace_id)
        {
            return w.clone();
        }
        let w = StoredWorkspace {
            workspace_id: workspace_id.clone(),
            created_at: now,
        };
        self.workspaces.push(w.clone());
        w
    }

    /// 登録済みの workspace 一覧。
    #[must_use]
    pub fn workspaces(&self) -> &[StoredWorkspace] {
        &self.workspaces
    }

    /// channel を保存部に加える。
    pub fn add_channel(&mut self, channel: Channel) {
        self.channels.push(channel);
    }

    /// workspace の channel 一覧。
    #[must_use]
    pub fn channels(&self, workspace_id: &WorkspaceId) -> Vec<&Channel> {
        self.channels
            .iter()
            .filter(|c| &c.workspace_id == workspace_id)
            .collect()
    }

    /// メッセージを 1 件保存する（`client_msg_id` は持たない）。ID・seq・時刻はサーバー付け。
    pub fn add_message(&mut self, input: NewMessage, now: TsMs) -> StoredMessage {
        self.next_message += 1;
        let seq = u64::try_from(
            self.messages
                .iter()
                .filter(|m| {
                    m.workspace_id == input.workspace_id && m.channel_id == input.channel_id
                })
                .count(),
        )
        .unwrap_or(u64::MAX)
            + 1;
        let message = StoredMessage {
            message_id: MessageId(format!("msg-{:08}", self.next_message)),
            workspace_id: input.workspace_id,
            channel_id: input.channel_id,
            seq,
            author_id: input.author_id,
            member_kind: input.member_kind,
            purpose_id: input.purpose_id,
            body: input.body,
            thread_root: input.thread_root,
            created_at: now,
        };
        self.messages.push(message.clone());
        message
    }

    /// workspace のメッセージ一覧。
    #[must_use]
    pub fn messages(&self, workspace_id: &WorkspaceId) -> Vec<&StoredMessage> {
        self.messages
            .iter()
            .filter(|m| &m.workspace_id == workspace_id)
            .collect()
    }

    /// 明示メモリを 1 件保存する。ID・時刻はサーバー付け。
    pub fn add_explicit_memory(&mut self, input: NewExplicitMemory, now: TsMs) -> ExplicitMemory {
        self.next_memory += 1;
        let memory = ExplicitMemory {
            memory_id: MemoryId(format!("mem-{:08}", self.next_memory)),
            workspace_id: input.workspace_id,
            purpose_id: input.purpose_id,
            content: input.content,
            source_ref: input.source_ref,
            created_at: now,
        };
        self.explicit_memories.push(memory.clone());
        memory
    }

    /// workspace の明示メモリ一覧。
    #[must_use]
    pub fn explicit_memories(&self, workspace_id: &WorkspaceId) -> Vec<&ExplicitMemory> {
        self.explicit_memories
            .iter()
            .filter(|m| &m.workspace_id == workspace_id)
            .collect()
    }

    /// 根拠記録を 1 件作る。ID・版・時刻・status（`pending`）・epoch はサーバー付け
    /// （要求からは指定できない）。作成直後は未確定（[`BasisStatus::Pending`]）で、
    /// 同意根拠として使える状態にはしない。`scope` と参照列（`*_ref`）は保存前に
    /// 合成の参照形式の検証を通り、形式に合わない値（本文や人物情報の自由文)は
    /// [`StoreError::InvalidReference`] として保存しない（参照列は参照だけにする）。
    ///
    /// # Errors
    /// `scope` または参照列が合成の参照形式でないとき
    /// [`StoreError::InvalidReference`]。
    pub fn create_basis_record(
        &mut self,
        workspace_id: &WorkspaceId,
        input: NewBasisRecord,
        now: TsMs,
    ) -> Result<BasisRecord, StoreError> {
        input.validate()?;
        self.next_basis += 1;
        let record = BasisRecord {
            workspace_id: workspace_id.clone(),
            basis_id: BasisId(format!("bas-{:08}", self.next_basis)),
            basis_version: 1,
            basis_type: input.basis_type,
            purpose_id: input.purpose_id,
            scope: input.scope,
            party_coverage_ref: input.party_coverage_ref,
            grantor_ref: input.grantor_ref,
            representative_authority_ref: input.representative_authority_ref,
            explanation_version: input.explanation_version,
            evidence_ref: input.evidence_ref,
            collected_at: now,
            effective_at: None,
            expires_at: None,
            withdrawn_at: None,
            withdrawal_method_ref: None,
            status: BasisStatus::Pending,
            epoch: 1,
        };
        self.basis_records.push(record.clone());
        Ok(record)
    }

    /// workspace の根拠記録一覧。
    #[must_use]
    pub fn basis_records(&self, workspace_id: &WorkspaceId) -> Vec<&BasisRecord> {
        self.basis_records
            .iter()
            .filter(|r| &r.workspace_id == workspace_id)
            .collect()
    }

    /// 根拠記録を 1 件読む。
    #[must_use]
    pub fn basis_record(
        &self,
        workspace_id: &WorkspaceId,
        basis_id: &BasisId,
    ) -> Option<&BasisRecord> {
        self.basis_records
            .iter()
            .find(|r| &r.workspace_id == workspace_id && &r.basis_id == basis_id)
    }

    /// 撤回する（取得と同じ 1 操作）。対象の `epoch` を 1 進め、`withdrawn_at`（サーバー
    /// 付け）と status を更新する。撤回後の読取・検索・AI 入力の拒否そのものは
    /// QA-GDPR-03 の範囲（ここでは実装しない）。撤回済みの記録の再撤回も受け付ける
    /// （epoch をさらに進める。取得より難しい別手順にはしない）。`withdrawal_method_ref`
    /// は合成の参照形式だけを受け付ける（形式に合わない値は保存しない）。
    ///
    /// # Errors
    /// 対象の記録が見つからないとき [`StoreError::NotFound`]。
    /// `withdrawal_method_ref` が合成の参照形式でないとき
    /// [`StoreError::InvalidReference`]（この場合状態を変えない）。
    pub fn withdraw_basis_record(
        &mut self,
        workspace_id: &WorkspaceId,
        basis_id: &BasisId,
        withdrawal_method_ref: Option<String>,
        now: TsMs,
    ) -> Result<BasisRecord, StoreError> {
        validate_opt_ref("withdrawal_method_ref", withdrawal_method_ref.as_deref())?;
        let Some(record) = self
            .basis_records
            .iter_mut()
            .find(|r| &r.workspace_id == workspace_id && &r.basis_id == basis_id)
        else {
            return Err(StoreError::NotFound);
        };
        record.epoch += 1;
        record.withdrawn_at = Some(now);
        if withdrawal_method_ref.is_some() {
            record.withdrawal_method_ref = withdrawal_method_ref;
        }
        record.status = BasisStatus::Withdrawn;
        Ok(record.clone())
    }

    /// 利用方針を 1 件作る（version はサーバー付け。同じ workspace × purpose で
    /// 1 から振る）。変更前の版は証拠参照として削除せず残す。`source_scope` と
    /// `retention_policy_ref` は保存前に合成の参照形式の検証を通り、形式に合わない
    /// 値は [`StoreError::InvalidReference`] として保存しない。
    ///
    /// # Errors
    /// `source_scope` または `retention_policy_ref` が合成の参照形式でないとき
    /// [`StoreError::InvalidReference`]。
    pub fn create_use_policy(
        &mut self,
        workspace_id: &WorkspaceId,
        input: NewUsePolicy,
        now: TsMs,
    ) -> Result<UsePolicy, StoreError> {
        input.validate()?;
        let version = u64::try_from(
            self.use_policies
                .iter()
                .filter(|p| &p.workspace_id == workspace_id && p.purpose_id == input.purpose_id)
                .count(),
        )
        .unwrap_or(u64::MAX)
            + 1;
        let policy = UsePolicy {
            workspace_id: workspace_id.clone(),
            purpose_id: input.purpose_id,
            version,
            data_classes: input.data_classes,
            source_scope: input.source_scope,
            processors: input.processors,
            allowed_outputs: input.allowed_outputs,
            explanation_version: input.explanation_version,
            retention_policy_ref: input.retention_policy_ref,
            created_at: now,
        };
        self.use_policies.push(policy.clone());
        Ok(policy)
    }

    /// workspace の利用方針一覧（変更前の版を含む）。
    #[must_use]
    pub fn use_policies(&self, workspace_id: &WorkspaceId) -> Vec<&UsePolicy> {
        self.use_policies
            .iter()
            .filter(|p| &p.workspace_id == workspace_id)
            .collect()
    }

    /// 削除・制限記録を 1 件残す（削除の処理そのものは 05 の範囲で実装しない）。
    pub fn record_deletion_restriction(
        &mut self,
        workspace_id: &WorkspaceId,
        input: NewDeletionRestriction,
        now: TsMs,
    ) -> DeletionRestriction {
        self.next_restriction += 1;
        let record = DeletionRestriction {
            workspace_id: workspace_id.clone(),
            record_id: DeletionRestrictionId(format!("del-{:08}", self.next_restriction)),
            target_ref: input.target_ref,
            kind: input.kind,
            state: HaltState::Received,
            preservation_ref: input.preservation_ref,
            outcome: input.outcome,
            effective_seq: self.next_restriction,
            created_at: now,
        };
        self.deletion_restrictions.push(record.clone());
        record
    }

    /// workspace の削除・制限記録一覧。
    #[must_use]
    pub fn deletion_restrictions(&self, workspace_id: &WorkspaceId) -> Vec<&DeletionRestriction> {
        self.deletion_restrictions
            .iter()
            .filter(|r| &r.workspace_id == workspace_id)
            .collect()
    }

    /// 目的の機能が有効か（12 §3.3: 根拠未確定の機能は無効）。
    /// 有効（Active）で期間内の根拠記録があるときだけ `true`。**この実装には
    /// Active な根拠記録を作る経路が無い** ため、同意を使う機能（P03/P04/P06）は
    /// 常にオフ（初期オフ）。`pending`・撤回済み・期限切れは根拠にしない。
    #[must_use]
    pub fn is_purpose_enabled(
        &self,
        workspace_id: &WorkspaceId,
        purpose: PurposeId,
        now: TsMs,
    ) -> bool {
        self.basis_records
            .iter()
            .any(|r| &r.workspace_id == workspace_id && r.is_valid_consent_basis_at(purpose, now))
    }

    /// 外の保存部（07 のログ出力先など）を登録簿に登録する。
    pub fn register_location(&mut self, location: StorageLocation) {
        self.registry.register(location);
    }

    /// 登録済みの保存先一覧（10 の所在一覧 API は未実装）。
    #[must_use]
    pub fn registered_locations(&self) -> &[StorageLocation] {
        self.registry.locations()
    }
}

impl BasisRecord {
    /// 同意根拠として使えるか（12 §3.2: status・期間・目的で判定する）。
    /// `pending`・撤回済み・期限切れ・未発効は `false`。
    #[must_use]
    pub fn is_valid_consent_basis_at(&self, purpose: PurposeId, now: TsMs) -> bool {
        self.purpose_id == purpose
            && self.status == BasisStatus::Active
            && self.effective_at.is_some_and(|t| t <= now)
            && self.expires_at.is_none_or(|t| now < t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 本文・コメント用の合成 canary（07 の検証と同じ値。実在の値ではない）。
    const CANARY_BODY: &str = "PELWAY-CANARY-BODY-7Q2M9X";
    /// `client_msg_id` 用の合成 canary（UUIDv4 の形の合成値）。
    const CANARY_MSG_ID: &str = "3f6c2a9e-8b1d-4c7a-9e2f-5a1b7c3d9e04";

    fn ws() -> WorkspaceId {
        WorkspaceId::from("ws-synthetic-gdpr02")
    }

    fn new_basis(purpose: PurposeId) -> NewBasisRecord {
        NewBasisRecord {
            basis_type: BasisType::SyntheticConsent,
            purpose_id: purpose,
            scope: "ch-synthetic-02".to_owned(),
            party_coverage_ref: "ref:party-coverage-1".to_owned(),
            grantor_ref: "ref:grantor-1".to_owned(),
            representative_authority_ref: None,
            explanation_version: "expl-v1".to_owned(),
            evidence_ref: "ref:evidence-1".to_owned(),
        }
    }

    fn new_policy(explanation: &str) -> NewUsePolicy {
        NewUsePolicy {
            purpose_id: PurposeId::P04,
            data_classes: vec!["message-body".to_owned()],
            source_scope: "ch-synthetic-02".to_owned(),
            processors: vec!["processor-synthetic-1".to_owned()],
            allowed_outputs: vec!["summary".to_owned()],
            explanation_version: explanation.to_owned(),
            retention_policy_ref: "ref:retention-1".to_owned(),
        }
    }

    fn new_message(body: String) -> NewMessage {
        NewMessage {
            workspace_id: ws(),
            channel_id: ChannelId::from("ch-synthetic-02"),
            author_id: MemberId::from("m-human-gdpr02"),
            member_kind: MemberKind::Human,
            purpose_id: PurposeId::P01,
            body,
            thread_root: None,
        }
    }

    // テスト1: 合成 workspace で、目的 P01〜P08 それぞれに根拠レコードが作れて読める。
    #[test]
    fn basis_records_are_created_and_read_for_all_purposes() {
        let mut store = DataStore::new();
        for (i, purpose) in PurposeId::ALL.iter().enumerate() {
            let rec = store
                .create_basis_record(&ws(), new_basis(*purpose), 1000 + i as TsMs)
                .expect("正当な参照形式で作成できる");
            assert_eq!(rec.purpose_id, *purpose);
            assert_eq!(rec.basis_id.as_str(), format!("bas-{:08}", i + 1));
            assert_eq!(rec.status, BasisStatus::Pending, "作成直後は未確定");
        }
        let all = store.basis_records(&ws());
        assert_eq!(all.len(), 8, "8 目的ぶん保存されている: {:?}", all);
        for purpose in PurposeId::ALL {
            assert!(
                all.iter().any(|r| r.purpose_id == purpose),
                "{purpose:?} が読めない"
            );
        }
        // 1 件読み
        let one = store.basis_record(&ws(), &BasisId::from("bas-00000001"));
        assert!(one.is_some());
    }

    // テスト2: 本文 canary をメッセージ・明示メモリに入れても、根拠レコードと
    // 利用方針の直列化に出ない。陽性対照（canary を含む文字列は同じ検査で見つかる）も書く。
    #[test]
    fn basis_and_policy_serialization_avoid_message_and_memory_content() {
        let mut store = DataStore::new();
        store.ensure_workspace(&ws(), 1);
        store.add_channel(Channel {
            workspace_id: ws(),
            channel_id: ChannelId::from("ch-synthetic-02"),
            name: "合成チャンネル".to_owned(),
            members: Vec::new(),
        });
        store.add_message(new_message(format!("合成本文 {CANARY_BODY}")), 10);
        store.add_explicit_memory(
            NewExplicitMemory {
                workspace_id: ws(),
                purpose_id: PurposeId::P04,
                content: format!("合成メモリ {CANARY_BODY}"),
                source_ref: "ref:msg-00000001".to_owned(),
            },
            20,
        );

        let rec = store
            .create_basis_record(&ws(), new_basis(PurposeId::P03), 30)
            .expect("create basis");
        let pol = store
            .create_use_policy(&ws(), new_policy("expl-v1"), 40)
            .expect("create policy");

        let rec_json = serde_json::to_string(&rec).expect("basis record serializes");
        let pol_json = serde_json::to_string(&pol).expect("use policy serializes");
        assert!(
            !rec_json.contains(CANARY_BODY),
            "根拠レコードの直列化に本文 canary がある: {rec_json}"
        );
        assert!(
            !pol_json.contains(CANARY_BODY),
            "利用方針の直列化に本文 canary がある: {pol_json}"
        );

        // 保存部のメッセージ・明示メモリ自体は本文を持つ（これ自体は正しい）。
        // ただし client_msg_id は保存しない（キーも値も出ない）。
        let msg_json = serde_json::to_string(&store.messages(&ws())[0]).expect("msg");
        let mem_json = serde_json::to_string(&store.explicit_memories(&ws())[0]).expect("memory");
        assert!(msg_json.contains(CANARY_BODY));
        assert!(mem_json.contains(CANARY_BODY));
        for text in [&msg_json, &mem_json] {
            assert!(!text.contains("client_msg_id"), "{text}");
            assert!(!text.contains(CANARY_MSG_ID), "{text}");
        }

        // 陽性対照: canary を含む文字列は同じ検査（contains）で見つかる。
        let positive = format!("x {CANARY_BODY} y {CANARY_MSG_ID} z");
        assert_eq!(positive.matches(CANARY_BODY).count(), 1);
        assert_eq!(positive.matches(CANARY_MSG_ID).count(), 1);
        // canary の部分一致では拾わない（canary 全体が無いことの検査であること）。
        assert!(!"PELWAY-CANARY-BODY-7Q2M9".contains(CANARY_BODY));
    }

    // テスト3: 偽の時刻・ID を送っても、保存された正本はサーバーの値になる
    // （contract 側: 入力型に ID・時刻の欄が無く、サーバーが付けた値だけが入る）。
    #[test]
    fn server_assigns_ids_timestamps_and_epoch_at_creation() {
        let mut store = DataStore::new();
        let rec = store
            .create_basis_record(&ws(), new_basis(PurposeId::P06), 12_345)
            .expect("create basis");
        assert_eq!(rec.basis_id.as_str(), "bas-00000001", "ID はサーバー採番");
        assert_eq!(rec.basis_version, 1, "版はサーバー採番");
        assert_eq!(rec.epoch, 1, "epoch はサーバー採番");
        assert_eq!(rec.collected_at, 12_345, "時刻はサーバー渡しの値");
        assert_eq!(rec.effective_at, None, "未確定のうちは発効無し");
        assert_eq!(rec.withdrawn_at, None);
        assert_eq!(rec.status, BasisStatus::Pending);
        // 2 件目は別のサーバー ID。
        let rec2 = store
            .create_basis_record(&ws(), new_basis(PurposeId::P06), 12_500)
            .expect("create basis 2");
        assert_eq!(rec2.basis_id.as_str(), "bas-00000002");
    }

    // テスト5: 同意を使う機能が初期オフであること（根拠なし、または未確定の
    // 根拠だけでは有効にならない）。
    #[test]
    fn consent_using_purposes_stay_disabled_without_valid_basis() {
        let mut store = DataStore::new();
        for purpose in [PurposeId::P03, PurposeId::P04, PurposeId::P06] {
            assert!(
                !store.is_purpose_enabled(&ws(), purpose, 100),
                "{purpose:?} が根拠なしで有効になっている"
            );
        }
        // 未確定（pending）の根拠記録を作っても有効にしない。
        for purpose in [PurposeId::P03, PurposeId::P04, PurposeId::P06] {
            store
                .create_basis_record(&ws(), new_basis(purpose), 100)
                .expect("create basis");
        }
        for purpose in PurposeId::ALL {
            assert!(
                !store.is_purpose_enabled(&ws(), purpose, 200),
                "{purpose:?} が未確定根拠で有効になっている"
            );
        }
        // 撤回済みでも有効にしない（テスト6 と同じ操作）。
        let rec = store.basis_records(&ws())[0].clone();
        store
            .withdraw_basis_record(&ws(), &rec.basis_id, None, 300)
            .expect("withdraw");
        assert!(!store.is_purpose_enabled(&ws(), rec.purpose_id, 400));
    }

    // テスト6: 撤回が取得と同じ1操作で受け付けられ、epoch が進むこと。
    #[test]
    fn withdrawal_updates_epoch_status_and_time_in_one_call() {
        let mut store = DataStore::new();
        let rec = store
            .create_basis_record(&ws(), new_basis(PurposeId::P03), 1000)
            .expect("create basis");
        assert_eq!(rec.epoch, 1);
        assert_eq!(rec.status, BasisStatus::Pending);

        let withdrawn = store
            .withdraw_basis_record(
                &ws(),
                &rec.basis_id,
                Some("ref:withdraw-method-1".to_owned()),
                2000,
            )
            .expect("撤回は1操作で受け付ける");
        assert_eq!(withdrawn.epoch, 2, "撤回で epoch が進む");
        assert_eq!(withdrawn.status, BasisStatus::Withdrawn);
        assert_eq!(withdrawn.withdrawn_at, Some(2000), "撤回時刻はサーバー付け");
        assert_eq!(
            withdrawn.withdrawal_method_ref.as_deref(),
            Some("ref:withdraw-method-1")
        );

        // 保存部の中の記録も同じ状態（読み直しで確認）。
        let again = store.basis_record(&ws(), &rec.basis_id).expect("record");
        assert_eq!(again.epoch, 2);
        assert_eq!(again.status, BasisStatus::Withdrawn);

        // 対象が無ければエラー（状態を変えない）。
        assert_eq!(
            store.withdraw_basis_record(&ws(), &BasisId::from("bas-99999999"), None, 3000),
            Err(StoreError::NotFound)
        );
    }

    // テスト7: purpose_id の未設定・不正値の書き込みが拒否されること。
    #[test]
    fn purpose_id_invalid_or_missing_values_are_rejected_at_deserialization() {
        let valid = json!({
            "actor": "m-human-gdpr02",
            "basis_type": "synthetic_consent",
            "purpose_id": "p03",
            "scope": "ch-synthetic-02",
            "party_coverage_ref": "ref:1",
            "grantor_ref": "ref:2",
            "explanation_version": "expl-v1",
            "evidence_ref": "ref:3",
        });
        assert!(
            serde_json::from_value::<CreateBasisRecordRequest>(valid.clone()).is_ok(),
            "前提: p01〜p08 は受理"
        );
        // 不正値（P08 以外・大文字・空・別形式）。
        for bad in ["p09", "p0", "p1", "P03", "purpose-3", "", "x13"] {
            let mut v = valid.clone();
            v["purpose_id"] = json!(bad);
            assert!(
                serde_json::from_value::<CreateBasisRecordRequest>(v).is_err(),
                "根拠記録: {bad:?} が受理されている"
            );
        }
        // 未設定（項目ごと無い）。
        let mut missing = valid.clone();
        missing
            .as_object_mut()
            .expect("object")
            .remove("purpose_id");
        assert!(
            serde_json::from_value::<CreateBasisRecordRequest>(missing).is_err(),
            "根拠記録: purpose_id 未設定が受理されている"
        );

        // 利用方針も同じ検査。
        let policy = json!({
            "actor": "m-human-gdpr02",
            "purpose_id": "p04",
            "data_classes": ["message-body"],
            "source_scope": "ch-synthetic-02",
            "processors": ["processor-synthetic-1"],
            "allowed_outputs": ["summary"],
            "explanation_version": "expl-v1",
            "retention_policy_ref": "ref:retention-1",
        });
        assert!(serde_json::from_value::<CreateUsePolicyRequest>(policy.clone()).is_ok());
        let mut bad_policy = policy.clone();
        bad_policy["purpose_id"] = json!("p99");
        assert!(serde_json::from_value::<CreateUsePolicyRequest>(bad_policy).is_err());
        let mut missing_policy = policy;
        missing_policy
            .as_object_mut()
            .expect("object")
            .remove("purpose_id");
        assert!(serde_json::from_value::<CreateUsePolicyRequest>(missing_policy).is_err());

        // 保存部の内部 API（NewMessage 等）は purpose_id が必須（非 Option）のため、
        // 「未設定・不正値」は型レベルで不可能（値を作れない）。上の直列化検査と
        // 併せて、未設定・不正値の書き込み経路は無い。
    }

    // 利用方針の変更前の版は証拠として残る。
    #[test]
    fn use_policy_keeps_previous_versions() {
        let mut store = DataStore::new();
        let p1 = store
            .create_use_policy(&ws(), new_policy("expl-v1"), 10)
            .expect("create policy");
        assert_eq!(p1.version, 1);
        let p2 = store
            .create_use_policy(&ws(), new_policy("expl-v2"), 20)
            .expect("create policy 2");
        assert_eq!(p2.version, 2, "版はサーバーが進める");
        let all = store.use_policies(&ws());
        assert_eq!(all.len(), 2, "変更前の版も残る");
        assert!(all.iter().any(|p| p.explanation_version == "expl-v1"));
    }

    // 削除・制限記録は取消P・根拠失効・削除の利用停止を別の状態として持てる。
    #[test]
    fn deletion_restrictions_hold_kind_and_state_separately() {
        let mut store = DataStore::new();
        let r = store.record_deletion_restriction(
            &ws(),
            NewDeletionRestriction {
                target_ref: "ch:ch-synthetic-02".to_owned(),
                kind: HaltKind::DeletionSuspension,
                preservation_ref: None,
                outcome: "accepted".to_owned(),
            },
            10,
        );
        assert_eq!(r.kind, HaltKind::DeletionSuspension);
        assert_eq!(r.state, HaltState::Received, "受付と実効停止は別の状態");
        assert_eq!(r.effective_seq, 1);
        // 3 種類は別の値として区別できる（03 の 9 状態には足していない）。
        assert_ne!(HaltKind::OperationCancellation, HaltKind::BasisInvalidation);
        assert_ne!(HaltKind::BasisInvalidation, HaltKind::DeletionSuspension);
        assert_eq!(store.deletion_restrictions(&ws()).len(), 1);
    }

    // テスト8(contract 側): 参照列・scope は合成の参照形式だけを受け付ける。
    // 本文 canary(単体・混在)や自由文・空・長すぎは InvalidReference として保存しない。
    #[test]
    fn reference_columns_accept_only_synthetic_reference_forms() {
        let mut store = DataStore::new();
        // scope の不正値: canary 単体・canary 混在・空・大文字・空白・多バイト・長すぎ。
        let bad_scopes = [
            CANARY_BODY.to_owned(),
            format!("ch-synthetic-02 {CANARY_BODY}"),
            String::new(),
            "CH-synthetic-02".to_owned(),
            "ch synthetic".to_owned(),
            "チャンネル".to_owned(),
            "ref:x".to_owned(), // scope に参照プレフィックスは付けさせない
            format!("{}x", "a".repeat(MAX_REF_BYTES)),
        ];
        for bad in &bad_scopes {
            let mut input = new_basis(PurposeId::P03);
            input.scope = bad.clone();
            assert_eq!(
                store.create_basis_record(&ws(), input.clone(), 1),
                Err(StoreError::InvalidReference { column: "scope" }),
                "scope が受理されている: {bad:?}"
            );
        }
        // 参照列も同じ(canary・`ref:` 無し・空・混在・大文字)。
        for bad in [
            CANARY_BODY.to_owned(),
            format!("ref:evidence-1 {CANARY_BODY}"),
            String::new(),
            "evidence-1".to_owned(),
            "REF:evidence-1".to_owned(),
            "ref:にほんご".to_owned(),
        ] {
            let mut input = new_basis(PurposeId::P03);
            input.evidence_ref = bad.clone();
            assert_eq!(
                store.create_basis_record(&ws(), input.clone(), 1),
                Err(StoreError::InvalidReference {
                    column: "evidence_ref"
                }),
                "evidence_ref が受理されている: {bad:?}"
            );
            let mut input = new_basis(PurposeId::P03);
            input.party_coverage_ref = bad.clone();
            assert_eq!(
                store.create_basis_record(&ws(), input.clone(), 1),
                Err(StoreError::InvalidReference {
                    column: "party_coverage_ref"
                }),
                "party_coverage_ref が受理されている: {bad:?}"
            );
            let mut input = new_basis(PurposeId::P03);
            input.grantor_ref = bad.clone();
            assert_eq!(
                store.create_basis_record(&ws(), input.clone(), 1),
                Err(StoreError::InvalidReference {
                    column: "grantor_ref"
                }),
                "grantor_ref が受理されている: {bad:?}"
            );
            let mut input = new_basis(PurposeId::P03);
            input.representative_authority_ref = Some(bad.clone());
            assert_eq!(
                store.create_basis_record(&ws(), input.clone(), 1),
                Err(StoreError::InvalidReference {
                    column: "representative_authority_ref"
                }),
                "representative_authority_ref が受理されている: {bad:?}"
            );
        }
        // representative_authority_ref = None は正当(必須ではない)。
        store
            .create_basis_record(&ws(), new_basis(PurposeId::P03), 1)
            .expect("None は正当");
        // 不正値は 1 件も保存されていない。
        assert_eq!(store.basis_records(&ws()).len(), 1);
        // 正当な形式の参照列はそのまま保存される(参照だけが残る)。
        let (saved_scope, saved_evidence, basis_id) = {
            let rec = &store.basis_records(&ws())[0];
            (
                rec.scope.clone(),
                rec.evidence_ref.clone(),
                rec.basis_id.clone(),
            )
        };
        assert_eq!(saved_scope, "ch-synthetic-02");
        assert_eq!(saved_evidence, "ref:evidence-1");

        // 利用方針: source_scope / retention_policy_ref も同じ検査。
        for bad in [
            CANARY_BODY.to_owned(),
            format!("ch-synthetic-02 {CANARY_BODY}"),
            "ref:retention-1 x!".to_owned(),
        ] {
            let mut policy = new_policy("expl-v1");
            policy.source_scope = bad.clone();
            assert_eq!(
                store.create_use_policy(&ws(), policy.clone(), 2),
                Err(StoreError::InvalidReference {
                    column: "source_scope"
                }),
                "source_scope が受理されている: {bad:?}"
            );
            let mut policy = new_policy("expl-v1");
            policy.retention_policy_ref = bad.clone();
            assert_eq!(
                store.create_use_policy(&ws(), policy.clone(), 2),
                Err(StoreError::InvalidReference {
                    column: "retention_policy_ref"
                }),
                "retention_policy_ref が受理されている: {bad:?}"
            );
        }
        assert!(store.use_policies(&ws()).is_empty());

        // 撤回: withdrawal_method_ref に canary は InvalidReference(状態を変えない)。
        assert_eq!(
            store.withdraw_basis_record(
                &ws(),
                &basis_id,
                Some(format!("ref:withdraw-method-1 {CANARY_BODY}")),
                3
            ),
            Err(StoreError::InvalidReference {
                column: "withdrawal_method_ref"
            })
        );
        let unchanged = store.basis_record(&ws(), &basis_id).expect("record");
        assert_eq!(unchanged.epoch, 1, "検証失敗で epoch は進まない");
        assert_eq!(unchanged.status, BasisStatus::Pending);

        // 陽性対照: canary を含む文字列は同じ直列化検査で見つかる。
        // (canary 入り参照が保存されることがあれば直列化に出る、の対照。
        //   上の検査で保存は拒否されるため、ここでは文字列検査の有効性だけ示す。)
        let positive = serde_json::to_string(&json!({ "scope": CANARY_BODY })).expect("json");
        assert_eq!(positive.matches(CANARY_BODY).count(), 1);
    }

    // 登録簿: 各保存部が自分を登録し、外の保存部（07 のログ出力先など）も登録できる。
    #[test]
    fn store_registers_collections_and_accepts_external_locations() {
        let mut store = DataStore::new();
        let names: Vec<&str> = store
            .registered_locations()
            .iter()
            .map(|l| l.name.as_str())
            .collect();
        for want in [
            "workspaces",
            "channels",
            "messages",
            "explicit_memories",
            "communication_basis_records",
            "communication_use_policies",
            "deletion_restrictions",
        ] {
            assert!(names.contains(&want), "{want} が未登録: {names:?}");
        }
        assert!(
            store
                .registered_locations()
                .iter()
                .all(|l| l.medium == StorageMedium::Memory),
            "保存部はすべてメモリ"
        );
        // 外の保存部（例: 07 のログ出力先）の登録。
        store.register_location(StorageLocation::file("target/demo-logs"));
        assert!(
            store
                .registered_locations()
                .iter()
                .any(|l| l.name == "target/demo-logs" && l.medium == StorageMedium::File),
            "外部保存部が登録できない"
        );
    }
}
