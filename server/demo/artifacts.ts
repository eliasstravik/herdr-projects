// The demo agents' artifacts: their pages by HTML version (made with shepherdr artifact in real use) and their data.

/** the week board draws itself from its data, and again whenever new data comes in */
const WEEK = `<style>
body{display:flex;flex-direction:column;gap:10px}
.head{display:flex;align-items:baseline;justify-content:space-between;gap:8px}
.num{font-size:22px;line-height:28px;font-weight:600;font-variant-numeric:tabular-nums}
.small{font-size:12px;color:var(--fg2)}
.bars{display:flex;align-items:flex-end;gap:6px;height:84px}
.full .bars{height:280px}
.bars i{flex:1;border-radius:3px 3px 0 0;background:var(--fg3);opacity:.45}
.bars i.hi{background:var(--chart-1);opacity:1}
.days{display:flex;gap:6px}
.days span{flex:1;text-align:center;font-size:11px;color:var(--fg3)}
.phone .small{font-size:14px}
</style>
<div class="head"><div><div class="num" id="n"></div><div class="small">tasks merged this week</div></div><div class="small" id="at"></div></div>
<div class="bars" id="b"></div>
<div class="days" id="d"></div>
<script>
function draw(d){
  if (!d) return;
  var max = Math.max.apply(null, d.merged);
  document.getElementById('n').textContent = d.merged.reduce(function(a, b){ return a + b }, 0);
  document.getElementById('at').textContent = 'as of ' + d.asOf;
  document.getElementById('b').innerHTML = d.merged.map(function(v, i){ return '<i style="height:' + Math.max(6, Math.round(v / max * 100)) + '%"' + (v === max ? ' class="hi"' : '') + '></i>' }).join('');
  document.getElementById('d').innerHTML = d.days.map(function(x){ return '<span>' + x + '</span>' }).join('');
}
draw(shepherdr.data);
document.addEventListener('shepherdr:data', function(e){ draw(e.detail) });
</script>`

const DISK = `<style>
.rows{display:flex;flex-direction:column;gap:8px}
.row{display:grid;grid-template-columns:7rem 1fr 3.5rem;align-items:center;gap:8px;font-size:13px}
.row b{font-weight:500;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.row s{display:block;height:8px;border-radius:4px;background:var(--bubble);text-decoration:none;overflow:hidden}
.row s i{display:block;height:100%;background:var(--chart-2);border-radius:4px}
.row span{text-align:right;color:var(--fg2);font-variant-numeric:tabular-nums}
.phone .row{font-size:15px}
</style><div class="rows">${[
  ['worktrees', 41, '18 GB'],
  ['node_modules', 27, '12 GB'],
  ['herdr logs', 12, '5 GB'],
  ['docker', 9, '4 GB'],
]
  .map(([k, p, v]) => `<div class="row"><b>${k}</b><s><i style="width:${p}%"></i></s><span>${v}</span></div>`)
  .join('')}</div>`

const SHOT_CSS = `<style>
.shot{height:96px;border-radius:8px;box-shadow:inset 0 0 0 1px var(--line);background:var(--well);position:relative;overflow:hidden}
.full .shot{height:260px}
.shot::before{content:"";position:absolute;left:12px;right:40%;top:12px;height:8px;border-radius:4px;background:var(--line2);box-shadow:0 18px 0 var(--line),0 32px 0 var(--line),0 46px 0 var(--line)}
.shot::after{content:"";position:absolute;right:12px;top:12px;width:28%;bottom:12px;border-radius:6px;background:var(--line)}
</style>`

const SHOT = `${SHOT_CSS}<div class="shot"></div>`

const board = (count: number) => `${SHOT_CSS}<style>
body{display:flex;flex-direction:column;gap:8px}
.row{display:flex;gap:8px;align-items:center;flex-wrap:wrap;padding:8px;border-radius:8px;background:var(--well)}
.small{font-size:12px;color:var(--fg2)}
.count{font-size:13px;font-variant-numeric:tabular-nums}
.count b{font-weight:600}
.seg{display:inline-flex;gap:2px;padding:2px;border-radius:8px;background:rgb(127 127 127/.08)}
.seg label{display:inline-flex;align-items:center;height:28px;padding:0 10px;border-radius:6px;font-size:13px;color:var(--fg2);cursor:pointer}
.seg input{display:none}
.seg input:checked+span{color:var(--fg)}
.seg label:has(input:checked){background:var(--bg);box-shadow:0 0 0 1px var(--line2)}
button{font:inherit;font-weight:500;height:32px;padding:0 12px;border:0;border-radius:9px;background:var(--bg);color:var(--fg);box-shadow:inset 0 0 0 1px var(--line2),0 1px 2px rgb(0 0 0/.05);cursor:pointer}
.sent{display:flex;align-items:center;gap:6px;color:var(--ready);font-size:13px}
.phone .small,.phone .count{font-size:14px}
.phone .seg label{height:36px;font-size:14px;padding:0 14px}
.sent svg{width:16px;height:16px;fill:currentColor}
</style>
<div class="count">Pinned <b>${count}</b> · 2 projects pinned</div>
<div class="shot"></div>
<form class="row" id="f"><span class="small">Pinned colour</span>
<span class="seg"><label><input type="radio" name="Pinned colour" value="Accent" checked><span>Accent</span></label><label><input type="radio" name="Pinned colour" value="Grey"><span>Grey</span></label></span>
<button type="submit">Send</button></form>
<div class="sent" id="s" hidden><svg viewBox="0 0 16 16"><path fill-rule="evenodd" d="M12.416 3.376a.75.75 0 0 1 .208 1.04l-5 7.5a.75.75 0 0 1-1.154.114l-3-3a.75.75 0 0 1 1.06-1.06l2.353 2.353 4.493-6.74a.75.75 0 0 1 1.04-.207Z" clip-rule="evenodd"/></svg><span id="t"></span></div>
<script>
function done(text){ var v = String(text).split(': ').pop(); document.getElementById('f').hidden = true; document.getElementById('s').hidden = false; document.getElementById('t').textContent = 'Sent: Pinned colour =\\u00a0' + v }
if (shepherdr.sent) done(shepherdr.sent);
document.addEventListener('shepherdr:sent', function(e){ done(e.detail.text) });
</script>`

