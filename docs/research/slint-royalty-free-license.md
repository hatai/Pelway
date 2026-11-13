# Slint Royalty-free License 2.0 の利用条件

Issue #227 / A1（Rust＋Slint のネイティブ GUI）の要件として、Slint を `LicenseRef-Slint-Royalty-free-2.0` で使う条件を一次資料で確認した。対象は `crates/pelway-client-ui` が固定している `slint = "=1.18.1"`（`Cargo.toml`）で、確認日は 2026-11-13。

この文書では、ライセンス本文・FAQ・`LICENSE.md` の記述を「事実」とし、Pelway クライアントへの当てはめを「判断」として分ける。判断は出典元の主張ではない。

## 1. ライセンスの選択肢

Slint framework は triple-licensed で、利用者は Royalty-free License、GPLv3、Commercial license のいずれかを選べる [Slint LICENSE.md (2026/11), Licenses of the Slint Framework]。

Pelway は #168 の判断で GPL-3.0 ではなく Royalty-free License を選ぶ。したがって、この文書では Royalty-free License 2.0 だけを扱う。GPLv3 と Commercial license は対象外。

## 2. 条件の一覧

| 条件 | ライセンス上の扱い（要約） | Pelway クライアントへの当てはめ | 出典 |
|---|---|---|---|
| 利用できる製品の種類 | Desktop Application、Mobile Application、Web Application の一部として、Slint の使用・複製・提供・修正・表示・実演・頒布を認める。Application は汎用コンピュータ、モバイル端末、ブラウザ sandbox で動くプログラムと定義される | A1 は汎用コンピュータ向けデスクトップクライアントの UI 骨組み。`ui/main.slint` の `MainWindow inherits Window` が対象で、Desktop Application に該当する想定 | [Slint Royalty-free License 2.0 (2026/11), §1] |
| attribution 表示 | Application の一部として Slint を頒布する場合、(a) `AboutSlint` widget をトップレベルメニューから到達できる About 画面/ダイアログに表示する（そのような画面/ダイアログが無い場合は Splash Screen に表示する）、または (b) 公開ウェブページに Slint attribution badge を表示する、のどちらかが必要 | 現在の `pelway-client-ui` には `AboutSlint`・`MadeWithSlint`・Splash Screen・トップレベルメニューが見つからない。`Cargo.toml` も表示場所を未決としている。フォロー Issue は #237 | [Slint Royalty-free License 2.0 (2026/11), §2] |
| Slint 単体の頒布・公開 | Slint を Application に統合せず、単体で頒布または公開することは許可しない。その目的には GPLv3 を使う | 現時点で Slint を単体配布する経路は見当たらない。`pelway-client-ui` は `publish = false` のアプリ側クレート | [Slint Royalty-free License 2.0 (2026/11), §3] |
| Embedded Systems | Embedded Systems 内で Slint を使うことは許可しない。Embedded System は、より大きな機械・電気システム内で特定タスクを行うコンピュータシステム | A1 はデスクトップクライアントで、組込み用途の記述は見当たらない。用途が組込みへ変わる場合はこのライセンスでは使えない | [Slint Royalty-free License 2.0 (2026/11), §3]、[Slint FAQ Licensing (2026/11), What counts as an embedded system?] |
| Slint API の公開 | Slint の API を全部または一部でも公開する Application の頒布は許可しない | 現在のクライアントはアプリ UI 骨組みで、Slint API を公開する配布物ではない。`publish = false` で外部 crate としても公開していない。公開 API を持つ配布形態にする場合は再確認が必要 | [Slint Royalty-free License 2.0 (2026/11), §3] |
| license notices の保持 | Slint のソースコード中の license notices（copyright notices、warranty disclaimers、liability limitations を含む）を除去または変更してはならない | このリポジトリは Slint のソースを vendor・patch していない。crates.io 版 `slint = "=1.18.1"` を依存として使うため、現時点で notices を改変する経路は見当たらない | [Slint Royalty-free License 2.0 (2026/11), §3] |

## 3. ライセンス本文の引用

### 3.1 §1 Grant of Rights と Application の定義

