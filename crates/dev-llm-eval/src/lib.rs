//! `pelway-dev-llm-eval`: LLMゲートウェイの測定（Issue #95 の PR 3、ADR 0008「測定」）。
//!
//! `data/eval-data.json` の合成データ（日本語の業務文: 要約・承認カードの文案・
//! 引き継ぎの5項目・埋め込み用の小さな検索セット）を [`pelway_llm::LlmGateway`] に流し、
//! スキーマに合う割合・応答時間・トークン数・検索の当たり（recall@k）を
//! `docs/research/` 向けの Markdown 表にして [`markdown`] で返す。
//!
//! 実際の提供元での測定は hatai が自分のトークンで行い、結果を `docs/research/` に置く。
//! CI と `cargo test` はローカルのモックHTTPサーバーだけで動く（tests/eval.rs）。

use std::time::Instant;

use pelway_llm::{ChatMessage, ChatRequest, LlmError, LlmGateway};
use serde::Deserialize;
use serde_json::Value;

/// 合成データ。実行時にファイルを読まず、バイナリに埋め込む。
pub const DATA: &str = include_str!("../data/eval-data.json");

#[derive(Deserialize)]
struct EvalData {
    summarization: Vec<Item>,
    approval_cards: Vec<Item>,
    handover: Vec<Item>,
    retrieval: RetrievalData,
}

#[derive(Deserialize)]
struct Item {
    #[allow(dead_code)]
    id: String,
    input: String,
}

#[derive(Deserialize)]
struct RetrievalData {
    queries: Vec<Query>,
    documents: Vec<Document>,
}

#[derive(Deserialize)]
struct Query {
    #[allow(dead_code)]
    id: String,
    text: String,
    relevant: Vec<String>,
}

#[derive(Deserialize)]
struct Document {
    id: String,
    text: String,
}

/// chat 課題（要約・承認カード・引き継ぎ）1つ分の集計。
#[derive(Clone, Debug)]
pub struct TaskResult {
    pub task: &'static str,
    pub total: usize,
    pub schema_ok: usize,
    pub avg_ms: f64,
    pub max_ms: f64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

/// 検索課題（recall@k）の集計。
#[derive(Clone, Debug)]
pub struct RetrievalResult {
    pub queries: usize,
    pub k: usize,
    pub recall_at_k: f64,
    pub input_tokens: u64,
}

/// 測定結果。[`markdown`] で表にする。
#[derive(Clone, Debug, Default)]
pub struct EvalReport {
    pub tasks: Vec<TaskResult>,
    pub retrieval: Option<RetrievalResult>,
}

/// 要約の応答スキーマ（pelway-llm の部分実装の範囲で書く）。
fn summary_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "summary": {"type": "string"},
            "decisions": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["summary"]
    })
}

/// 承認カードの文案の応答スキーマ。
fn approval_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "title": {"type": "string"},
            "summary": {"type": "string"},
            "risk": {"enum": ["low", "medium", "high"]}
        },
        "required": ["title", "summary", "risk"]
    })
}

/// 引き継ぎ5項目の応答スキーマ。
fn handover_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "items": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["items"]
    })
}

/// 合成データを `gateway` に流して測定する。chat の失敗（拒否・結果不明を含む）は
/// その課題の「スキーマ不適合」として数え、測定は続ける。
pub async fn run_eval<G: LlmGateway>(gateway: &G, k: usize) -> Result<EvalReport, LlmError> {
    let data: EvalData = serde_json::from_str(DATA).expect("data/eval-data.json が壊れています");
    let mut report = EvalReport {
        tasks: Vec::new(),
        retrieval: None,
    };
    report
        .tasks
        .push(run_task(gateway, "要約", &data.summarization, summary_schema()).await);
    report.tasks.push(
        run_task(
            gateway,
            "承認カード",
            &data.approval_cards,
            approval_schema(),
        )
        .await,
    );
    report
        .tasks
        .push(run_task(gateway, "引き継ぎ", &data.handover, handover_schema()).await);
    report.retrieval = Some(run_retrieval(gateway, &data.retrieval, k).await?);
    Ok(report)
}

/// 課題1つを1件ずつ測る。応答時間は送信から応答受領まで。
async fn run_task<G: LlmGateway>(
    gateway: &G,
    name: &'static str,
    items: &[Item],
    schema: Value,
) -> TaskResult {
    let mut elapsed_ms: Vec<f64> = Vec::new();
    let mut schema_ok = 0;
    let mut input_tokens = 0;
    let mut output_tokens = 0;
    for item in items {
        let start = Instant::now();
        let reply = gateway
            .chat(ChatRequest {
                messages: vec![ChatMessage::user(item.input.clone())],
                schema: Some(schema.clone()),
            })
            .await;
        elapsed_ms.push(start.elapsed().as_secs_f64() * 1000.0);
        if let Ok(reply) = reply {
            schema_ok += 1;
            input_tokens += reply.usage.input_tokens;
            output_tokens += reply.usage.output_tokens;
        }
    }
    TaskResult {
        task: name,
        total: items.len(),
        schema_ok,
        avg_ms: elapsed_ms.iter().sum::<f64>() / elapsed_ms.len() as f64,
        max_ms: elapsed_ms.iter().cloned().fold(0.0, f64::max),
        input_tokens,
        output_tokens,
    }
}

