#!/usr/bin/env python3
"""notify-grokbot.yml の run: を抽出し、偽 gh/curl で実行するテストハーネス。"""
import copy
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
WORKFLOW = REPO / ".github" / "workflows" / "notify-grokbot.yml"
SHA = "a" * 40
OTHER_SHA = "b" * 40
PR = 42
OWN_ID = 999000111
EXPECTED_RUN_NAME = """run-name: >-
  ${{
    (
      (github.event_name == 'pull_request' || github.event_name == 'pull_request_review') &&
      format('notify #{0} {1}', github.event.pull_request.number, github.event.action)
    ) ||
    format('notify {0} {1}', github.event_name, github.run_id)
  }}"""


def step_blocks(path):
    lines = path.read_text().splitlines()
    starts = []
    for i, line in enumerate(lines):
        m = re.match(r"^(\s*)- name: (.+?)\s*$", line)
        if m:
            starts.append((i, len(m.group(1)), m.group(2)))
    out = {}
    for pos, (start, indent, name) in enumerate(starts):
        end = len(lines)
        for nstart, nindent, _ in starts[pos + 1:]:
            if nindent == indent:
                end = nstart
                break
        block = lines[start:end]
        if_line = next((x.strip()[4:].strip() for x in block if x.strip().startswith("if: ")), "")
        run_body = None
        for j, line in enumerate(block):
            if re.match(r"^\s*run: \|\s*$", line):
                run_indent = len(line) - len(line.lstrip())
                body_lines = []
                for raw in block[j + 1:]:
                    cur_indent = len(raw) - len(raw.lstrip())
                    if raw.strip() and cur_indent <= run_indent:
                        break
                    body_lines.append(raw)
                nonempty = [len(x) - len(x.lstrip()) for x in body_lines if x.strip()]
                cut = min(nonempty) if nonempty else run_indent + 2
                run_body = "\n".join(x[cut:] if len(x) >= cut else "" for x in body_lines) + "\n"
                break
        if name in out:
            raise ValueError(f"ambiguous step name: {name}")
        out[name] = {"if": if_line, "run": run_body or ""}
    return out


def extract(path, outdir):
    target = Path(outdir)
    target.mkdir(parents=True, exist_ok=True)
    steps = step_blocks(Path(path))
    for name in ("Check duplicate notification", "Send notification", "Record delivery"):
        body = steps.get(name, {}).get("run", "")
        if not body:
            raise SystemExit(f"missing run block: {name}")
        safe = {"Check duplicate notification": "dedup.sh", "Send notification": "send.sh", "Record delivery": "record.sh"}[name]
        (target / safe).write_text(body)
    meta = {name: {"if": steps.get(name, {}).get("if", "")} for name in steps}
    (target / "steps.json").write_text(json.dumps(meta, ensure_ascii=False, indent=2))


def run_obj(i, title, sha=SHA, created=None, event="pull_request"):
    return {
        "id": i,
        "event": event,
        "head_sha": sha,
        "display_title": title,
        "created_at": created or f"2026-01-01T00:{i % 60:02d}:00Z",
    }


def step(name, conclusion):
    return {"name": name, "status": "completed", "conclusion": conclusion, "number": 1}


def job(steps):
    return {"id": 1, "run_id": 1, "name": "notify-grokbot", "status": "completed", "conclusion": "success", "steps": steps}


def runs_pages(pages):
    total = sum(len(p) for p in pages)
    return [{"total_count": total, "workflow_runs": p} for p in pages]


def jobs_pages(pages):
    total = len(pages)
    return [{"total_count": total, "jobs": p} for p in pages]


def empty_jobs():
    return jobs_pages([[job([])]])


def positive_jobs():
    return jobs_pages([[job([step("Record delivery", "success")])]])