```text
SixtyFPS hereby grants You a world-wide, royalty-free, non-exclusive license to use, reproduce, make available, modify, display, perform, distribute the Software as part of a Desktop, Mobile, or Web Application.

A **Desktop Application** is a computer program that is designed to run on a general-purpose computer (PC or notebook), typically installed and executed locally on the computer's operating system.

A **Mobile Application** is a computer program that is designed to run on a general-purpose mobile computer (mobile phone or tablet), typically installed and executed locally on the computer's operating system.

A **Web Application** is a computer program that is designed to run in the sandbox environment provided by a web browser.

Desktop Application, Mobile Application, and Web Application are hereafter referred to as **Application**.

【和訳】
SixtyFPS は、Desktop、Mobile、Web Application の一部として Software を使用、複製、提供、修正、表示、実演、頒布することを認める、世界中で有効な、ロイヤリティフリーで非独占的なライセンスを You に付与します。

**Desktop Application** とは、汎用コンピュータ（PC またはノート PC）で動作するよう設計されたコンピュータプログラムで、通常はコンピュータの OS にローカルインストールされ、実行されます。

**Mobile Application** とは、汎用モバイルコンピュータ（携帯電話またはタブレット）で動作するよう設計されたコンピュータプログラムで、通常はコンピュータの OS にローカルインストールされ、実行されます。

**Web Application** とは、Web ブラウザが提供する sandbox 環境で動作するよう設計されたコンピュータプログラムです。

Desktop Application、Mobile Application、Web Application は、以後 **Application** と呼びます。
```

[Slint Royalty-free License 2.0 (2026/11), §1]

### 3.2 §2 License Conditions - Attribution

```text
You may distribute the Software as part of an Application, modified or unmodified, provided that You do either of the following:

(a) Display the [`AboutSlint`](https://docs.slint.dev/latest/docs/slint/reference/std-widgets/misc/aboutslint/) widget in an "About" screen or dialog that is accessible from the top level menu of the Application. In the absence of such a screen or dialog, display the widget in the "Splash Screen" of the Application.

(b) Display the [Slint attribution badge](https://github.com/slint-ui/slint/tree/master/logo/MadeWithSlint-logo-whitebg.png) on a public webpage, preferably where the binaries of your Application can be downloaded from, in such a way that it can be easily found by any visitor to that page.

【和訳】
Software を Application の一部として、修正の有無にかかわらず頒布してもよい。ただし、次のいずれかを行うこと。

(a) `AboutSlint` widget を、Application のトップレベルメニューから到達できる "About" 画面またはダイアログに表示する。そのような画面またはダイアログが無い場合は、Application の "Splash Screen" に widget を表示する。

(b) Slint attribution badge を公開ウェブページに表示する。できれば Application のバイナリをダウンロードできる場所に置き、そのページの訪問者が容易に見つけられるようにする。
```

[Slint Royalty-free License 2.0 (2026/11), §2]

### 3.3 §3 Limitations

```text
The License does not permit to distribute or make the Software publicly available alone and without integration into an Application. For this purpose you may use the Software under the GNU General Public License, version 3.

The License does not permit the use of the Software within Embedded Systems. An **Embedded System** is a computer system designed to perform a specific task within a larger mechanical or electrical system.

The License does not permit the distribution of Application that exposes the APIs, in part or in total, of the Software.

You may not remove or alter any license notices (including copyright notices, disclaimers of warranty, or limitations of liability) contained within the source code form of the Software.

【和訳】
このライセンスは、Software を Application に統合せず、単体で頒布または公開することを許可しません。この目的には、GNU General Public License version 3 の下で Software を使用できます。

このライセンスは、Embedded Systems 内での Software の使用を許可しません。**Embedded System** とは、より大きな機械または電気システム内で特定タスクを行うよう設計されたコンピュータシステムです。

このライセンスは、Software の API を全部または一部でも公開する Application の頒布を許可しません。

Software のソースコード形式に含まれる license notices（copyright notices、warranty disclaimers、liability limitations を含む）を除去または変更してはなりません。
```

[Slint Royalty-free License 2.0 (2026/11), §3]

### 3.4 FAQ の Embedded System の説明

