# コーディングエージェントの並行数の上限と利用可否を設定する方式の調査

AI 開発チーム（`docs/superpowers/specs/agent-team.md`）が使うコーディングエージェントについて、「どの担当を同時に何本まで動かせるか」と「その環境でどの担当が使えるか」を開発者ごとに設定できるようにするための調査。担当とは、実装・調査・レビューを担うコーディングエージェントの 1 種で、skill `command-team` の割り当ての行の `agent=` の値と同じものである。既存のツールがこの 2 つをどこに置き、どう検証し、上限に達したときにどう動くかを、公式文書と公開リポジトリのソースで確認した。あわせて、開発者の機械を追加するときに必要になる事実（fnox による秘密の共有、GitHub Projects の共同作業者）を確認した。

## 結論

**採用する**。リポジトリで共有する 1 つの設定ファイルに、上限と利用可否を別々の情報で置き、コードが読んで検証する方式にする。

- 置き場所はリポジトリの 1 つのファイルにする。利用者の機械側の設定とリポジトリ側の設定を重ねる層構造と、起動時のフラグや環境変数で選ぶプロファイルは採らない。
- 利用可否は、上限とは別の情報で表す。上限の値を 0 にして「使えない」を表さない。
- 上限は 1 以上の整数として検証する。
- 上限を読む主体をコードに置く。文書に書いた上限を指揮役だけが読む形にしない。指揮役とは、Issue を実装担当に割り振るエージェントである（`docs/superpowers/specs/agent-team.md` の「役割と GitHub 上の名義」）。

各方式の一次資料は 1〜3 に、秘密の共有と Projects の事実は 4〜5 に、Pelway の現状と各決定の根拠は 6 にある。

## この文書の前提と読み方

- 「確認」は公式文書または公開リポジトリのソースに記載があるという意味で、実機での動作確認ではない。
- ソースはコミットを固定して読んだ（コミットは出典に示す）。Web の文書は 2026-11-25 時点の内容で、更新される可能性がある。
- 引用は原文（英語）のまま抜き出す。引用のあとの文は本調査の整理である。6 の「採用する」「採用しない」は Pelway 側の判断であって、出典の主張ではない。
- ツールの呼び方は出典どおり。「サブエージェント」と「spawn されたスレッド」は、親のエージェントが起動する子のエージェントを指す。

## 1. 並行数の上限の置き場所・検証・上限に達したときの動き

| ツール | 上限の置き場所と既定値 | 値の検証 | 上限に達したとき |
|---|---|---|---|
| gptme | 環境変数 `GPTME_SUBAGENT_MAX_CONCURRENT`、プロジェクトの `gptme.toml` の `[subagent] max_concurrent`、既定値 `min(8, os.cpu_count() or 2)` の順に決まる [gptme-concurrency] | 1 未満の値は警告を出して無視し、次の層を使う [gptme-concurrency] | セマフォで空きを待つ [gptme-concurrency] |
| OmO | ユーザー層 `~/.omo/omo.jsonc` とプロジェクト層 `.omo/omo.jsonc` の `background_task`（`defaultConcurrency`・`providerConcurrency`・`modelConcurrency`）。モデル、provider、既定の順に決まり、既定は 5 [omo-configuration] [omo-concurrency] | 0 以上の数。0 は無制限 [omo-schema] [omo-configuration] | 待ち行列に入り、空きができると取り出される [omo-concurrency] |
| Claude Code | 環境変数 `CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`。既定は 20 [claude-env-vars] | 正の整数だけを受け付ける。それ以外は無視され、この変数では無効にできない [claude-env-vars] | Agent ツールは起動に失敗する。workflow は空きを待つ [claude-sub-agents] [claude-env-vars] |
| Codex CLI | `~/.codex/config.toml` の `agents.max_concurrent_threads_per_session`。未設定なら既定を使う [codex-config-reference] | スキーマの `minimum` は 1 [codex-schema] | ホストが容量を管理する場合は `AgentLimitReached` を返し、待たない [codex-admission] |
| GSD | 文書の「Parallelization Settings」の表にある `parallelization.max_concurrent_agents`。既定は 3 [gsd-configuration] | 実装はこのキーを読まない [gsd-config-ts] [gsd-core-cjs] | 並列実行時に上限をかけない [gsd-phase-runner] |
| T3 Code | 並行して動かせるスレッド数の上限を設定する項目は、確認した文書に無い [t3-docs] | 該当なし | 該当なし |

