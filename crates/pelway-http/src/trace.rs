//! tracing のローカル出力（#103・02「観測」）。
//!
//! - 出力は **ローカルだけ**（既定は stderr）。外部へ送る exporter
//!   （OpenTelemetry/OTLP など）は **持たない**。これは 02 の「外部exporterや自動設定を
//!   初期有効にしない」と `10-domestic-only` の方針。
//! - 環境変数（`RUST_LOG`・`OTEL_*`）からの自動設定もしない。固定の INFO レベル。
//! - subscriber の組み立ては [`local_subscriber`] が 1 か所で行い、
//!   [`init_local_tracing`] はそれを stderr に向けて大域に設定するだけ。
//! - 台帳（`pelway-contract`）の承認まわりの span はここを設定すると見える。
//!   コメント本文・添付の中身などは span に載せない（`pelway-http/src/log.rs` の
//!   許可リスト方針と同じ）。

#![forbid(unsafe_code)]

use std::io;

use tracing::Subscriber;
use tracing_subscriber::fmt::MakeWriter;

/// ローカル出力だけの subscriber を組み立てる（exporter 無し・固定 INFO）。
///
/// [`tracing_subscriber::fmt`] だけを重ねる。`EnvFilter`（`RUST_LOG` などの自動設定）も
/// OpenTelemetry 系の layer も持たない。レベルは INFO 固定。
fn local_subscriber<W>(writer: W) -> impl Subscriber + Send + Sync + 'static
where
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(writer)
        .finish()
}

/// ローカル出力の tracing を大域に設定する（既定の stderr 出力）。
///
/// すでに大域の subscriber があるときは何もせず `false` を返す
/// （[`tracing_subscriber::util::SubscriberInitExt::try_init`] と同じ扱い）。
#[must_use = "設定できたかどうかは呼び出し側で確かめる"]
pub fn init_local_tracing() -> bool {
    use tracing_subscriber::util::SubscriberInitExt;
    local_subscriber(io::stderr).try_init().is_ok()
}

/// 外部 exporter を示す印（パッケージ名・依存指定に現れる語）。
const EXPORTER_MARKERS: [&str; 2] = ["opentelemetry", "otlp"];

/// テキスト（`Cargo.lock`・`Cargo.toml`）の中から外部 exporter を示す行を集める。
///
/// #103 の受け入れ条件「外部へ送るexporterは有効にしないことを確かめる」のための
/// 検査。埋め込み側・CI が依存表を点検するときにも使えるように公開する。
/// 空の `Vec` なら外部 exporter を示す行は無い。
#[must_use]
pub fn exporter_mentions(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            EXPORTER_MARKERS.iter().any(|marker| lower.contains(marker))
        })
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// インメモリの書き込み先（ローカル出力の検証用）。
    #[derive(Clone, Default)]
    struct MemWriter(Arc<Mutex<Vec<u8>>>);

    impl MemWriter {
        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().expect("mem writer lock").clone()).expect("utf-8")
        }
    }

    impl io::Write for &MemWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .expect("mem writer lock")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for MemWriter {
        type Writer = &'a MemWriter;

        fn make_writer(&'a self) -> Self::Writer {
            self
        }
    }

    /// #103 の受け入れ条件: tracing の出力がローカルの書き込み先に届く
    /// （exporter を経由しない）。
    #[test]
    fn local_tracing_writes_events_to_the_local_sink() {
        let mem = MemWriter::default();
        let subscriber = local_subscriber(mem.clone());
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("pelway-local-tracing-probe");
        });
        let out = mem.contents();
        assert!(
            out.contains("pelway-local-tracing-probe"),
            "ローカルの書き込み先に届いていない: {out:?}"
        );
    }

    /// 検出器そのものの検証: exporter を示す行を見つけられること
    /// （次の 2 つの検査が「最初から通る」だけでないことの保証）。
    #[test]
    fn exporter_detector_catches_violations() {
        let crafted = r#"
[[package]]
name = "opentelemetry-otlp"
version = "0.0.0"
"#;
        assert_eq!(
            exporter_mentions(crafted),
            vec!["name = \"opentelemetry-otlp\"".to_owned()]
        );
        let also_otlp = "tracing-opentelemetry = \"0.30\"\notlp-something = \"1\"";
        assert_eq!(exporter_mentions(also_otlp).len(), 2, "{also_otlp:?}");
        // exporter を示さない行には反応しない。
        assert!(exporter_mentions("tracing = \"0.1\"\ntracing-subscriber = \"0.3\"").is_empty());
    }

    /// #103 の受け入れ条件: 依存に外部 exporter が無いこと（Cargo.lock）。
    #[test]
    fn workspace_lock_has_no_external_exporter_packages() {
        let lock = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
        let text = std::fs::read_to_string(&lock)
            .unwrap_or_else(|e| panic!("{} を読めない: {e}", lock.display()));
        assert!(
            exporter_mentions(&text).is_empty(),
            "Cargo.lock に外部 exporter がある: {:?}",
            exporter_mentions(&text)
        );
    }

    /// #103 の受け入れ条件: どの crate も外部 exporter に依存していないこと。
    #[test]
    fn workspace_manifests_have_no_external_exporter_dependencies() {
        let crates_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let entries = std::fs::read_dir(&crates_dir)
            .unwrap_or_else(|e| panic!("{} を読めない: {e}", crates_dir.display()));
        let mut checked = 0;
        for entry in entries.flatten() {
            let manifest = entry.path().join("Cargo.toml");
            if !manifest.is_file() {
                continue;
            }
            let text = std::fs::read_to_string(&manifest)
                .unwrap_or_else(|e| panic!("{} を読めない: {e}", manifest.display()));
            assert!(
                exporter_mentions(&text).is_empty(),
                "{} に外部 exporter がある: {:?}",
                manifest.display(),
                exporter_mentions(&text)
            );
            checked += 1;
        }
        assert!(checked >= 3, "crates を確認していない: {checked}");
    }

    #[test]
    fn init_local_tracing_returns_false_when_global_subscriber_exists() {
        // グローバル subscriber は 1 度だけ設定できる（先に設定されたのが
        // このテスト自身か別のテストかにかかわらず、2 回目は必ず失敗する）。
        let _ = init_local_tracing();
        assert!(!init_local_tracing());
    }

    #[test]
    fn mem_writer_flush_is_ok() {
        let mem = MemWriter::default();
        let mut w = &mem;
        std::io::Write::flush(&mut w).expect("flush");
    }
}
