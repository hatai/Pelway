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
//! - runner-lite の LLM は台本モード（[`ScriptGateway`]）に固定する。環境変数は
//!   読まないので、キーがあっても実提供元には接続しない。
//! - 待ち受けを始めたら標準出力に 1 行だけ出す:
//!   `demo-serve ready addr=127.0.0.1:<port> pid=<pid> llm=script`
//! - 停止 API は無い。止めるときは起動した側が PID を指定して SIGTERM を送る。

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use pelway_contract::ids::MemberId;
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::store::{DEMO_HAL_MEMBER_ID, DataStore, PurposeId};
use pelway_http::{SharedDataStore, SharedLedger, serve_with_store_and_runner};
use pelway_runner::demo::ScriptGateway;
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
    let store: SharedDataStore = Arc::new(Mutex::new(DataStore::new_demo_seeded(now_ms())));
    let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
    let runner = Arc::new(Runner::new(
        RunnerConfig {
            bot_member_id: MemberId::from(DEMO_HAL_MEMBER_ID),
            decider: MemberId::from(DECIDER),
            llm_basis: PurposeId::P06,
            llm_timeout: Duration::from_secs(30),
        },
        Arc::clone(&store),
        Arc::clone(&ledger),
        Arc::new(ScriptGateway),
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