### gptme

上限は最初の使用時に 1 回だけ決まる。

```
The limit is resolved at first use from (highest to lowest priority):

1. ``GPTME_SUBAGENT_MAX_CONCURRENT`` environment variable
2. ``[subagent] max_concurrent`` in ``gptme.toml``
3. ``min(8, os.cpu_count() or 2)`` safe default (mirrors Claude Code's ``min(16, cores-2)``)
```

[gptme-concurrency]

環境変数と `gptme.toml` のそれぞれで、1 未満の値は警告とともに無視される。

```python
            if val < 1:
                raise ValueError(f"must be >= 1, got {val}")
```

```python
                if mc < 1:
                    logger.warning(
                        f"Ignoring invalid [subagent] max_concurrent={mc!r}: must be >= 1"
                    )
```

[gptme-concurrency]

待機はセマフォで行う。

```
Provides a module-level semaphore that caps the number of concurrently running subagents.
```

[gptme-concurrency]

### OmO

`0` は無制限を表す。

```
Setting a concurrency value to `0` means unlimited (no cap) in both places: `background_task` (`defaultConcurrency`, `providerConcurrency`, `modelConcurrency`) and the core `task` object.
```

[omo-configuration]

優先順位はモデル、provider、既定の順で、どこにも無ければ 5 を返す。

```ts
    if (defaultLimit !== undefined) {
      return defaultLimit === 0 ? Infinity : defaultLimit
    }
    return 5
```

[omo-concurrency]

スキーマは 0 以上の数を認める。

```ts
  defaultConcurrency: z.number().min(0).optional(),
```

[omo-schema]

空きが無いときは待ち行列に入る。

```ts
  async acquire(model: string, taskId?: string): Promise<void> {
    if (this.tryAcquire(model)) {
      return
    }
```

[omo-concurrency]

### Claude Code

```
How many subagents can be running in one session before the Agent tool refuses to spawn another (default: 20). Accepts a positive whole number in plain digits; anything else is ignored, so the variable can adjust the cap but can’t disable it.
```

[claude-env-vars]

Agent ツールで上限に達したときは、起動が失敗する。

```
By default, when 20 subagents are running in a session, spawning another with the Agent tool fails with Concurrent subagent limit reached, and the error tells Claude not to retry.
```

[claude-sub-agents]

workflow には別の変数があり、こちらは空きを待つ。

```
How many agents a single workflow run executes at once, from 1 to 256. By default, a run executes up to 16 agents at once, fewer when Claude Code has fewer CPUs available; queued agent() calls wait for a free slot.
```

[claude-env-vars]

### Codex CLI

```
agents.max_concurrent_threads_per_session | number | Maximum number of spawned-agent threads that can be open concurrently, excluding the primary thread. When unset, Codex chooses the default.
```

[codex-config-reference]

スキーマでは 0 を指定できない。

```json
        "max_concurrent_threads_per_session": {
          "description": "Maximum number of spawned-agent threads that can be open concurrently per session. When unset, the selected multi-agent backend uses its default.",
          "format": "uint",
          "minimum": 1.0,
          "type": "integer"
        },
```

[codex-schema]

ホスト側の容量管理の規約を書いたコメントには、次の文がある。

```
/// Select host ownership and install this provider before thread startup finishes. Return
/// `AgentLimitReached` when capacity is exhausted rather than waiting for a slot.
```

[codex-admission]

### GSD

文書の表には上限のキーがある。

```
| `parallelization.max_concurrent_agents` | number | `3` | Maximum simultaneous agents |
```

[gsd-configuration]

実装の型は真偽値で、真偽値に正規化して読む。

```ts
  parallelization: boolean;
```

[gsd-config-ts]

```js
    const parallelization = (() => {
      const val = get('parallelization');
      if (typeof val === 'boolean') return val;
      if (typeof val === 'object' && val !== null && 'enabled' in val) return val.enabled;
      return defaults.parallelization;
    })();
```

[gsd-core-cjs]

並列実行は `Promise.allSettled` で一斉に始まり、上限をかけるコードは無い。

```ts
        const settled = await Promise.allSettled(
          wavePlans.map(plan => this.executeSinglePlan(phaseNumber, plan.id, sessionOpts)),
        );
```

[gsd-phase-runner]