```text
An **embedded system** is a computer system that performs a specific task within a larger mechanical or electrical system — for example the controller driving the screen of an appliance, a point-of-sale terminal, or a car dashboard. The Commercial license or the GPL are suitable licenses for Slint for such systems.

It's **not** an embedded system when your application runs on a user's own general-purpose computer or phone, installed as one application among many. That's the desktop, mobile, and web case the Royalty-free License covers for free.

【和訳】
**embedded system** とは、より大きな機械または電気システム内で特定タスクを行うコンピュータシステムです。例えば、家電の画面を駆動するコントローラ、POS 端末、自動車のダッシュボードなどです。そのようなシステムには、Commercial license または GPL が適した Slint のライセンスです。

アプリケーションがユーザー自身の汎用コンピュータまたは電話で、多数のアプリケーションの 1 つとしてインストールされて動作する場合、それは embedded system ではありません。それが、Royalty-free License が無償で対象とする desktop、mobile、web のケースです。
```

[Slint FAQ Licensing (2026/11), What counts as an embedded system?]

## 4. Pelway クライアントの現状

- `crates/pelway-client-ui/Cargo.toml` は `slint = "=1.18.1"` と `slint-build = "=1.18.1"` を固定し、`publish = false` を指定している。同ファイルのコメントは「Slint のライセンス（U05）は未決のため、表示の置き場所はまだ決めていない」と残っている。
- `crates/pelway-client-ui/build.rs` は `slint_build::compile("ui/main.slint")` で UI をコンパイルし、`src/lib.rs` は `slint::include_modules!()` で生成モジュールを読み込む。
- `crates/pelway-client-ui/ui/main.slint` の `MainWindow` は `Window` を継承する画面全体で、トップレベルメニュー・About 画面/ダイアログ・Splash Screen を持たない。`AboutSlint`・`MadeWithSlint`・attribution badge の表示は見つからない。
- `crates/pelway-client-ui/src/i18n.rs` のコメントも、fluent 関連 crate のライセンス表示をどこに出すか未決で、About 画面は作っていないと記録している。
- `AboutSlint` は "Made with Slint" badge を表示する widget である [AboutSlint docs (2026/11), AboutSlint]。

## 5. フォロー Issue と残る確認

配布を始める時点で必須になる attribution 表示が未実装のため、フォロー Issue #237 を起票した。#237 は `AboutSlint` を About 画面/ダイアログに置き、トップレベルメニューから到達できるようにする方針を採用する。

その他の条件については、現時点のクライアントでは次のように扱う。

- Desktop Application の範囲: A1 は汎用コンピュータ向けデスクトップクライアントなので、Royalty-free License の対象に入る想定。ただし配布形態が組込み・API 公開型へ変わる場合は、ライセンス選択を再確認する。
- Slint 単体の頒布・公開: 現在はアプリに組み込む利用で、Slint 単体を配布する経路は見当たらない。
- Embedded Systems: 現在の用途は組込みではない。組込み用途が加わった場合は Commercial license または GPLv3 を再検討する。
- Slint API の公開: 現在は Slint API を公開する配布物ではない。公開 API を持つ配布形態にする場合は再確認する。
- license notices: Slint ソースを改変・同梱していないため、現時点で notices を除去・変更する経路は見当たらない。依存の更新や vendoring を行う場合は再確認する。

## 6. 出典

[Slint LICENSE.md, 2026/11] SixtyFPS. "LICENSE.md." slint-ui/slint. https://github.com/slint-ui/slint/blob/v1.18.1/LICENSE.md
[Slint Royalty-free License 2.0, 2026/11] SixtyFPS. "LicenseRef-Slint-Royalty-free-2.0." slint-ui/slint. https://github.com/slint-ui/slint/blob/v1.18.1/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md
[Slint FAQ Licensing, 2026/11] SixtyFPS. "FAQ.md." slint-ui/slint. https://github.com/slint-ui/slint/blob/v1.18.1/FAQ.md#licensing
[AboutSlint docs, 2026/11] SixtyFPS. "AboutSlint." Slint documentation. https://github.com/slint-ui/slint/blob/v1.18.1/docs/astro/src/content/docs/reference/std-widgets/misc/aboutslint.mdx
