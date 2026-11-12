# 入口制御・常時表示モック（提案）· 2026-11-09

**状態：提案。法務条件付き承認（2026-11-09・開発リード経由）。実装・Git・公開・支出は含まない。**  
Asia/Tokyo · Pelway UI担当

出典：`demo-screen-list-v0.1.md` §2.2 / §3.4 / §4 / §5 #4 #5 #11。  
トークン・`final/`・既存 addenda・export-i18n の ftl は**触っていない**。本フォルダの ftl は後で本流へマージする前提の**新規 ID のみ**。

## 置き場所

`/workspace/pelway-design-system/ui-pro-review/mocks/2026-11-09-entry-points/`

## ファイル

| ファイル | 内容 |
|---|---|
| `native-dp-ja.html` / `_ja_dark` / `_en` | ネイティブ版データ保護パネル（残す3項目＋静的注記） |
| `sidebar-ja-vs-en.html` | S01 サイドバー日英比較 |
| `mobile-simulated-ja.html` / `_en` | 360px：S00バナーと S04「送信済み（模擬）」の折り返し（省略なし） |
| `ai-badge-placement-ja.html` / `_ja_dark` / `_en` | メッセージ・プロフィール・メンション候補の AI バッジ |
| `entry-points.ja.ftl` / `entry-points.en.ftl` | 新規 Fluent ID |
| `*.png` | 全文スクショ |
| `README.md` | 本ファイル |

## 法務の条件（2026-11-09・開発リード経由）

4文言（`native-dp-web-only-note` と S00 の3つ）は**条件付きで使える**。条件：

1. **「（模擬）」/ "(simulated)" / "simulated responses"**（`state-sent-simulated` と `banner-demo-mock`）は、狭い画面でも通知でも **省略・三点リーダー・脱落を禁止**。`text-overflow: ellipsis` を使わない。狭い幅では最大2行まで折り返す。
2. **AIバッジ**（見える文字は「AI」/"AI" = `label-ai-member`）  
   (1) **すべてのメッセージ**・プロフィール・メンション候補で、名前の直後に出す（メンバー一覧だけではない）。  
   (2) 利用者・管理者の設定で **隠したり消したりできない**（トグルは存在しない。実装者向け：設定画面に項目を作らない）。  
   (3) 読み上げは 「AIメンバー」/"AI member"（`label-ai-member-a11y`）。見える "AI" は `aria-hidden`、視覚的に隠した要素に a11y 文言を置く（下記）。  
   (4) **色だけに頼らない**：文字 "AI" に加え、枠線（inset border）を付ける。
3. **ネイティブから削除／書き出し／同意を外す**のは、**同じアカウントで Web 版から実際にそれらを実行でき、台本でも Web 版で見せる**ことが条件。静的注記の文言は変えない。

### QAチェックポイント（§3.4）

- [ ] **同じアカウントで、Web版から書き出し・削除・同意を実際に実行できる**（ネイティブ非表示の前提条件）。

## A. §3.4 ネイティブ版：削除／JSON書き出し／同意

**決定：表示しない（hide）。押せなくする（disabled）は採らない。**

**条件（法務・上記3）**：同じアカウントが Web 版で書き出し・削除・同意を実行でき、デモ台本が Web 版でそれらを見せること。満たせないならネイティブから外さない（または締切延長等の別判断）。

- 残す3つ：データの置き場所一覧／目的ごとの根拠記録／本文なしログの表示。
- 削除・書き出し・同意のボタンとメニューは **DOM に出さない**。
- 静的注記（文言変更なし）：
  - ja `native-dp-web-only-note` = 「書き出し・削除・同意の操作はWeb版で行います。」
  - en = "Export, removal, and consent are handled in the web version."

**理由（従来どおり）**：disabled はデモで不具合に見える／WCAG 1.4.3 Incidental で非活性はコントラスト対象外で読みにくい（https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html）／文言なし操作を防ぐ／機能の存在は注記で伝える。

**切替（リード回答 13:43）**：ネイティブで残す画面を見せたあと S05 の前で、発表者が予め開いたブラウザへ手で移る。アプリ内リンクは作らない。

## B. §4 英語モードの入口

**決定：デモ経路外の入口は英語モードで表示しない。** Wiki／通知は **日英とも表示しない**。「日本語に切り替えて開く」ツールチップは出さない。英語切替時に経路外なら **S01 へ戻す**。S06 は常時。空の tab ストップを残さない。

（入口表は `sidebar-ja-vs-en.html` と同一。略）

## C. S00 常時表示・模擬・AI

| ID | ja | en | 区分 | 印 |
|---|---|---|---|---|
| `banner-demo-mock` | デモ・模擬応答 | Demo · simulated responses | ja=RUI L147 / GDPR-BE L104。en・ID=UI提案 | `# REQUIRED` · 省略禁止 |
| `label-ai-member` | AI | AI | 見える文字。リード：デモは「AI」のみ | `# REQUIRED` |
| `label-ai-member-a11y` | AIメンバー | AI member | 読み上げ専用。法務条件 | `# REQUIRED` |
| `state-sent-simulated` | 送信済み（模擬） | Sent (simulated) | ja=RUI L146/L183。`sent_awaiting_reply` のデモ文言（状態enumはそのまま・リード） | `# REQUIRED` · 省略禁止 |

### 「（模擬）」省略禁止（実装規則）

- `text-overflow: ellipsis` を **付けない**。
- `overflow: hidden` ＋幅固定で切らない。
- 狭い幅・通知では **最大2行まで折り返し**（`white-space: normal; word-break: break-word`）。
- モック：`mobile-simulated-*.html`（360px）。

### AIバッジの a11y 実装（採用）

**方式：見える "AI" に `aria-hidden="true"` ＋ `.visually-hidden` に `label-ai-member-a11y`。**

理由：`aria-label="AI"` と見える "AI" を併用すると「AI AI」と二重読みされる。名前の直後にバッジを置くと、読み上げは「仕様Bot、AIメンバー」/"Hamada Bot, AI member" になる。バッジ要素自体に `aria-label` を載せる方式でもよいが、名前テキストと結合しにくいので visually-hidden を推奨。

### AIバッジのコントラスト（実測）

| テーマ | 文字 | 面 | 比 | 判定 |
|---|---|---|---|---|
| light | `#2B36FF` | `#E3E5FF` | **5.50** | ≥4.5 OK |
| dark | `#8F96FF` | `#2A2C4A` | **5.12** | ≥4.5 OK |

枠線：`box-shadow: inset 0 0 0 1px` にアクセント色（色だけ識別にしない）。

設定トグル：**存在しない**。実装者は設定画面に「AI表示を隠す」等を追加しない。

## Fluent ID 一覧

| id | REQUIRED | CONDITIONAL | 備考 |
|---|---|---|---|
| banner-demo-mock | ✓ | | 省略禁止 |
| label-ai-member | ✓ | | 見える文字 |
| label-ai-member-a11y | ✓ | | 読み上げ |
| state-sent-simulated | ✓ | | 省略禁止・デモ文言 |
| native-dp-web-only-note | ✓ | ✓ native only | 静的注記 |
| native-dp-* / nav-* | | | パネル・モック用 |

## 検証

```
python3 /tmp/check_ftl.py entry-points.ja.ftl entry-points.en.ftl
```

## 開発リード回答（2026-11-09 13:43）・法務条件（同日）は上に統合済み

残る未決：候補 A/B 本人確認 → 入口表更新。`state-sent-simulated` のコア契約は 11/10 12:00 候補に載せる（リード）。
