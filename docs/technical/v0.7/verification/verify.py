"""Read-only inputs; writes verification JSON only. No runtime or external I/O."""
from pathlib import Path
import hashlib,json,re,unicodedata,datetime
from urllib.parse import unquote,urlsplit
R=Path(__file__).resolve().parent.parent
V=R/'verification'
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def dump(n,v):(V/n).write_text(json.dumps(v,ensure_ascii=False,indent=2)+'\n')
def strip_fences(t):return re.sub(r'(?ms)^\s*(```|~~~).*?^\s*\1[^\n]*\n?','',t)
def anchors(p):
 t=strip_fences(p.read_text());ids=re.findall(r'<a\s+id=["\x27]([^"\x27]+)',t)
 seen={}
 for raw in re.findall(r'^#{1,6}\s+(.+?)\s*#*$',t,re.M):
  s=re.sub(r'\[([^\]]+)\]\([^)]*\)',r'\1',raw)
  s=s.replace('`','').replace('*','').replace('~','').lower()
  s=''.join(c for c in s if c in '-_ ' or not unicodedata.category(c).startswith(('P','S')))
  s=re.sub(r'\s','-',s);n=seen.get(s,0);seen[s]=n+1
  ids.append(s if n==0 else f'{s}-{n}')
 return ids
manifest=json.loads((V/'source-preservation.json').read_text())['files']
changed=[];missing=[];copyerr=[];copies=0
# 原本ツリー（/workspace/pelway-technical-docs）は原本作成環境にだけ存在する。
# 無い環境（CI 等）では原本自体の存在・hash照合を not-run とし、リポジトリ内の
# コピー（sources/）が manifest の hash と一致するかの照合だけを行う。
origavail=Path('/workspace/pelway-technical-docs').is_dir()
for name,h in manifest.items():
 p=Path(name)
 if origavail:
  if not p.exists():missing.append(name)
  elif digest(p)!=h:changed.append(name)
 if '/delivery-v0.6/' in name:q=R/'sources/v0.6'/name.split('/delivery-v0.6/',1)[1]
 elif '/addenda/' in name:q=R/'sources/addenda'/name.split('/addenda/',1)[1]
 else:continue
 if not q.exists() or digest(q)!=h:copyerr.append(str(q))
 else:copies+=1
zipold=Path('/workspace/pelway-technical-docs-v0.6.zip')
expected='d00c771272000d529197b7dca16d1d16fe56b07e82c0d0e4b315da8e4e7c3927'
# 凍結 ZIP は原本作成環境（/workspace）にだけ存在する。無い環境（CI 等）では
# sha256 を null・check を not-run として記録し、失敗には数えない。ある環境では
# 従来どおり照合し、不一致は失敗に数える。
zipsha=digest(zipold) if zipold.is_file() else None
md=sorted((R/'body').glob('*.md'))
md += [R/n for n in ['README.md','source-correspondence.md','status.md','final-confirmation.md','reviews/disposition-v0.7.md','verification/report-v0.7.md'] if (R/n).exists()]
links=[];linkerr=[];cache={}
for p in md:
 for raw in re.findall(r'\[[^\]\n]+\]\(([^\s)]+)(?:\s+"[^"]*")?\)',strip_fences(p.read_text())):
  if urlsplit(raw).scheme or raw.startswith('//'):continue
  links.append({'from':str(p.relative_to(R)),'to':raw})
  path,_,frag=unquote(raw).partition('#');q=(p.parent/path).resolve() if path else p
  if not q.exists():linkerr.append({'from':str(p),'to':raw,'error':'missing target'});continue
  if frag and q.suffix=='.md':
   if q not in cache:cache[q]=set(anchors(q))
   if frag not in cache[q]:linkerr.append({'from':str(p),'to':raw,'error':'missing fragment'})
rows=json.loads((V/'section-correspondence.json').read_text())
original=json.loads((V/'source-sections.json').read_text())
om=[a for a in original if not any(b['source']==a['source'] and b['source_heading']==a['heading'] for b in rows)]
cur=json.loads((V/'current-sections.json').read_text());cm=[]
for filename,sects in cur.items():
 for sec in sects:
  target='body/'+filename+'#'+sec['anchor']
  if not any(a['target']==target for a in rows):cm.append(target)
  if sec['anchor'] not in anchors(R/'body'/filename):cm.append('target missing: '+target)
dup=[]
for p in (R/'body').glob('*.md'):
 ids=re.findall(r'<a\s+id=["\x27]([^"\x27]+)',p.read_text())
 for a in set(ids):
  if ids.count(a)>1:dup.append(str(p)+':'+a)
ui=[]
a=(R/'body/03-ai-bot-runtime.md').read_text();b=(R/'body/06-i18n.md').read_text()
wanted='| notice-ai-input-excluded | 一部の発言を除外して処理しました（利用の同意を確認できない範囲） | Some messages were excluded (consent for this use could not be confirmed) |'
if wanted not in a or wanted not in b:ui.append('excluded notice mismatch')
if '| reason-cancel-after-start | 停止要求済み · 結果確認中（撤回されない可能性があります） | Stop requested · checking result (may not be reversed) |' not in a:ui.append('after-start mismatch')
for p in (R/'body').glob('*.md'):
 for bad in ['reason-cancel-requested','取り消し依頼済み','以後送信なし']:
  if bad in p.read_text():ui.append(str(p.relative_to(R))+': '+bad)
reg=json.loads((V/'review-register.json').read_text())
result={'checked_at_jst':datetime.datetime.now().astimezone().isoformat(),'original_files':len(manifest),'originals_check':'not-run: originals absent' if not origavail else 'run','original_changed':changed,'original_missing':missing,'archive_copies_verified':copies,'archive_copy_errors':copyerr,'frozen_zip_sha256':zipsha,'frozen_zip_expected':expected,'frozen_zip_check':'not-run: archive absent' if zipsha is None else ('pass' if zipsha==expected else 'fail'),'current_markdown_files':len(md),'relative_links_checked':len(links),'relative_link_errors':linkerr,'original_sections':len(original),'original_sections_without_mapping':om,'current_sections':sum(map(len,cur.values())),'current_sections_without_mapping':cm,'correspondence_rows':len(rows),'duplicate_body_anchor_ids':dup,'latest_ui_regression_errors':ui,'runtime_tests':'not run','final_role_confirmation':reg['reviews'],'zip':'separate packaging check: /workspace/pelway-technical-docs-v0.7.zip.verify.json (outside ZIP to avoid self-reference)'}
failed=bool(changed or missing or copyerr or linkerr or om or cm or dup or ui or (zipsha is not None and zipsha!=expected))
result['static_pass']=not failed
dump('relative-links.json',links);dump('static-checks.json',result)
print(json.dumps({k:v for k,v in result.items() if k not in ['final_role_confirmation']},ensure_ascii=False,indent=2))
raise SystemExit(1 if failed else 0)