`max_concurrent_agents` が登場するのは、コミット `bdcaab2c752d` のツリー全体（1854 ファイル）のうち 4 ファイルだけである。`docs/CONFIGURATION.md` とその日本語版・韓国語版、`get-shit-done/templates/config.json` である [gsd-template-config]。

### T3 Code

コミット `8b70e7337160` の `docs/`（78 ファイル）を `concurr`、`parallel`、`max threads`、`thread limit`、`max agents` で検索したところ、並行して動かせるスレッド数の上限を設定する項目は見当たらない。ソースコードは調べていない [t3-docs]。

## 2. 利用できるエージェントと provider を絞る方法

| ツール | 絞り方 | リポジトリ側から変えられるか |
|---|---|---|
| Claude Code | managed settings の `availableModels` と `deniedModels` でモデルを絞る。`Agent` を tools から外すとサブエージェントを起動できない [claude-settings] [claude-sub-agents] | managed settings は他のどの設定よりも上にある [claude-settings] |
| Codex CLI | provider の設定は機械側のユーザー設定に置く [codex-config-reference] | プロジェクトの `.codex/config.toml` に `model_provider` や `model_providers` を書いても無視される [codex-config-reference] |
| OpenCode 2 | `experimental.policies` で provider の使用を許すか拒むかを決める [opencode-policies] | プロジェクトにも書けるが、利用者の全体設定が勝つ [opencode-policies] |
| T3 Code | ポリシーの `providers.allowedDrivers` で動ける provider を列挙する [t3-managed-policy]。設定画面の provider の switch は機械ごとに決まる [t3-project-settings] | プロジェクト単位で on と off のどちら向きにも上書きできる [t3-project-settings] |
| Devin CLI | `subagents_enabled` は user only [devin-config] | プロジェクトの設定で使えるのは permissions、read_config_from、hooks と、別ファイルに置く mcpServers [devin-config] |

Claude Code は、モデルを絞る鍵を managed settings に置いている。

```
A managed model is a starting default, not a lock; the locks are availableModels and deniedModels.
```

[claude-settings]

```
If you omit Agent from the tools list entirely, the agent can’t spawn any subagents with the Agent tool.
```

[claude-sub-agents]

Codex CLI は、provider などの機械側の設定をプロジェクト側から変えられないようにしている。

```
Project-scoped config can’t override machine-local provider, auth, host-owned app request metadata, notification, configuration profile selection, or telemetry routing keys. Codex ignores openai_base_url, chatgpt_base_url, apps_mcp_product_sku, model_provider, model_providers, notify, profile, profiles, experimental_realtime_ws_base_url, and otel when they appear in a project-local .codex/config.toml; put provider, notification, and telemetry keys in user-level config instead.
```

[codex-config-reference]

OpenCode 2 の policies は実験的な機能で、`opencode.json` の `experimental.policies` に書く [opencode-policies]。

```
If policies from both locations match the same provider, your global policy takes priority over the project policy. This prevents a repository from re-enabling a provider that you deny globally.
```

[opencode-policies]

T3 Code のポリシーは、動ける provider を ID で列挙する。

```
`providers.allowedDrivers` lists the providers that may run, by ID: `codex` (Codex),
`claudeAgent` (Claude), `cursor` (Cursor), `grok` (Grok), `opencode` (OpenCode), `antigravity`
(Antigravity), `fx` (fx), `kiro` (Kiro), `pi` (Pi), `muse` (Muse Code), and `acpRegistry` (ACP
Registry).
```

```
`t3 policy check` rejects an ID this version does not know. Threads on other providers stay,
marked unavailable, and users can still turn those providers off.
```

[t3-managed-policy]

Devin CLI は、プロジェクトの設定で使える項目を限っている。

```
Options marked with User only can only be set in the user config (~/.config/devin/config.json; %APPDATA%\devin\config.json on Windows). Only permissions, read_config_from, and hooks are available in project configs. mcpServers can also be set at both levels, but lives in the dedicated mcp_config.json files (see mcpServers).
```

[devin-config]

## 3. 設定の層構造