/** a deck: one slide inline, every slide and arrows full screen */
const DECK = `<style>
.slide{display:flex;flex-direction:column;justify-content:center;gap:8px;min-height:120px;padding:20px;border-radius:10px;background:var(--well);box-shadow:inset 0 0 0 1px var(--line)}
.full .slide{min-height:min(70vh,520px);padding:40px}
.slide h1{margin:0;font-size:20px;line-height:26px}
.full .slide h1{font-size:32px;line-height:40px}
.slide p{margin:0;color:var(--fg2)}
.nav{display:none;justify-content:space-between;align-items:center;margin-top:12px;color:var(--fg2);font-variant-numeric:tabular-nums}
.full .nav{display:flex}
.nav button{font:inherit;height:36px;padding:0 14px;border:0;border-radius:9px;background:var(--bubble);color:var(--fg);cursor:pointer}
</style>
<div class="slide" id="s"></div>
<div class="nav"><button id="p">Back</button><span id="c"></span><button id="n">Next</button></div>
<script>
var slides = [['Welcome to gtm', 'One repo for your go-to-market.'], ['Context', 'ICPs, personas and voice live next to the code.'], ['Workflows', 'Outbound and enrichment run on a schedule.'], ['Console', 'See every run, and what it cost.']];
var i = 0;
function show(){ var s = slides[i]; document.getElementById('s').innerHTML = '<h1>' + s[0] + '</h1><p>' + s[1] + '</p>'; document.getElementById('c').textContent = (i + 1) + ' / ' + slides.length }
document.getElementById('p').onclick = function(){ i = Math.max(0, i - 1); show() };
document.getElementById('n').onclick = function(){ i = Math.min(slides.length - 1, i + 1); show() };
addEventListener('keydown', function(e){ if (e.key === 'ArrowRight') document.getElementById('n').click(); if (e.key === 'ArrowLeft') document.getElementById('p').click() });
show();
</script>`

/** each artifact's pages, by the version that brought its HTML */
export const ARTIFACT_HTML: Record<string, Record<number, string>> = {
  a1: { 1: WEEK },
  a5: { 1: board(3), 2: board(2) },
  a6: { 1: DISK },
  a7: { 1: SHOT },
  a8: { 1: DECK },
}

const week = (merged: number[], asOf: string) => ({ days: ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'], merged, asOf })

/** each artifact's data, by version */
export const ARTIFACT_DATA: Record<string, Record<number, unknown>> = {
  a1: { 1: week([3, 5, 4, 0, 0, 0, 0], 'Wed 8:00'), 2: week([3, 5, 4, 7, 0, 0, 0], 'Thu 8:00'), 3: week([3, 5, 4, 7, 6, 2, 0], 'Sat 8:00') },
}

/** a made-up picture of each, as the check would take it */
export function thumb(id: string, dark: boolean) {
  const bg = dark ? '#0b0b0d' : '#ffffff'
  const fg = dark ? '#2c2c31' : '#e4e4e7'
  const hi = dark ? '#fb923c' : '#c2410c'
  const bars = [38, 52, 44, 70, 61, 88, 47]
  const body =
    id === 'a1'
      ? bars.map((h, j) => `<rect x="${24 + j * 48}" y="${300 - h * 2}" width="36" height="${h * 2}" rx="4" fill="${j === 5 ? hi : fg}"/>`).join('')
      : id === 'a6'
        ? [41, 27, 12, 9].map((w, j) => `<rect x="24" y="${40 + j * 40}" width="${w * 8}" height="14" rx="7" fill="${j ? fg : hi}"/>`).join('')
        : id === 'a8'
          ? `<rect x="24" y="40" width="342" height="200" rx="14" fill="${fg}"/><rect x="48" y="110" width="200" height="22" rx="6" fill="${hi}"/>`
          : `<rect x="24" y="24" width="342" height="44" rx="10" fill="${fg}"/><rect x="24" y="84" width="342" height="200" rx="12" fill="${fg}"/><rect x="40" y="36" width="90" height="20" rx="6" fill="${hi}"/>`
  return new Blob([`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 390 320"><rect width="390" height="320" fill="${bg}"/>${body}</svg>`], { type: 'image/svg+xml' })
}