def fake_gh():
    args = sys.argv[1:]
    scenario_path = os.environ.get("FAKE_GH_SCENARIO")
    state_path = Path(os.environ.get("FAKE_GH_STATE", "/tmp/fake-gh-state.json"))
    log_path = os.environ.get("FAKE_GH_LOG")
    if not scenario_path:
        print("FAKE_GH_SCENARIO is unset", file=sys.stderr)
        return 2
    scenario = json.loads(Path(scenario_path).read_text())
    endpoint = ""
    paginate = "--paginate" in args
    for a in args:
        if a.startswith("repos/"):
            endpoint = a
    if log_path:
        with open(log_path, "a") as f:
            f.write(json.dumps({"args": args, "endpoint": endpoint, "paginate": paginate}) + "\n")
    if "/actions/workflows/notify-grokbot.yml/runs" in endpoint:
        key = "runs"
    else:
        m = re.search(r"/actions/runs/(\d+)/jobs", endpoint)
        key = f"jobs:{m.group(1)}" if m else "unknown"
    state = json.loads(state_path.read_text()) if state_path.exists() else {}
    state[key] = state.get(key, 0) + 1
    state_path.write_text(json.dumps(state))
    spec = scenario.get(key, {"pages": jobs_pages([[job([])]]) if key.startswith("jobs:") else runs_pages([[]])})
    if state[key] <= spec.get("fail", 0):
        print("fake gh failure", file=sys.stderr)
        return 1
    if spec.get("malformed"):
        print("{not-json")
        return 0
    if spec.get("missing_array"):
        print(json.dumps({"total_count": 1}))
        return 0
    if spec.get("empty_output"):
        return 0
    pages = spec.get("pages", [])
    if not pages:
        pages = runs_pages([[]]) if key == "runs" else jobs_pages([[job([])]])
    selected = pages if paginate else pages[:1]
    sys.stdout.write("\n".join(json.dumps(p) for p in selected) + "\n")
    return 0


def fake_curl():
    log = os.environ.get("FAKE_CURL_LOG")
    if log:
        Path(log).write_text(json.dumps(sys.argv[1:]))
    sys.stdin.read()
    code = int(os.environ.get("FAKE_CURL_EXIT", "0"))
    if code:
        return code
    sys.stdout.write(os.environ.get("FAKE_CURL_HTTP_CODE", "200"))
    return 0


def fake_date():
    if sys.argv[1:] != ["+%s"]:
        sys.stdout.write(subprocess.check_output(["/usr/bin/date"] + sys.argv[1:], text=True))
        return 0
    state_path = Path(os.environ["FAKE_DATE_STATE"])
    n = int(state_path.read_text()) if state_path.exists() else 0
    state_path.write_text(str(n + 1))
    start = int(os.environ.get("FAKE_DATE_START", "1700000000"))
    step_n = int(os.environ.get("FAKE_DATE_STEP", "0"))
    print(start + n * step_n)
    return 0


def write_exe(path, text):
    path.write_text(text)
    path.chmod(0o755)