| ツール | 層 | どの値が効いているか |
|---|---|---|
| Claude Code | managed settings、コマンドラインの `--settings`、`.claude/settings.local.json`、`.claude/settings.json`、`~/.claude/settings.json` の 5 層。上の層が下の層を上書きする [claude-settings] | 環境変数はこの層とは別に、変数と設定キーの組ごとに決まる [claude-settings] |
| T3 Code | プロジェクトの上書き、環境（機械）の設定、`t3.json`、組み込みの既定値の順 [t3-project-settings] | 画面のアイコンが出所（既定値、環境、プロジェクトの上書き）を示す [t3-project-settings] |
| Codex CLI | プロファイルのファイルを `$CODEX_HOME` に置き、`--profile` で 1 つ選ぶ [codex-config-reference] | 選んだプロファイル 1 つ |
| OmO | ユーザー層が最も弱く、最も近いプロジェクトファイルが勝つ [omo-configuration]。プロファイルは `OMO_PROFILE` などで選ぶ [omo-configuration] | 該当なし |
| GSD | プロジェクトごとの設定が全体の既定値より優先される [gsd-configuration] | 該当なし |

Claude Code の下位 3 層は次のとおりである。

```
3. Project local settings (.claude/settings.local.json): your personal settings for this project.
4. Shared project settings (.claude/settings.json): settings checked into source control.
5. User settings (~/.claude/settings.json): your personal settings for every project.
```

[claude-settings]

settings ファイルの中の `env` は、層とは別の仕組みではなく、上の層に従う。

```
An env block inside a settings file is an ordinary key and follows the levels above.
```

[claude-settings]

T3 Code は層の順序を 1 つに決めており、複数の環境への一括編集は同期ではない。

```
Settings a repository can also declare in `t3.json`, such as the workspace for new threads, resolve in one order: a project override, then the environment setting, then `t3.json`, then the built-in default.
```

```
Offline environments keep their current values; this is a bulk edit, not a synced global default.
```

[t3-project-settings]

管理者ポリシーについては、ソースをマージせず、両方あれば起動を拒む。

```
T3 Code does not merge sources. If a machine has both a native policy and a policy file, T3 Code refuses to start until you remove one.
```

[t3-managed-policy]

Codex CLI のプロファイルは、ファイル名で区切り、フラグで選ぶ。

```
Config profile files live next to config.toml as $CODEX_HOME/profile-name.config.toml; select one with --profile profile-name.
```

[codex-config-reference]

OmO の層とプロファイルは次のとおりである。

```
1. User layer (lowest precedence): `~/.omo/omo.jsonc` on every platform (`omo.json` is accepted as a fallback basename).
```

```
Farther ancestors merge first, so the nearest project file wins and beats the user layer.
```

```
Activating a profile that does not exist produces a diagnostic and falls back to the base configuration.
```

[omo-configuration]

GSD は、新規プロジェクトの作成時に全体の既定値を取り込み、以後はプロジェクトごとの設定を優先する。

```
When `/gsd-new-project` creates a new `config.json`, it reads global defaults and merges them as the starting configuration. Per-project settings always override globals.
```

[gsd-configuration]

## 4. fnox の age による秘密の共有

fnox はシークレットを管理するツールである（`docs/dev-environment.md` の「ツール構成と採否」）。age プロバイダーは、値を fnox の設定ファイルの中に age で暗号化して置く。

```
The `age` provider encrypts values into `fnox.toml` with age. Anyone with a matching identity (an age key, a supported SSH private key, or an age plugin identity) can decrypt them, offline and without a cloud account. `fnox set` writes new ciphertext.
```

[fnox-age]

受取人には公開鍵だけを並べる。`fnox set` は受取人全員の公開鍵で暗号化する。

```
`recipients` takes public keys only.
```

```
`fnox set` encrypts the value to every recipient and stores the base64-encoded age file as the secret's `value`:
```

[fnox-age]

受取人を追加しても既存の暗号文は更新されないため、暗号化し直す必要がある。

```
Adding a recipient does not update existing ciphertext. After adding the new public key to `recipients`, re-encrypt:
```

[fnox-age]

受取人を外す場合も暗号化し直す。ただし古い暗号文へのアクセスは取り消せないため、アクセスを絶つには秘密そのものを差し替える。

```
Remove their recipient and run `fnox reencrypt` for each affected profile. Re-encryption cannot revoke access to old ciphertext in git history; rotate the underlying secret if that person's access must end.
```

[fnox-age]

`key_file` のパスは、`~` で始まれば home に展開され、絶対パスはそのまま使われる。