/// 埋め込みで検索し、recall@k を出す。文書とクエリはそれぞれ1回の embed にまとめる。
async fn run_retrieval<G: LlmGateway>(
    gateway: &G,
    data: &RetrievalData,
    k: usize,
) -> Result<RetrievalResult, LlmError> {
    let documents = gateway
        .embed(data.documents.iter().map(|d| d.text.clone()).collect())
        .await?;
    let queries = gateway
        .embed(data.queries.iter().map(|q| q.text.clone()).collect())
        .await?;
    let input_tokens = documents.usage.input_tokens + queries.usage.input_tokens;
    let mut recalls = Vec::new();
    for (query, query_vector) in data.queries.iter().zip(&queries.vectors) {
        let mut scored: Vec<(f64, &str)> = data
            .documents
            .iter()
            .zip(&documents.vectors)
            .map(|(doc, vector)| (cosine(query_vector, vector), doc.id.as_str()))
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        let top_k: std::collections::HashSet<&str> =
            scored.iter().take(k).map(|(_, id)| *id).collect();
        let relevant: std::collections::HashSet<&str> =
            query.relevant.iter().map(String::as_str).collect();
        recalls.push(top_k.intersection(&relevant).count() as f64 / relevant.len() as f64);
    }
    Ok(RetrievalResult {
        queries: data.queries.len(),
        k,
        recall_at_k: recalls.iter().sum::<f64>() / recalls.len() as f64,
        input_tokens,
    })
}

/// コサイン類似度。0ベクトルとの類似度は 0 とする。
fn cosine(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// `docs/research/` 向けの Markdown 表。課題ごとの行と recall@k の行を持つ。
pub fn markdown(report: &EvalReport) -> String {
    let mut rows = String::new();
    rows.push_str("| タスク | 件数 | スキーマ適合 | 平均応答 (ms) | 最大応答 (ms) | 入力トークン | 出力トークン |\n");
    rows.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for task in &report.tasks {
        rows.push_str(&format!(
            "| {} | {} | {}/{} | {:.1} | {:.1} | {} | {} |\n",
            task.task,
            task.total,
            task.schema_ok,
            task.total,
            task.avg_ms,
            task.max_ms,
            task.input_tokens,
            task.output_tokens
        ));
    }
    match &report.retrieval {
        Some(retrieval) => rows.push_str(&format!(
            "| 検索 (recall@{}) | {} クエリ | {:.3} | — | — | {} | 0 |\n",
            retrieval.k, retrieval.queries, retrieval.recall_at_k, retrieval.input_tokens
        )),
        None => rows.push_str("| 検索 (recall@k) | 未実施 | — | — | — | — | — |\n"),
    }
    rows
}

/// コマンドライン引数。`llm-eval --provider zai --base-url ... --k 3`。
#[derive(Clone, Debug, PartialEq)]
pub struct CliOptions {
    /// `zai` か `openai`。設定例（`ProviderConfig::zai_example` 等）を選ぶ。
    pub provider: String,
    /// base_url の上書き（モックサーバー向け）。
    pub base_url: Option<String>,
    pub chat_model: Option<String>,
    pub embed_model: Option<String>,
    /// トークンを読む環境変数（fnox が注入する）。
    pub api_key_env: String,
    /// recall@k の k。
    pub k: usize,
    /// 有効な base_url が `https:` のとき true。
    pub https_only: bool,
}

/// 引数を解析する。不明なフラグ・空の値・範囲や形式に合わない値は `Err`（`--k` は1以上、PR #176 の must-fix）。
pub fn parse_args(args: impl Iterator<Item = String>) -> Result<CliOptions, String> {
    let mut provider = "zai".to_string();
    let mut base_url: Option<String> = None;
    let mut chat_model: Option<String> = None;
    let mut embed_model: Option<String> = None;
    let mut api_key_env = "PELWAY_LLM_API_KEY".to_string();
    let mut k = 3usize;
    let items: Vec<String> = args.skip(1).collect();
    let mut index = 0;
    while index < items.len() {
        let flag = items[index].as_str();
        let Some(value) = items.get(index + 1) else {
            return Err(format!("{flag} には値が要ります"));
        };
        if value.is_empty() {
            return Err(format!("{flag} に空の値は指定できません"));
        }
        match flag {
            "--provider" => provider = value.clone(),
            "--base-url" => base_url = Some(value.clone()),
            "--chat-model" => chat_model = Some(value.clone()),
            "--embed-model" => embed_model = Some(value.clone()),
            "--api-key-env" => api_key_env = value.clone(),
            "--k" => {
                k = value
                    .parse::<std::num::NonZeroUsize>()
                    .map_err(|_| format!("--k には1以上の整数を指定してください: {value}"))?
                    .get();
            }
            other => return Err(format!("不明な引数です: {other}")),
        }
        index += 2;
    }
    if provider != "zai" && provider != "openai" {
        return Err(format!(
            "--provider には zai か openai を指定してください: {provider}"
        ));
    }
    if let Some(base_url) = &base_url
        && !base_url
            .parse::<url::Url>()
            .is_ok_and(|parsed| matches!(parsed.scheme(), "http" | "https"))
    {
        return Err(format!(
            "--base-url には http/https の URL を指定してください: {base_url}"
        ));
    }
    let effective_base = base_url
        .clone()
        .unwrap_or_else(|| "https://api.z.ai/api/paas/v4".to_string());
    Ok(CliOptions {
        provider,
        base_url,
        chat_model,
        embed_model,
        api_key_env,
        k,
        https_only: effective_base.starts_with("https"),
    })
}