class Harness:
    def __init__(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="notify-harness-"))
        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.extracted = self.tmp / "steps"
        extract(WORKFLOW, self.extracted)
        harness = Path(__file__).resolve()
        for name, cmd in (("gh", "fake-gh"), ("curl", "fake-curl"), ("date", "fake-date")):
            write_exe(self.bin / name, f"#!/usr/bin/env bash\nexec python3 -I {harness} {cmd} \"$@\"\n")
        write_exe(self.bin / "sleep", "#!/usr/bin/env bash\nexit 0\n")
        self.failures = []

    def cleanup(self):
        shutil.rmtree(self.tmp)

    def check(self, name, ok, detail=""):
        print(("ok   " if ok else "FAIL ") + name + (f" ({detail})" if detail and not ok else ""))
        if not ok:
            self.failures.append(name)

    def env(self, case, run_id=OWN_ID):
        env = os.environ.copy()
        env.update({
            "PATH": str(self.bin) + os.pathsep + env.get("PATH", ""),
            "FAKE_GH_SCENARIO": str(case / "scenario.json"),
            "FAKE_GH_STATE": str(case / "gh-state.json"),
            "FAKE_GH_LOG": str(case / "gh-calls.jsonl"),
            "FAKE_DATE_STATE": str(case / "date-state"),
            "FAKE_DATE_START": "1700000000",
            "GH_TOKEN": "dummy-token",
            "GITHUB_REPOSITORY": "example-org/pelway",
            "GITHUB_RUN_ID": str(run_id),
            "DEDUP_PR": str(PR),
            "DEDUP_SHA": SHA,
        })
        return env

    def scenario(self, name, data, date_step="0", run_id=OWN_ID):
        case = self.tmp / name
        case.mkdir()
        (case / "scenario.json").write_text(json.dumps(data))
        env = self.env(case, run_id)
        env["FAKE_DATE_STEP"] = date_step
        output = case / "github-output"
        env["GITHUB_OUTPUT"] = str(output)
        p = subprocess.run(["bash", str(self.extracted / "dedup.sh")], env=env, text=True, capture_output=True)
        text = output.read_text() if output.exists() else ""
        calls = []
        log = case / "gh-calls.jsonl"
        if log.exists():
            calls = [json.loads(x) for x in log.read_text().splitlines()]
        return {"rc": p.returncode, "stdout": p.stdout, "stderr": p.stderr, "output": text, "duplicate": "duplicate=true" in text, "calls": calls}

    def run_dedup_tests(self):
        title = f"notify #{PR} synchronize"
        empty_scenario = {"runs": {"pages": runs_pages([[]])}, f"jobs:{OWN_ID}": {"pages": empty_jobs()}}
        r = self.scenario("no-history", empty_scenario)
        self.check("dedup no history sends", r["rc"] == 0 and not r["duplicate"], r["stderr"])

        cand = run_obj(1001, title)
        s = copy.deepcopy(empty_scenario)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:1001"] = {"pages": positive_jobs()}
        r = self.scenario("positive", s)
        self.check("dedup exact record skips", r["duplicate"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        page1 = [run_obj(2000 + i, f"notify #{PR} opened") for i in range(100)]
        cand = run_obj(2101, title)
        s["runs"] = {"pages": runs_pages([page1, [cand]])}
        s["jobs:2101"] = {"pages": positive_jobs()}
        r = self.scenario("run-page2", s)
        self.check("dedup finds marker on run page 2", r["duplicate"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(2201, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:2201"] = {"pages": jobs_pages([[job([])], [job([step("Record delivery", "success")])]])}
        r = self.scenario("jobs-page2", s)
        self.check("dedup finds marker on jobs page 2", r["duplicate"], r["stderr"])

        s = {"runs": {"pages": runs_pages([[]])}, f"jobs:{OWN_ID}": {"pages": positive_jobs()}}
        r = self.scenario("own-attempt", s)
        self.check("dedup own previous attempt skips", r["duplicate"], r["stderr"])
        self.check("dedup own positive does not list runs", not any("/actions/workflows/" in c["endpoint"] for c in r["calls"]))

        s = copy.deepcopy(empty_scenario)
        others = [run_obj(3001, f"notify #43 synchronize"), run_obj(3002, f"notify #{PR} opened"), run_obj(3003, "legacy title"), run_obj(3004, f"notify #{PR} ready_for_review"), run_obj(3005, f"notify #{PR} closed")]
        s["runs"] = {"pages": runs_pages([others])}
        for x in others:
            s[f"jobs:{x['id']}"] = {"pages": positive_jobs()}
        r = self.scenario("nonmatching", s)
        self.check("dedup ignores other PR/action/legacy", not r["duplicate"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3101, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3101"] = {"pages": jobs_pages([[job([step("Send notification", "success")])]])}
        r = self.scenario("legacy-send", s)
        self.check("dedup ignores legacy Send success", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3201, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3201"] = {"pages": jobs_pages([[job([step("Record delivery", "failure")])]])}
        r = self.scenario("record-failure", s)
        self.check("dedup ignores failed Record delivery", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3202, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3202"] = {"pages": jobs_pages([[job([step("Record delivery", "cancelled")])]])}
        r = self.scenario("record-cancelled", s)
        self.check("dedup ignores cancelled Record delivery", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        s[f"jobs:{OWN_ID}"]["fail"] = 9
        cand = run_obj(3301, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3301"] = {"pages": positive_jobs()}
        r = self.scenario("own-fail-positive", s)
        self.check("dedup positive wins after own jobs failure", r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        c1 = run_obj(3401, title)
        c2 = run_obj(3402, title, created="2026-01-02T00:00:00Z")
        s["runs"] = {"pages": runs_pages([[c1, c2]])}
        s["jobs:3402"] = {"fail": 9}
        s["jobs:3401"] = {"pages": positive_jobs()}
        r = self.scenario("jobs-fail-positive", s)
        ordered_job_calls = [c["endpoint"] for c in r["calls"] if "/actions/runs/" in c["endpoint"] and str(OWN_ID) not in c["endpoint"]]
        first_newer = next((i for i, x in enumerate(ordered_job_calls) if f"/actions/runs/{c2['id']}/jobs" in x), -1)
        first_older = next((i for i, x in enumerate(ordered_job_calls) if f"/actions/runs/{c1['id']}/jobs" in x), -1)
        self.check("dedup continues after candidate jobs failure", r["duplicate"], r["stderr"])
        self.check("dedup scans candidates newest first", first_newer == 0 and first_older > first_newer, str(ordered_job_calls))

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3501, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3501"] = {"fail": 9}
        r = self.scenario("jobs-fail-open", s)
        self.check("dedup sends after candidate jobs failure without positive", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3601, title)
        s["runs"] = {"pages": runs_pages([[cand]]), "fail": 2}
        s["jobs:3601"] = {"pages": positive_jobs()}
        r = self.scenario("list-retry", s)
        self.check("dedup retries failed run list", r["duplicate"], r["stderr"])
        self.check("dedup successful retry does not warn", "::warning::" not in r["stderr"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        s["runs"]["fail"] = 9
        r = self.scenario("list-fail-open", s)
        self.check("dedup sends after persistent run list failure", not r["duplicate"])
        self.check("dedup persistent failure warns without endpoint", "::warning::" in r["stderr"] and "repos/" not in r["stderr"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        s["runs"]["malformed"] = True
        r = self.scenario("malformed", s)
        self.check("dedup treats malformed JSON as failure", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        many = [run_obj(4000 + i, f"notify #{PR} opened") for i in range(1000)]
        pages = [many[i:i + 100] for i in range(0, 1000, 100)]
        s["runs"] = {"pages": runs_pages(pages)}
        r = self.scenario("cap", s)
        self.check("dedup sends at 1000-item cap without positive", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        s["runs"] = {"pages": runs_pages([[]])}
        r = self.scenario("budget", s, date_step="200")
        self.check("dedup sends after time budget", not r["duplicate"])
        self.check("dedup time budget warns", "::warning::" in r["stderr"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3701, title, sha=OTHER_SHA)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3701"] = {"pages": positive_jobs()}
        r = self.scenario("different-sha", s)
        self.check("dedup ignores different head sha", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        malformed_run = {"id": "3801", "event": "pull_request", "head_sha": SHA}
        s["runs"] = {"pages": runs_pages([[malformed_run]])}
        r = self.scenario("malformed-entry", s)
        self.check("dedup malformed run entry stays fail-open", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        s[f"jobs:{OWN_ID}"]["fail"] = 9
        r = self.scenario("own-fail-open", s)
        self.check("dedup sends after own jobs failure without positive", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        s["runs"]["empty_output"] = True
        r = self.scenario("empty-runs-output", s)
        self.check("dedup treats empty run-list output as failure", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        s["runs"]["missing_array"] = True
        r = self.scenario("missing-runs-array", s)
        self.check("dedup treats missing workflow_runs array as failure", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(3901, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:3901"] = {"missing_array": True}
        r = self.scenario("missing-jobs-array", s)
        self.check("dedup treats missing jobs array as failure", not r["duplicate"])

        s = copy.deepcopy(empty_scenario)
        positive = run_obj(4000, title, created="2026-01-02T00:00:00Z")
        many = [positive] + [run_obj(4001 + i, f"notify #{PR} opened") for i in range(999)]
        pages = [many[i:i + 100] for i in range(0, 1000, 100)]
        s["runs"] = {"pages": runs_pages(pages)}
        s["jobs:4000"] = {"pages": positive_jobs()}
        r = self.scenario("cap-positive", s)
        self.check("dedup confirmed positive wins at 1000-item cap", r["duplicate"], r["stderr"])

        s = copy.deepcopy(empty_scenario)
        cand = run_obj(4101, title)
        s["runs"] = {"pages": runs_pages([[cand]])}
        s["jobs:4101"] = {"pages": positive_jobs()}
        r = self.scenario("positive-before-budget", s, date_step="20")
        self.check("dedup positive before time budget skips", r["duplicate"], r["stderr"])

        first = copy.deepcopy(empty_scenario)
        r1 = self.scenario("repeat-first", first)
        progressed = copy.deepcopy(first)
        first_run = run_obj(OWN_ID, title)
        progressed["runs"] = {"pages": runs_pages([[first_run]])}
        progressed[f"jobs:{OWN_ID}"] = {"pages": positive_jobs()}
        r2 = self.scenario("repeat-second", progressed, run_id=OWN_ID + 1)
        self.check("dedup repeated same head sends first and skips second", not r1["duplicate"] and r2["duplicate"], f"first={r1['output']} second={r2['output']}")

        calls = r["calls"] or self.scenario("query-shape", empty_scenario)["calls"]
        run_calls = [c for c in calls if "/actions/workflows/" in c["endpoint"]]
        job_calls = [c for c in calls if "/actions/runs/" in c["endpoint"]]
        qshape = self.scenario("query-shape2", empty_scenario)
        run_calls = [c for c in qshape["calls"] if "/actions/workflows/" in c["endpoint"]]
        job_calls = [c for c in qshape["calls"] if "/actions/runs/" in c["endpoint"]]
        endpoint = run_calls[0]["endpoint"] if run_calls else ""
        self.check("dedup run query has required filters", all(x in endpoint for x in ("event=pull_request", "status=completed", "head_sha=", "per_page=100", "exclude_pull_requests=true")), endpoint)
        self.check("dedup jobs query has required filters", bool(job_calls) and all("filter=all" in c["endpoint"] and "per_page=100" in c["endpoint"] for c in job_calls))
        self.check("dedup uses gh api --paginate", bool(qshape["calls"]) and all(c["paginate"] for c in qshape["calls"]))

    def run_send_tests(self):
        meta = json.loads((self.extracted / "steps.json").read_text())
        self.check("dedup if is exact", meta.get("Check duplicate notification", {}).get("if") == "github.event.action == 'synchronize' && steps.gate.outputs.should_send == 'true'", meta.get("Check duplicate notification", {}).get("if", ""))
        self.check("Send if is exact", meta.get("Send notification", {}).get("if") == "steps.gate.outputs.should_send == 'true' && env.COMMENT_ALLOWED == 'true' && !(github.event.action == 'synchronize' && steps.dedup.outputs.duplicate == 'true')", meta.get("Send notification", {}).get("if", ""))
        self.check("Record delivery if is exact", meta.get("Record delivery", {}).get("if") == "steps.send.outputs.delivered == 'true'", meta.get("Record delivery", {}).get("if", ""))
        for label, curl_exit, http, want in (
            ("curl-exit", "7", "000", False),
            ("http-500", "0", "500", False),
            ("http-301", "0", "301", False),
            ("http-200", "0", "200", True),
            ("http-201", "0", "201", True),
            ("http-204", "0", "204", True),
        ):
            case = self.tmp / f"send-{label}"
            case.mkdir()
            out = case / "output"
            env = os.environ.copy()
            env.update({
                "PATH": str(self.bin) + os.pathsep + env.get("PATH", ""),
                "FAKE_CURL_EXIT": curl_exit,
                "FAKE_CURL_HTTP_CODE": http,
                "FAKE_CURL_LOG": str(case / "curl-args"),
                "GITHUB_OUTPUT": str(out),
                "GITHUB_SERVER_URL": "https://github.com",
                "GITHUB_REPOSITORY": "example-org/pelway",
                "GITHUB_RUN_ID": str(OWN_ID),
                "REPO": "example-org/pelway",
                "EVENT": "pull_request",
                "ACTION": "synchronize",
                "PR_NUMBER": str(PR),
                "HEAD_BRANCH": "feature/demo",
                "HEAD_SHA": SHA,
                "MERGED": "false",
                "CONCLUSION": "",
                "WORKFLOW_NAME": "",
                "RUN_URL": "",
                "REVIEW_STATE": "",
                "ACTOR": "tester",
                "WEBHOOK_URL": "https://secret.example/WEBHOOK-CANARY",
                "WEBHOOK_AUTH": "Bearer AUTH-CANARY",
            })
            p = subprocess.run(["bash", str(self.extracted / "send.sh")], env=env, text=True, capture_output=True)
            got = out.read_text() if out.exists() else ""
            ok = p.returncode == 0 and f"delivered={'true' if want else 'false'}" in got
            self.check(f"send {label} delivered={want}", ok, p.stderr + got)
            leak = "WEBHOOK-CANARY" in p.stdout + p.stderr + got or "AUTH-CANARY" in p.stdout + p.stderr + got
            self.check(f"send {label} does not leak secrets", not leak)
        rec = subprocess.run(["bash", str(self.extracted / "record.sh")], text=True, capture_output=True)
        self.check("Record delivery prints only fixed confirmation", rec.returncode == 0 and "Delivery confirmed by HTTP 2xx." in rec.stdout and "CANARY" not in rec.stdout)

    def run_static_tests(self):
        text = WORKFLOW.read_text()
        self.check("workflow has exact run-name", EXPECTED_RUN_NAME in text)
        self.check("run-name does not use PR title", "pull_request.title" not in text)
        self.check("workflow keeps actions read permission", re.search(r"^permissions:\n  actions: read\n(?=\n)", text, re.M) is not None)
        self.check("workflow still has no checkout or uses", "actions/checkout" not in text and not re.search(r"^\s+uses:", text, re.M))
        rec_section = text[text.find("- name: Record delivery"):]
        self.check("Record delivery has no continue-on-error", "continue-on-error" not in rec_section)
        self.check("Send notification has id send", re.search(r"- name: Send notification\n\s+id: send", text) is not None)

    def run(self):
        try:
            self.run_static_tests()
            self.run_dedup_tests()
            self.run_send_tests()
            if self.failures:
                print(f"notify_harness: {len(self.failures)} check(s) failed", file=sys.stderr)
                return 1
            print(f"notify_harness: all checks passed")
            return 0
        finally:
            self.cleanup()


def main():
    if len(sys.argv) < 2:
        print("usage: notify_harness.py extract|fake-gh|fake-curl|fake-date|run-tests ...", file=sys.stderr)
        return 2
    cmd = sys.argv[1]
    sys.argv = [sys.argv[0]] + sys.argv[2:]
    if cmd == "extract":
        if len(sys.argv) != 3:
            print("usage: notify_harness.py extract <workflow> <outdir>", file=sys.stderr)
            return 2
        extract(Path(sys.argv[1]), Path(sys.argv[2]))
        return 0
    if cmd == "fake-gh":
        return fake_gh()
    if cmd == "fake-curl":
        return fake_curl()
    if cmd == "fake-date":
        return fake_date()
    if cmd == "run-tests":
        return Harness().run()
    print(f"unknown command: {cmd}", file=sys.stderr)
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