```
Relative `key_file` paths are resolved from the config file that declares the provider. Paths beginning with `~` expand to your home directory, and absolute paths are used unchanged.
```

[fnox-age]

Pelway では、実装担当・レビュー担当・QA 担当の GitHub App の秘密を、それぞれ別の fnox 設定に age で暗号化して置く（`scripts/agent-gh` の冒頭のコメント、`docs/dev-environment.md` の「レビュー担当の秘密ファイルの遮断と鍵の分割（#449）」）。レビュー用の設定の `key_file` は絶対パスで書かれている（`docs/dev-environment.md` の「鍵の分割」）。

## 5. GitHub Projects のユーザーレベルのプロジェクトの共同作業者

ユーザーが所有する Projects には、個人の共同作業者を招待できる。

```
Admins of user-level projects can invite individual collaborators and manage their access.
```

[gh-projects-access]

共同作業者の役割には Read、Write、Admin がある。

```
Write: The individual can view and edit the project.
```

[gh-projects-access]

Projects への招待だけでは、項目（Issue）を読めるとは限らない。項目が属するリポジトリの権限も要る。

```
This only affects collaborators for your project, not for repositories in your project. To view an item on the project, someone must have the required permissions for the repository that the item belongs to.
```

[gh-projects-access]

Pelway のボードは `users/hatai/projects/7` で、Status の読み書きは人間の名義の `gh` で行う（`docs/superpowers/specs/agent-team.md` の「状態の表し方」と「制約」）。`gh` から共同作業者が Status を書けるかは確認していない。

## 6. Pelway の現状と、採用すること・採用しないこと

### 現状

- 並行数の上限は文書の文章にだけある（`docs/superpowers/specs/agent-team.md` の「制約」、`docs/agents/grokbot-runbook.md` の「3. 同時数と順番待ち」、`docs/agents/command-team-details.md` の「調査（入口B）」）。使用数を計算するコードは無い。`scripts/` に `agent:devin` を扱う箇所がなく、`agent:working` は `scripts/conductor-watch.sh` の「誰待ちか」の判定にだけ現れる。
- 指揮役はネイティブの `t3_thread_launch` で、Grok Bot（クラウドで動く指揮役）は T3 Connect の MCP で、作業スレッドを直接立てる（`docs/superpowers/specs/agent-team.md` の「制約」）。
- Grok Bot のルーティンとメモリには「この runbook を読んで従う」とだけ書く（`docs/superpowers/specs/agent-team.md` の「指揮役の運用ルール」）。

### 採用すること

| 決定 | 根拠 |
|---|---|
| 上限と利用可否を、リポジトリで共有する 1 つの設定ファイルに置く | 指揮役には Grok Bot（クラウドで動く）と T3 のスレッドの 2 つの経路がある。Grok Bot が機械の中の設定ファイルを読む経路は、`docs/superpowers/specs/agent-team.md` に書かれていない。層を重ねているツールは、出所の表示（T3 Code）や、リポジトリ側から変えられるキーの制限（Codex CLI、OpenCode 2）を別に持つ。Pelway では層を 1 つにして、出所の確認が要らないようにする |
| 利用可否は、上限とは別の情報で表す | 0 の意味がツールごとに違う。OmO は無制限、Codex CLI は 0 を指定できない、gptme は 1 未満を無視する、Claude Code は変数では無効にできない。利用可否は、各ツールとも上限とは別の仕組みで表している |
| 上限は 1 以上の整数として検証する | gptme と Codex CLI は 1 未満を受け付けない |
| 上限を読む主体をコードに置く | GSD は文書に上限のキーがありながら実装が読んでおらず、上限が機能していない |
| 使用数を計算して知らせ、起動は拒まない | Claude Code は Agent ツールが、Codex CLI はホスト側の容量管理が、起動を拒む。Pelway の指揮役はスレッドを直接立てるため、間に入る場所が無い。T3 Code の文書にも並行数の上限は無い |

### 採用しないこと

- **起動時のフラグや環境変数で選ぶプロファイル**（Codex CLI の `--profile`、OmO の `OMO_PROFILE`）。Pelway では設定を読む主体が開発者の login 名で表を選ぶ形にするため、起動時の指定を要する方式は合わせない。
- **管理者ポリシー**（T3 Code のポリシーファイル）。Pelway では設定の検証を `mise run verify` の試験で行い、ポリシーの配布と起動時の拒否は作らない。

