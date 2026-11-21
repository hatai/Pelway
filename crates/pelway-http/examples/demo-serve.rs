//! デモのバックエンドを止めるまで動かし続ける起動口（skill `verify-pelway` 用）。
//!
//! 実行（ワークスペースルートから）:
//!
//! ```text
//! cargo build --offline --locked -p pelway-http --example demo-serve
//! target/debug/examples/demo-serve
//! ```
//!
//! - 待ち受けは `127.0.0.1:0` だけ（ポートは OS が選ぶ）。外部への接続はしない。
//! - 保存部はデモ seed（[`DataStore::new_demo_seeded`]。#案件B と 4 人）。メモリのみ。
//! - runner-lite の LLM は台本モード（`pelway_runner::demo::ScriptGateway`）に固定する。設定は
//!   [`DemoLlmConfig::from_vars`] と [`DemoLlmConfig::build_gateway_with`] に「どの
//!   変数も無い」写像を渡して組み立て、環境変数（`PELWAY_LLM_API_KEY`・
//!   `PELWAY_LLM_CUSTOM_BASE_URL` を含む）は参照しない。キーが無いので台本モードに
//!   なり、実提供元には接続しない。組み立て時に接続先を知らせる 1 行
//!   （`pelway-runner: LLM endpoint: ...`）が標準エラーに出る。
//! - 待ち受けを始めたら標準出力に 1 行だけ出す:
//!   `demo-serve ready addr=127.0.0.1:<port> pid=<pid> llm=script`
//! - 停止 API は無い。止めるときは起動した側が PID を指定して SIGTERM を送る。

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pelway_contract::ids::MemberId;
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::store::{DEMO_HAL_MEMBER_ID, DataStore};
use pelway_http::{SharedDataStore, SharedLedger, serve_with_store_and_runner};
use pelway_runner::demo::DemoLlmConfig;
use pelway_runner::{Runner, RunnerConfig};

/// デモ seed で判断待ちの決定者になる人（山本）。
const DECIDER: &str = "m-yamamoto";

fn now_ms() -> u64 {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    u64::try_from(ms).unwrap_or(u64::MAX)
}

fn main() {
    // どの変数も無い写像で組み立てる（環境変数は参照しない）。キーが無いので台本モード。
    let config = DemoLlmConfig::from_vars(|_| None).expect("default demo LLM config");
    let gateway = config.build_gateway_with(|_| None);
    let store: SharedDataStore = Arc::new(Mutex::new(DataStore::new_demo_seeded(now_ms())));
    let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let runner = Arc::new(Runner::new(
        RunnerConfig {
            bot_member_id: MemberId::from(DEMO_HAL_MEMBER_ID),
            decider: MemberId::from(DECIDER),
            llm_basis: config.llm_basis,
            llm_timeout: config.timeout,
        },
        Arc::clone(&store),
        Arc::clone(&ledger),
        gateway,
    ));
    let server = serve_with_store_and_runner(
        "127.0.0.1:0".parse().expect("loopback addr"),
        ledger,
        store,
        runner,
    )
    .expect("serve on 127.0.0.1:0");
    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "demo-serve ready addr={} pid={} llm=script",
        server.local_addr(),
        std::process::id()
    )
    .expect("write ready line");
    out.flush().expect("flush ready line");
    drop(out);
    loop {
        std::thread::park();
    }
}
