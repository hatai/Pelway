#!/usr/bin/env python3
"""Claude Design 書き出し照合 v3（Round2）。
v2 を拡張: contrast-pairs の decorative / known-exception / retarget-text、
個人名禁止、Round2 追加トークン。

使い方:
  python3 check_export_v3.py /path/to/pelway.tokens.json
  # contrast-pairs.json は tokens と同じディレクトリを自動探索
  # design-system.md があれば個人名も検査

期待値: 親ディレクトリの expected-values.json
標準ライブラリのみ。check_export_v2.py は変更しない。
"""
from __future__ import annotations
import json, sys, re
from pathlib import Path

HERE = Path(__file__).resolve().parent
EXPECT = HERE.parent / 'expected-values.json'
DEFAULT_TOKENS = Path('/workspace/pelway-design-system/claude-design-export-2026-11-09/pelway.tokens.json')
WCAG_TOOLS = Path('/workspace/pelway-design-system/ui-pro-review/tools')
sys.path.insert(0, str(WCAG_TOOLS))
from wcag import cr  # noqa: E402

# --- reuse v2 helpers by import if possible; else inline minimal ---
V2 = HERE.parent.parent / 'claude-design-fix-instructions-2026-11-09' / 'tools' / 'check_export_v2.py'
import importlib.util
spec = importlib.util.spec_from_file_location('check_export_v2', V2)
v2 = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v2)

load = v2.load
count_dollar_value = v2.count_dollar_value
find_forbidden = v2.find_forbidden
walk_colors_dtcg = v2.walk_colors_dtcg
walk_colors_legacy = v2.walk_colors_legacy
read_brand_hex = v2.read_brand_hex
is_dtcg_token = v2.is_dtcg_token

def ratio(a,b):
    return round(cr(a,b), 2)

def find_contrast_pairs(tokens_path: Path):
    sib = tokens_path.parent / 'contrast-pairs.json'
    if sib.exists():
        return sib
    return None

def check_contrast_policy(pairs, policy, report):
    """Validate kinds and substantive AA.
    Returns list of issues (round2-relevant).
    """
    issues = []
    exceptions = []
    # index pairs
    def key(p):
        return (p.get('fg','').upper(), p.get('bg','').upper(), p.get('theme',''), p.get('kind',''))

    for rule in policy:
        fg, bg = rule['fg'].upper(), rule['bg'].upper()
        exp_kind = rule['expected_kind']
        role = rule.get('role')  # text/nontext optional
        for th in rule['themes']:
            matches = [p for p in pairs
                       if p.get('fg','').upper()==fg and p.get('bg','').upper()==bg and p.get('theme')==th]
            if role:
                matches = [p for p in matches if p.get('kind')==role] or matches
            if not matches:
                # retarget-text: absence after fix is OK; presence with wrong kind is issue
                if exp_kind == 'retarget-text':
                    continue
                if exp_kind == 'known-exception':
                    issues.append({'type':'missing_exception_pair','rule':rule,'theme':th})
                continue
            for p in matches:
                kind = p.get('kind')
                r = ratio(p['fg'], p['bg'])
                mn = float(p.get('min') or (4.5 if kind=='text' else 3))
                if exp_kind == 'decorative':
                    if kind != 'decorative':
                        issues.append({
                            'type':'kind_should_be_decorative',
                            'theme':th,'fg':p['fg'],'bg':p['bg'],'actual_kind':kind,
                            'ratio':r,'min':mn,'maps_to':rule.get('maps_to'),'reason':rule.get('reason'),
                        })
                elif exp_kind == 'known-exception':
                    if kind not in ('known-exception','nontext'):
                        # still accept nontext as "needs rekind" issue
                        issues.append({
                            'type':'kind_should_be_known_exception',
                            'theme':th,'fg':p['fg'],'bg':p['bg'],'actual_kind':kind,'ratio':r,
                            'maps_to':rule.get('maps_to'),'reason':rule.get('reason'),
                        })
                    else:
                        # count as documented exception (not a pass)
                        exceptions.append({
                            'theme':th,'fg':p['fg'],'bg':p['bg'],'kind':kind,'ratio':r,
                            'maps_to':rule.get('maps_to'),'reason':rule.get('reason'),
                        })
                        if kind == 'nontext':
                            issues.append({
                                'type':'kind_should_be_known_exception',
                                'theme':th,'fg':p['fg'],'bg':p['bg'],'actual_kind':kind,'ratio':r,
                                'maps_to':rule.get('maps_to'),'reason':rule.get('reason'),
                            })
                elif exp_kind == 'retarget-text':
                    if kind == 'text':
                        issues.append({
                            'type':'text_uses_fill_blue',
                            'theme':th,'fg':p['fg'],'bg':p['bg'],'ratio':r,'min':mn,
                            'fix':'fg を acc（#8F96FF）にするか、この text 組を削除',
                            'reason':rule.get('reason'),
                        })

    # Any remaining non-decorative non-exception fails
    policy_fgs = {(r['fg'].upper(), r['bg'].upper(), th) for r in policy for th in r['themes']}
    for p in pairs:
        kind = p.get('kind')
        if kind in ('decorative','known-exception'):
            continue
        r = ratio(p['fg'], p['bg'])
        mn = float(p.get('min') or (4.5 if kind=='text' else 3))
        if r >= mn:
            continue
        k = (p['fg'].upper(), p['bg'].upper(), p.get('theme'))
        # if covered by policy as pending fix, already in issues
        covered = any(
            p['fg'].upper()==rule['fg'].upper() and p['bg'].upper()==rule['bg'].upper() and p.get('theme') in rule['themes']
            for rule in policy
        )
        if covered:
            continue
        issues.append({
            'type':'unexpected_aa_fail',
            'theme':p.get('theme'),'fg':p['fg'],'bg':p['bg'],'kind':kind,'ratio':r,'min':mn,
        })

    report['contrast_exceptions_documented'] = exceptions
    report['contrast_issues'] = issues
    return issues