## 出典

- [gptme-concurrency] https://github.com/gptme/gptme/blob/5dda53370184/gptme/tools/subagent/concurrency.py （上限の解決順序、1 未満の扱い、セマフォ）
- [omo-configuration] https://github.com/code-yeongyu/oh-my-openagent/blob/2a410286d455/docs/reference/configuration.md （層の優先順位、`0` の意味、プロファイル）
- [omo-concurrency] https://github.com/code-yeongyu/oh-my-openagent/blob/2a410286d455/packages/omo-opencode/src/features/background-agent/concurrency.ts （上限の解決と待ち行列）
- [omo-schema] https://github.com/code-yeongyu/oh-my-openagent/blob/2a410286d455/packages/omo-opencode/src/config/schema/background-task.ts （上限のスキーマ）
- [claude-settings] https://code.claude.com/docs/en/settings#settings-precedence （設定の優先順位、`availableModels` と `deniedModels`）
- [claude-env-vars] https://code.claude.com/docs/en/env-vars （`CLAUDE_CODE_MAX_CONCURRENT_SUBAGENTS`、`CLAUDE_CODE_WORKFLOW_MAX_CONCURRENT_AGENTS`）
- [claude-sub-agents] https://code.claude.com/docs/en/sub-agents#concurrent-subagent-limit （上限に達したときのエラー）
- [codex-config-reference] https://developers.openai.com/codex/config-reference （`agents.max_concurrent_threads_per_session`、プロジェクト側で無視されるキー、プロファイル。2026-11-25 時点で `https://learn.chatgpt.com/docs/config-file/config-reference` へ転送される）
- [codex-schema] https://github.com/openai/codex/blob/5a2cb7358038/codex-rs/core/config.schema.json （上限の `minimum`）
- [codex-admission] https://github.com/openai/codex/blob/5a2cb7358038/codex-rs/core/src/agent/host/admission.rs （容量が尽きたときの規約のコメント）
- [gsd-configuration] https://github.com/gsd-build/get-shit-done/blob/bdcaab2c752d/docs/CONFIGURATION.md （並列実行の設定の表、全体の既定値とプロジェクトの設定）
- [gsd-config-ts] https://github.com/gsd-build/get-shit-done/blob/bdcaab2c752d/sdk/src/config.ts （`parallelization` の型）
- [gsd-core-cjs] https://github.com/gsd-build/get-shit-done/blob/bdcaab2c752d/get-shit-done/bin/lib/core.cjs （`parallelization` の正規化）
- [gsd-phase-runner] https://github.com/gsd-build/get-shit-done/blob/bdcaab2c752d/sdk/src/phase-runner.ts （並列実行の箇所）
- [gsd-template-config] https://github.com/gsd-build/get-shit-done/blob/bdcaab2c752d/get-shit-done/templates/config.json （設定のテンプレート）
- [opencode-policies] https://github.com/anomalyco/opencode/blob/66d53a82d03e/services/site/src/docs/v1/content/policies.mdx （OpenCode 2 の policies）
- [t3-project-settings] https://github.com/pingdotgg/t3code/blob/8b70e7337160/docs/user/project-settings.md （設定の層の順序、provider は機械ごと、プロジェクトの上書き）
- [t3-managed-policy] https://github.com/pingdotgg/t3code/blob/8b70e7337160/docs/user/managed-policy.md （`providers.allowedDrivers`、ソースをマージしない）
- [t3-docs] https://github.com/pingdotgg/t3code/tree/8b70e7337160/docs （`docs/` 全体の検索。並行数の上限を設定する項目は見当たらない）
- [devin-config] https://docs.devin.ai/cli/reference/configuration/config-file （user only の項目と、プロジェクトの設定で使える項目）
- [fnox-age] https://fnox.jdx.dev/providers/age （age プロバイダーの受取人、再暗号化、`key_file`）
- [gh-projects-access] https://docs.github.com/en/issues/planning-and-tracking-with-projects/managing-your-project/managing-access-to-your-projects （ユーザーレベルのプロジェクトの共同作業者）
- リポジトリ内: `docs/superpowers/specs/agent-team.md`、`scripts/conductor-watch.sh`、`scripts/agent-gh`、`docs/dev-environment.md`（「ツール構成と採否」「レビュー担当の秘密ファイルの遮断と鍵の分割（#449）」）
