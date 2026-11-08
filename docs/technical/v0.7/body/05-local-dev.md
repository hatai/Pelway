<a id="ch05-s01"></a>
# 05 3OSのローカル開発・試験・配布設計

環境準備・インストール・ユーザーPC操作は今回実施しない。以下は承認後に用いる構成案で、手順の成功報告ではない。

<a id="ch05-s02"></a>
## OS別の境界

| 項目 | Windows | macOS | Linux |
|---|---|---|---|
| GUI build/run | Windows MSVCのRustとWindows SDK | macOSのRustとApple SDK | Linux native toolchain |
| backend/tool | WSL 2内PodmanまたはDocker Engine | Apple container。Intel/未対応OSの代替はPodman machineを別審査 | rootless Podman |
| CPU | x86_64/ARM64を別targetとして確認 | Apple containerはApple silicon。Intel Macへそのまま使わない | x86_64/ARM64を別imageで確認 |
| native試験 | Microsoft IME、NVDA、通知、clipboard、DPI | 日本語入力、VoiceOver、通知、権限、Retina | fcitx5/IBus、Orca、X11/Wayland、font |
| hostで行う配布工程 | MSI/MSIX等と署名・SmartScreen挙動の確認 | app/dmg、署名、公証、Gatekeeper確認 | deb/AppImage/Flatpakの選定・依存確認 |

Linux containerのbuild成功はWindows/macOS GUIのbuild成功を意味しない。共有するのはソースとOCI image定義であり、CPU arch別のnative binaryまで同一ではない。

<a id="ch05-s03"></a>
## WindowsとWSL 2

Microsoftの[WSL install](https://learn.microsoft.com/en-us/windows/wsl/install)はWindows 10 version 2004 build19041以降、またはWindows 11を示す。これはインストール方法の条件で、利用予定Windowsの保守状況や全container機能の保証とは別。仮想化・CPU・kernel・network要件を実環境で確認する。

GUIはWindows、Linux backendはWSL内で動かす。WSLgのLinux GUIはWindows向け製品試験の代わりにしない。単一ソースの置き場所案はWindows側の同期外workspace、backend build contextをWSLの管理volumeへコピーする方式。共有mountでのfile I/O性能は未測定で、Windows側にソースを置くこと自体が技術上必須ではない。

Docker Desktopの要件と費用条件をPodman/Docker Engineへ転用しない。[Desktopの要件](https://docs.docker.com/desktop/setup/install/windows-install)と[license条件](https://docs.docker.com/subscription-billing/desktop-license)は別製品の比較資料。初期案はDesktopではなくWSL内runtime。runtimeの更新・image取得・diagnostic通信も10で確認する。

<a id="ch05-s04"></a>
## macOSとApple container

[apple/container](https://github.com/apple/container)の公式資料に従いApple siliconとmacOS 26を想定する。対象releaseのサポート範囲は実装直前に再照合。Linux containerを軽量VMで動かす構成であり、Rust GUIやApple固有APIはmacOS hostに残る。

[BUILDING.md](https://github.com/apple/container/blob/main/BUILDING.md)のvmnet関連制約を確認し、作業ディレクトリ例は同期外の `~/projects` とする。ただしディレクトリ名だけでiCloud等の同期外を保証しない。内部専用network、複数containerの起動順、DNS、port公開、rootless性、GPU可用性は要確認。Apple containerへDocker Composeの設定がそのまま使えるとはしない。

Apple siliconのGPUをLinux VMから直接使える前提を置かない。ローカル推論でhost backendを使う場合、国内ホストの専用プロセスをloopbackで接続し、sandboxにはそのsocketを直接渡さない。CPU推論またはスタブでも設計試験を続けられる。

<a id="ch05-s05"></a>
## Linuxと起動構成

Linuxはrootless Podmanを推奨。X11/Wayland、fcitx5/IBusを別試験にする。host kernel共有のcontainer隔離をmicroVMと同じ強度とは言わない。Apple containerのVMも構成・管理権限次第であり、他OSより無条件に安全とは断定しない。

起動単位はpostgres、server、runner、integration gateway、sandbox controller、stub/local inference、dummy mail、task sandbox。`app` と `sandbox` networkを分け、DBはsandboxから到達不能にする。未確認のruntimeは外向き遮断を実証するまで機密実行に使わない。

外部連携を使う場合だけ、gatewayから許可済みAdapterへの別出口を設ける。本体推論・DB・sandboxへ広いインターネット接続を与えない。外部サービスのcallbackを受けるためにローカルhostへ勝手にtunnelや公開portを作らない。polling/loopback bridgeで足りるか、公開ingressが将来必要かは11の制約として扱う。

秘密はhostのcredential storeとgatewayのbrokerで管理する案。host storeをcontainerへ丸ごとmountせず、認証済みの限定IPCで必要操作だけ仲介する。3OSのbroker実装と暗号化鍵の管理は未検証。DB管理者キーをBotへ渡さない。

<a id="ch05-s06"></a>
## 試験と配布

| 試験 | 場所 | 対象 |
|---|---|---|
| domain単体 | Linux containerとhost | 9状態、hash固定、scope、順序、承認失効 |
| 結合 | ローカルcontainer群 | DB/outbox、lease/fence、疑似agent、応答断・再起動・重複・権限取消 |
| GUI | 3OS host | Slintとeguiの同一fixture。headless試験だけでIME/読み上げ合格にしない |
| Web | host browser | DOM、WASM、focus、composition、アクセシビリティ、snapshot再同期 |
| 国内境界・連携例外 | host/VM/container全体 | 10の通信観測と11のscope逸脱・取消試験 |

既存計画の応答断20回、再承認20回、権限取消の条件別試験を減らさない。試験実行は別承認後で、現在は未実施。

ローカルの未署名buildと一般配布を分ける。Windowsの配布形式に応じた署名要件・証明書/サービス費用、macOS Developer ID署名・公証の登録、Linuxのpackage署名と配布方式は未決。『あらゆる署名は必ず有料』とは言わない。有料登録・証明書購入は今回行わず、選定後に確認する。

署名や公証は外部通信を伴い得る。機密データを含めないbuild成果物の配布工程と、機密実行時の本体国内境界を分離する。release用SBOM、NOTICE、再現build、image digest、model hash、CPU別配布物、手動update/rollbackを記録する設計。OS installerだけでLinux backendやmodelが自動で準備できると見せない。

controllerは信頼する管理側の高権限部品で、侵害時にhostへ影響し得る。runtime socketへ触る範囲をcontrollerだけへ限定するが、Docker Engineのroot権限相当の影響が消えるとはしない。初期Windowsの実機試験はWSL2＋rootless Podmanを第一候補として一つに絞る案で、Docker Engineは別見積もり/承認の代替。Podman/Engine/Apple containerごとのcontroller実装・保守・試験工数は未見積もり。

macOSのhost推論endpointをLinux VMから呼ぶ場合はloopbackとは別経路で、runner/gatewayだけに限定する。task sandboxとCLI連携区画からhost推論portへ到達できないことを10のD4で試す。未検証なら機密実行を許可しない。

運用のV01〜09は13を参照し、5秒境界、検査後中断/復旧、照会不能/429、削除失敗、backup復元と独立制限記録を未実施の期待値として既存試験へ対応付ける。局所モックで全ゲート合格とはしない。