def check_personal_names(paths, banned, report):
    hits = []
    for p in paths:
        if not p or not Path(p).exists():
            continue
        text = Path(p).read_text(encoding='utf-8', errors='ignore')
        for name in banned:
            if name in text:
                hits.append({'file': str(p), 'name': name, 'count': text.count(name)})
    report['personal_name_hits'] = hits
    return hits

def main():
    tokens_path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_TOKENS
    exp = load(EXPECT)
    doc = load(tokens_path)
    report = {
        'tokens_path': str(tokens_path),
        'expected_path': str(EXPECT),
        'ok': True,
        'structure': {},
        'value_mismatches': [],
        'missing_themes': [],
        'missing_keys': [],
        'unreadable_values': [],
        'notes': [],
        'round2': {},
    }

    n_val = count_dollar_value(doc)
    report['structure']['dollar_value_count'] = n_val
    report['structure']['looks_dtcg'] = n_val > 0

    root_forbid = exp['structure']['forbid_root_keys']
    bad_root = [k for k in root_forbid if k in doc]
    report['structure']['forbidden_root_keys_present'] = bad_root
    if bad_root:
        report['ok'] = False

    hits = find_forbidden(doc, {
        'substrings': [s for s in exp['structure']['forbid_value_substrings'] if s != '濱田'],
        'keys': [],
    })
    report['structure']['forbidden_substring_hits'] = hits[:50]
    if hits:
        report['ok'] = False

    unreadable = []
    if n_val > 0:
        got, unreadable = walk_colors_dtcg(doc)
        if unreadable:
            report['unreadable_values'].extend(unreadable)
            report['ok'] = False
    else:
        got = walk_colors_legacy(doc)
        report['notes'].append('DTCGではない')
        report['ok'] = False

    for theme, expect_map in exp['colors'].items():
        if theme not in got:
            report['missing_themes'].append(theme)
            report['ok'] = False
            continue
        for sem, want in expect_map.items():
            have = got.get(theme, {}).get(sem)
            if have is None:
                report['missing_keys'].append(f'{theme}.{sem}')
                report['ok'] = False
                continue
            want_u, have_u = want.upper(), have.upper()
            if len(have_u) == 9 and have_u.endswith('FF') and len(want_u) == 7:
                have_cmp = have_u[:7]
            else:
                have_cmp = have_u
            if have_cmp != want_u:
                report['value_mismatches'].append({
                    'path': f'color.{theme}.{sem}', 'expected': want, 'actual': have,
                })
                report['ok'] = False

    if exp['structure'].get('same_semantic_keys_across_themes') and n_val > 0:
        theme_keys = {}
        color = doc.get('color') or {}
        for th in exp['meta']['themes']:
            group = color.get(th)
            if not isinstance(group, dict):
                continue
            theme_keys[th] = sorted(k for k in group if not k.startswith('$') and is_dtcg_token(group[k]))
        report['structure']['theme_key_counts'] = {t: len(ks) for t, ks in theme_keys.items()}

    if n_val > 0:
        for k, want in exp['brand'].items():
            val, err = read_brand_hex(doc, k)
            if err or not val or val.upper()[:7] != want.upper():
                report['value_mismatches'].append({
                    'path': f'brand.{k}', 'expected': want, 'actual': val, 'error': err,
                })
                report['ok'] = False

    blob = json.dumps(doc)
    report['structure']['has_2B37FF'] = bool(re.search(r'#2[Bb]37[Ff]{2}', blob))
    if report['structure']['has_2B37FF']:
        report['ok'] = False

    # contrast-pairs
    cp_path = find_contrast_pairs(tokens_path)
    contrast_issues = []
    if cp_path:
        pairs = load(cp_path)
        if not isinstance(pairs, list):
            pairs = pairs.get('pairs') or []
        report['contrast_pairs_path'] = str(cp_path)
        contrast_issues = check_contrast_policy(pairs, exp.get('contrast_pairs_policy') or [], report)
        if contrast_issues:
            report['ok'] = False
    else:
        report['notes'].append('contrast-pairs.json なし（スキップ）')

    # personal names in tokens, contrast, sibling design-system.md
    banned = exp['structure'].get('forbid_personal_name_substrings') or []
    name_paths = [tokens_path, cp_path, tokens_path.parent / 'design-system.md']
    # also check original design-system if user regenerates from claude-design — optional path via argv2
    if len(sys.argv) > 2:
        name_paths.append(Path(sys.argv[2]))
    name_hits = check_personal_names(name_paths, banned, report)
    if name_hits:
        report['ok'] = False

    # Classify round2-only failures
    r2_missing_prefix = ('primary-bg-hover','primary-bg-pressed','success-fg','success-bg','pending-bar','border-form')
    r2_missing = [k for k in report['missing_keys'] if any(k.endswith('.'+p) or k.split('.',1)[-1]==p for p in r2_missing_prefix)]
    other_missing = [k for k in report['missing_keys'] if k not in r2_missing]
    report['round2'] = {
        'missing_keys_round2': r2_missing,
        'missing_keys_other': other_missing,
        'contrast_issues_count': len(contrast_issues),
        'personal_name_hits': len(name_hits),
        'value_mismatches_count': len(report['value_mismatches']),
        'only_round2_failures': (
            report['ok'] is False
            and len(other_missing) == 0
            and len(report['value_mismatches']) == 0
            and len(report['missing_themes']) == 0
            and len(report['unreadable_values']) == 0
            and len(bad_root) == 0
            and len(hits) == 0
            and not report['structure']['has_2B37FF']
            # contrast + missing round2 keys + optional name hits in design-system outside export are round2
            and True
        ),
    }
    # tighten only_round2: allow contrast_issues and r2_missing and name hits in design-system.md paths
    non_r2 = (
        other_missing
        or report['value_mismatches']
        or report['missing_themes']
        or report['unreadable_values']
        or bad_root
        or hits
        or report['structure']['has_2B37FF']
    )
    report['round2']['only_round2_failures'] = (not report['ok']) and (not non_r2)

    report['summary'] = {
        'mismatch_count': len(report['value_mismatches']),
        'missing_theme_count': len(report['missing_themes']),
        'missing_key_count': len(report['missing_keys']),
        'unreadable_count': len(report['unreadable_values']),
        'forbidden_root_count': len(bad_root),
        'forbidden_substring_count': len(hits),
        'contrast_issue_count': len(contrast_issues),
        'personal_name_hit_count': len(name_hits),
        'pass': report['ok'],
        'only_round2_failures': report['round2']['only_round2_failures'],
    }

    print(json.dumps(report, ensure_ascii=False, indent=2))
    sys.exit(0 if report['ok'] else 1)

if __name__ == '__main__':
    main()
