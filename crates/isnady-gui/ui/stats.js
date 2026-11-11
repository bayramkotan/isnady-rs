"use strict";
// Statistics (gui/statistics_page.py, gui/stats_charts.py, gui/stats_export.py — ST7, deep): the imported corpus
// measured — the narrators, the chains, the grades against the chains, the hadith model, the laboratory, the books,
// and the graders of each book; every measure in plain words. One section at a time, chosen on the left. Each
// section is a few FINDINGS: a sentence that says what the numbers show, the chart that shows it, the figures behind
// it on hover, how to read it (folded) and its figures as CSV. Ranks and grades are ordered categories: counted,
// cross-tabulated and rank-correlated, never averaged. The Sunni road and the Shia road are never measured together.

const ST = {
  built: false, scope: null, groups: [], counts: { sunni: 0, shia: 0 }, graderBooks: [], result: null, gresult: null,
  names: { short: {}, full: {}, books: {}, people: {} }, terms: [], modelIndex: 0, labIndex: 0, section: 0,
  seq: 0, gseq: 0, busy: false, gbusy: false, again: false, gagain: false, pending: null, tables: new Map(), cardN: 0,
  loaded: false, gbook: null, poll: null,
};

const ST_SECTIONS = [
  ["overview", "Overview", "the corpus at a glance"],
  ["narrators", "Narrators", "who carries the hadith"],
  ["chains", "Chains", "length, weakest link, time"],
  ["grades", "Grades and chains", "do the chains explain the grades"],
  ["model", "The hadith model", "what a scholar's grade follows"],
  ["lab", "Correlations and tests", "what goes with what, and is it real"],
  ["books", "Books", "the books compared"],
  ["graders", "Graders", "the scholars who graded"],
  ["terms", "Terms explained", "every measure in plain words"],
];
// ranks and grades are ordered AND two-sided: a diverging scale, blue the accepted side, red the rejected
const ST_GROUP_COLORS = {
  false: ["#1E4A8A", "#3A72B8", "#8DB4DE", "#C9C6C0", "#E4997A", "#B23A33"],
  true: ["#6E9FE0", "#4A7FC4", "#2E5A92", "#55534F", "#B4543F", "#E0705F"],
};
const ST_GRADE_COLORS = {
  false: ["#1E4A8A", "#8DB4DE", "#E4997A", "#C8564A", "#8E2A25"],
  true: ["#6E9FE0", "#2E5A92", "#B4543F", "#D86A55", "#F08A75"],
};
const ST_IMAMI_COLORS = {
  false: ["#1E4A8A", "#8DB4DE", "#C9C6C0", "#C8564A"],
  true: ["#6E9FE0", "#2E5A92", "#55534F", "#D86A55"],
};
const ST_PRAISED_NOTE = "In al-Najashi's own words “praised” takes in some of the greatest narrators — Zurara, Ibn Abi 'Umayr, Ahmad b. Muhammad b. 'Isa — whom he calls jalil or wajh without the word thiqa, and whom al-Tusi and the later scholars count as thiqa (some among the ashab al-ijma'): a chain whose weakest is “praised” is therefore often graded sahih.";
const ST_ALPHA = 0.05;

// ------------------------------------------------------------------ small things

function stPct(x) { return (100 * x).toFixed(1) + "%"; }
function stTh(n) { return Number(n || 0).toLocaleString("en-US"); }
function stSigned(x, d) { const s = x.toFixed(d); return x >= 0 && !s.startsWith("-") ? "+" + s : s; }
function stFmt(n) { return Math.abs(n) >= 1 || n === 0 ? Math.round(n).toLocaleString("en-US") : n.toFixed(2); }
// format(x, ".Ng") as Python writes it
function stG(x, p) {
  if (x === 0) return "0";
  if (!isFinite(x)) return String(x);
  const exp = Math.floor(Math.log10(Math.abs(Number(x.toPrecision(p)))));
  if (exp < -4 || exp >= p) {
    const [m, e] = x.toExponential(p - 1).split("e");
    const mant = m.includes(".") ? m.replace(/0+$/, "").replace(/\.$/, "") : m;
    const n = Number(e);
    return `${mant}e${n < 0 ? "-" : "+"}${String(Math.abs(n)).padStart(2, "0")}`;
  }
  const s = x.toFixed(Math.max(0, p - 1 - exp));
  return s.includes(".") ? s.replace(/0+$/, "").replace(/\.$/, "") : s;
}
function stQ(q) { return q == null ? "—" : (q < 0.001 ? "q < 0.001" : `q ${q.toFixed(3)}`); }
function stShort(g) { return ST.names.short[g] || g; }
function stFull(g) { return ST.names.full[g] || g; }
function stBk(name) { return ST.names.books[name] || name; }
function stName(id, name) { return ST.names.people[String(id)] || name || ""; }
function stOutcome(name) { return name.startsWith("Grade: ") ? stShort(name.slice(7)) : t(name); }
function stFactor(name) {
  return name.startsWith("Weakest: ") ? t("Weakest: {level}", { level: t(name.slice(9)) }) : t(name);
}
function stGrade(label) { return label.split(" or ").map((p) => t(p)).join(" / "); }
function stSafe(name) {
  return (name.toLowerCase().replace(/[^\p{L}\p{N}]/gu, "-").replace(/^-+|-+$/g, "").slice(0, 60)) || "isnady";
}
function stShia() { return ((ST.current || {}).corpus || {}).tradition === "shia"; }
function stColors() { const full = ST_GROUP_COLORS[!!DARK]; return stShia() ? full.slice(1, 5) : full; }
function stWords() {
  if (stShia()) {
    return { end: t("a Ma'sum"), End: t("a Ma'sum (the Prophet ﷺ or an Imam)"), rijal: t("al-Najashi's Rijal"),
      critic: t("al-Najashi"), reach: t("Reach a Ma'sum"), rank: "al-Najashi's category", rank_shown: t("al-Najashi's category"),
      grades: t("the four Imami categories (sahih, hasan, muwaththaq, da'if)"), scale: t("sahih → da'if"),
      otherwise: t("a narrator the grader judged otherwise than al-Najashi (al-Majlisi also draws on al-Tusi, al-Kashshi and the later critics), or a name matched to the wrong person") };
  }
  return { end: t("the Prophet ﷺ"), End: t("the Prophet ﷺ"), rijal: t("Ibn Hajar's Taqrib"), critic: t("Ibn Hajar"),
    reach: t("Reach the Prophet"), rank: "Ibn Hajar's rank", rank_shown: t("Ibn Hajar's rank"), grades: t("five ordered groups"),
    scale: t("sahih → fabricated"), otherwise: t("a narrator the grader judged otherwise than Ibn Hajar") };
}
function stGroupCats(r) { const c = stColors(); return r.groups.map(([name], i) => [t(name), c[i]]); }
function stGradeCats(r) {
  const c = (r.tradition === "shia" ? ST_IMAMI_COLORS : ST_GRADE_COLORS)[!!DARK];
  return r.grade_labels.map((g, i) => [t(g), c[i]]);
}

// colours of the theme, read from the page
function stToken(name) { return getComputedStyle(document.documentElement).getPropertyValue("--" + name).trim(); }
function stRgb(hex) {
  const h = hex.replace("#", "");
  const v = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  return [0, 2, 4].map((i) => parseInt(v.slice(i, i + 2), 16));
}
function stMix(a, b, x) {
  x = Math.max(0, Math.min(1, x));
  const [p, q] = [stRgb(a), stRgb(b)];
  return "#" + p.map((v, i) => Math.trunc(v + (q[i] - v) * x).toString(16).padStart(2, "0")).join("");
}
function stInk(fill) {
  const lum = (c) => {
    const ch = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; };
    const [r, g, b] = stRgb(c);
    return 0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b);
  };
  const l = lum(fill);
  return 1.05 / (l + 0.05) > (l + 0.05) / (lum("#16181D") + 0.05) ? "#FFFFFF" : "#16181D";
}

// ------------------------------------------------------------------ the charts (stats_charts.py)

function stTiles(items) {
  return `<div class="st-tiles">${items.map(([label, value, note]) => `<div class="st-tile"><div class="st-tile-l">${esc(label)}</div>
    <div class="st-tile-v">${esc(value)}</div><div class="st-tile-n">${esc(note)}</div></div>`).join("")}</div>`;
}

function stStacked(rows, cats, labelW) {
  labelW = labelW || 190;
  let html = `<div class="st-stacked">`;
  for (const [label, counts] of rows) {
    const total = counts.reduce((a, b) => a + b, 0);
    const lastOn = counts.reduce((acc, n, i) => (n ? i : acc), -1);
    html += `<div class="st-srow"><div class="st-slabel" style="width:${labelW}px">${esc(label)}</div><div class="st-sbar">`;
    counts.forEach((n, c) => {
      if (!n) return;
      const share = 100 * n / (total || 1);
      html += `<span class="st-seg${c === lastOn ? " last" : ""}" style="width:${share}%;background:${cats[c][1]}" data-tip="${esc(`${label}\n${cats[c][0]}: ${stTh(n)} (${share.toFixed(1)}%)`)}"></span>`;
    });
    html += `</div><div class="st-stotal">${stFmt(total)}</div></div>`;
  }
  html += `<div class="st-legend" style="margin-inline-start:${labelW}px">${cats.map(([n, c]) => `<span><i style="background:${c}"></i>${esc(n)}</span>`).join("")}</div></div>`;
  return html;
}

function stColumns(bins, unit, marked, labelEvery) {
  marked = marked || new Set();
  labelEvery = labelEvery || 1;
  const peak = Math.max(0, ...bins.map((b) => b[1])) || 1;
  let grid = "";
  for (let k = 0; k <= 4; k++) {
    grid += `<div class="st-cgrid" style="bottom:${25 * k}%"><span>${stFmt(peak * k / 4)}</span></div>`;
  }
  const cols = bins.map(([label, v], i) => `<div class="st-cslot" data-tip="${esc(`${label}: ${stFmt(v)}${unit}`)}">
      <div class="st-cbar${marked.has(i) ? " marked" : ""}" style="height:${100 * v / peak}%"></div>
      <div class="st-clabel">${i % labelEvery === 0 ? esc(label) : ""}</div></div>`).join("");
  return `<div class="st-columns"><div class="st-cplot">${grid}<div class="st-cbars">${cols}</div></div></div>`;
}

function stLorenz(curve) {
  const pts = curve.map(([x, y]) => `${(1000 * x).toFixed(2)},${(200 * (1 - y)).toFixed(2)}`).join(" ");
  let grid = "";
  for (let k = 0; k <= 4; k++) {
    grid += `<div class="st-lgy" style="bottom:${25 * k}%"><span>${25 * k}%</span></div><div class="st-lgx" style="left:${25 * k}%"><span>${25 * k}%</span></div>`;
  }
  const tips = curve.map(([x, y]) => t("The {least:.0f}% of narrators who carry least carry {share:.1f}% of the links;\nthe other {rest:.0f}% carry {rest_share:.1f}%.",
    { least: 100 * x, share: 100 * y, rest: 100 - 100 * x, rest_share: 100 - 100 * y }));
  const hot = curve.map(([x], i) => `<span class="st-lhot" style="left:${100 * x}%" data-tip="${esc(tips[i])}"></span>`).join("");
  return `<div class="st-lorenz"><div class="st-lplot">${grid}
    <svg viewBox="0 0 1000 200" preserveAspectRatio="none"><line x1="0" y1="200" x2="1000" y2="0" class="diag"/>
    <polyline points="${pts}" class="curve"/></svg>${hot}</div></div>`;
}

function stPeople(rows, value, unit) {
  value = value || ((r) => r.links);
  unit = unit || "";
  const shia = stShia();
  const colors = stColors();
  const peak = Math.max(0, ...rows.map(value)) || 1;
  const ranks = (WIN.people || {}).ranks || {};
  const shiaRanks = (WIN.people || {}).shia_ranks || {};
  let html = `<div class="st-people">`;
  for (const r of rows) {
    const rank = r.rank;
    const facts = [];
    if (shia) facts.push(shiaRanks[rank] ? t("al-Najashi: {meaning}", { meaning: t(shiaRanks[rank][1]) }) : t("no judgment in al-Najashi"));
    else facts.push(ranks[rank] ? t("rank {rank}: {meaning}", { rank, meaning: t(ranks[rank][1]) }) : t("no rank in the Taqrib"));
    if (r.tabaqa) facts.push(t("tabaqa {tabaqa}", { tabaqa: r.tabaqa }));
    if (r.death) facts.push(t("d. {year} AH", { year: r.death }));
    facts.push(t("{links:,} times in the chains, in {hadith:,} hadith", { links: r.links, hadith: r.hadith }));
    const group = shia ? (rank >= 1 && rank <= 4 ? rank - 1 : null) : stRankGroup(rank);
    const dot = group == null ? "var(--muted)" : colors[group];
    const name = stName(r.id, r.name);
    const tip = `${name}\n${r.name || ""}\n${facts.join("\n")}\n\n${t("Click to open him on the Narrators page")}`;
    const v = value(r);
    html += `<div class="st-prow" data-narr="${r.id}" data-tip="${esc(tip)}"><i style="background:${dot}"></i>
      <span class="st-pname">${esc(name)}</span><span class="st-ptrack"><span style="width:${Math.max(0.5, 100 * v / peak)}%"></span></span>
      <span class="st-pval">${esc(stFmt(v) + unit)}</span></div>`;
  }
  return html + "</div>";
}

function stRankGroup(rank) {
  const groups = [[1], [2, 3], [4, 5], [6], [7, 8, 9], [10, 11, 12]];
  const i = groups.findIndex((g) => g.includes(rank));
  return i < 0 ? null : i;
}

// rows = [[label, value, [low, high]]]; left/right: what each side of zero means
function stInterval(rows, left, right, fmt, labelW) {
  fmt = fmt || ((v) => stSigned(v, 3));
  labelW = labelW || 290;
  const span = 1.15 * Math.max(0.05, ...rows.map((r) => Math.abs(r[1])), ...rows.flatMap((r) => r[2].map(Math.abs)));
  const X = (v) => 100 * (v + span) / (2 * span);
  let html = `<div class="st-interval">`;
  for (const [label, v, [lo, hi]] of rows) {
    const c = v < 0 ? "var(--gold)" : "var(--lapis)";
    html += `<div class="st-irow"><div class="st-ilabel" style="width:${labelW}px">${esc(label)}</div>
      <div class="st-itrack"><span class="st-izero" style="left:${X(0)}%"></span>
      <span class="st-iline" style="left:${X(Math.min(lo, hi))}%;width:${Math.abs(X(hi) - X(lo))}%;background:${c}"></span>
      <span class="st-idot" style="left:${X(v)}%;background:${c}"></span></div>
      <div class="st-ifig">${esc(fmt(v, lo, hi))}</div></div>`;
  }
  html += `<div class="st-irow st-iaxis"><div style="width:${labelW}px"></div><div class="st-itrack plain">
    <span class="st-ileft" style="right:${100 - X(0)}%">◀ ${esc(left)}</span><span class="st-iright" style="left:${X(0)}%">${esc(right)} ▶</span></div>
    <div class="st-ifig"></div></div></div>`;
  return html;
}

// cells[r][c] = [value or null, [lines]]; diverging: below zero gold, above lapis
function stHeat(rows, cols, cells, opts) {
  const o = Object.assign({ low: 0, high: 1, diverging: false, cw: 132, ch: 58, labelW: 150, headH: 44, tips: null, corner: "" }, opts || {});
  const surface = stToken("surface"), lapis = stToken("lapis"), gold = stToken("gold");
  const colour = (v) => {
    if (o.diverging) {
      const span = Math.max(Math.abs(o.low), Math.abs(o.high)) || 1;
      const x = v / span;
      return x >= 0 ? stMix(surface, lapis, x) : stMix(surface, gold, -x);
    }
    return stMix(surface, lapis, (v - o.low) / ((o.high - o.low) || 1));
  };
  const tpl = `${o.labelW}px repeat(${cols.length}, ${o.cw}px)`;
  let html = `<div class="st-heat" style="grid-template-columns:${tpl};grid-template-rows:${o.headH}px;grid-auto-rows:${o.ch}px">`;
  html += `<div class="st-hcorner" style="height:${o.headH}px">${esc(o.corner).replace(/\n/g, "<br>")}</div>`;
  for (const c of cols) html += `<div class="st-hcol" style="height:${o.headH}px">${esc(c)}</div>`;
  rows.forEach((name, r) => {
    html += `<div class="st-hrow">${esc(name)}</div>`;
    cols.forEach((_c, c) => {
      const [value, lines] = cells[r][c];
      const tip = o.tips && o.tips[r][c] ? ` data-tip="${esc(o.tips[r][c])}"` : "";
      if (value == null) {
        html += `<div class="st-hcell empty"${tip}>${lines && lines.length ? `<span class="small">${esc(lines[0])}</span>` : ""}</div>`;
        return;
      }
      const fill = colour(value);
      html += `<div class="st-hcell" style="background:${fill};color:${stInk(fill)}"${tip}><b>${esc(lines[0] || "")}</b>${lines.length > 1 ? `<span class="small">${esc(lines[1])}</span>` : ""}</div>`;
    });
  });
  return html + "</div>";
}

function stCalibration(points, label) {
  const biggest = Math.max(1, ...points.map((p) => p[2]));
  let grid = "";
  for (let k = 0; k <= 4; k++) {
    grid += `<line x1="0" x2="220" y1="${220 - 55 * k}" y2="${220 - 55 * k}"/><line y1="0" y2="220" x1="${55 * k}" x2="${55 * k}"/>`;
  }
  const labels = [0, 1, 2, 3, 4].map((k) => `<span class="st-cay" style="top:${220 - 55 * k}px">${25 * k}%</span><span class="st-cax" style="left:${55 * k}px">${25 * k}%</span>`).join("");
  const dots = points.map(([x, y, n]) => `<circle cx="${220 * x}" cy="${220 * (1 - y)}" r="${4 + 8 * Math.sqrt(n / biggest)}"><title>${esc(t("{n:,} hadith: the model gave {given:.0f}% on average, {got:.0f}% got it", { n, given: 100 * x, got: 100 * y }))}</title></circle>`).join("");
  return `<div class="st-calib"><div class="st-cplotc">${labels}<svg width="220" height="220" viewBox="0 0 220 220"><g class="grid">${grid}</g>
    <line x1="0" y1="220" x2="220" y2="0" class="diag"/><g class="dots">${dots}</g></svg></div><div class="caption st-ctext">${esc(label)}</div></div>`;
}

function stTable(heads, rows, opts) {
  const o = Object.assign({ stretch: true, first: false, max: 0 }, opts || {});
  let html = `<div class="st-twrap"${o.max ? ` style="max-height:${o.max}px"` : ""}><table class="st-table${o.first ? " first-fit" : ""}"><thead><tr>`;
  html += heads.map((h) => (typeof h === "string" ? `<th>${esc(h)}</th>` : `<th data-tip="${esc(h[1])}">${esc(h[0])}</th>`)).join("") + "</tr></thead><tbody>";
  for (const r of rows) {
    const attrs = r.attrs || "";
    html += `<tr${attrs}>` + r.cells.map((c) => {
      const cell = typeof c === "object" && c !== null ? c : { text: c };
      return `<td${cell.muted ? ' class="muted"' : ""}${cell.tip ? ` data-tip="${esc(cell.tip)}"` : ""}${cell.narr ? ` data-narr="${cell.narr}"` : ""}>${esc(cell.text)}</td>`;
    }).join("") + "</tr>";
  }
  return html + "</tbody></table></div>";
}

// ------------------------------------------------------------------ the page

function statsBuild() {
  if (ST.built) return;
  ST.built = true;
  const page = $("page-statistics");
  page.className = "page st-page";
  page.innerHTML = `<div class="st-head"><div class="st-titles"><div class="st-title">${esc(t("Statistics"))}</div>
      <div class="lead">${esc(t("The imported books measured: who carries the hadith, how the chains hold, and whether they explain the grades."))}</div></div>
      <span class="np-ftitle">${esc(t("Books"))}</span><select id="st-scope"></select>
      <button class="quiet" id="st-recompute" data-tip="${esc(t("The results are kept in a file and reused while the data is unchanged"))}">${esc(t("Recompute"))}</button>
      <button class="quiet" id="st-export" data-tip="${esc(t("A section or every section as PDF or HTML, every table as Excel or CSV, the results as JSON; each finding has its own ⤓ for its chart and its figures"))}">${esc(t("Export"))}  ▾</button></div>
    <div class="caption st-status" id="st-status"></div>
    <div class="st-body"><div class="st-rail" id="st-rail">${ST_SECTIONS.map(([k, n, s], i) => `<div class="st-ritem" data-i="${i}" data-key="${k}"><div>${esc(t(n))}</div><div class="sub">${esc(t(s))}</div></div>`).join("")}</div>
      <div class="st-stack" id="st-stack">${ST_SECTIONS.map(([k]) => k === "graders"
        ? `<div class="st-section" data-key="graders" hidden><div class="st-gbar"><span class="np-ftitle" id="st-gtitle">${esc(t("Book (two or more graders)"))}</span><select id="st-gbook"></select>
            <button class="quiet" id="st-grecompute" data-tip="${esc(t("The graders' results are kept in a file and reused while the grades are unchanged"))}">${esc(t("Recompute graders"))}</button>
            <span class="caption grow" id="st-gstatus"></span></div><div class="st-box" id="st-box-graders"></div></div>`
        : `<div class="st-section" data-key="${k}" hidden><div class="st-box" id="st-box-${k}"></div></div>`).join("")}</div></div>`;
  $("st-rail").onclick = (e) => {
    const it = e.target.closest(".st-ritem");
    if (it) statsSection(Number(it.dataset.i));
  };
  $("st-scope").onchange = () => {
    ST.scope = $("st-scope").value === "" ? null : $("st-scope").value;
    statsCompute(false);
    statsFollowScope();
  };
  $("st-recompute").onclick = () => statsCompute(true);
  $("st-export").onclick = (e) => statsExportMenu(e.currentTarget);
  $("st-gbook").onchange = () => {
    const key = $("st-gbook").value;
    ST.gbook = key;
    if (key && ST.scope !== key && [...$("st-scope").options].some((o) => o.value === key)) {
      $("st-scope").value = key;
      ST.scope = key;
      statsCompute(false);
      statsFollowScope();
      return;
    }
    statsGraders(false);
  };
  $("st-grecompute").onclick = () => statsGraders(true);
  $("st-stack").addEventListener("click", statsClick);
  $("st-stack").addEventListener("dblclick", (e) => {
    const row = e.target.closest("tr[data-hadith], tr[data-narr2]");
    if (!row) return;
    if (row.dataset.hadith) openInBook(Number(row.dataset.hadith));
    else statsOpenNarrator(Number(e.target.closest("[data-narr]") ? e.target.closest("[data-narr]").dataset.narr : row.dataset.narr2));
  });
  $("st-stack").addEventListener("change", statsChange);
  statsSection(0);
}

function statsSection(i) {
  ST.section = i;
  for (const r of $("st-rail").children) r.classList.toggle("on", Number(r.dataset.i) === i);
  for (const s of $("st-stack").children) s.hidden = s.dataset.key !== ST_SECTIONS[i][0];
}

function statsOpenNarrator(id) {
  return place(["narrator", id], async () => {
    NARR.id = id;
    show("narrators");
    await selectNarrator(id);
  });
}

function statsClick(e) {
  const narr = e.target.closest(".st-prow[data-narr]");
  if (narr) { statsOpenNarrator(Number(narr.dataset.narr)); return; }
  const a = e.target.closest("[data-act]");
  if (!a) return;
  const card = a.closest(".st-card");
  if (a.dataset.act === "how") {
    const more = card.querySelector(".st-how");
    more.hidden = !more.hidden;
    a.textContent = more.hidden ? t("How to read it ▸") : t("How to read it ▾");
  } else if (a.dataset.act === "csv") {
    const tb = ST.tables.get(card.dataset.card);
    statsAskPath(t("Save as CSV"), `isnady-${stSafe(tb.kicker)}.csv`, (path) =>
      invoke("stats_csv", { path, header: tb.header, rows: tb.rows }).then((p) => statsSaid(p)));
  } else if (a.dataset.act === "term") {
    statsTerm(a.dataset.term);
  } else if (a.dataset.act === "arm") {
    statsArm(card, a);
  }
}

function statsChange(e) {
  const s = e.target;
  if (s.dataset.pick === "model") {
    ST.modelIndex = Number(s.value);
    statsModelShow(s.closest(".st-modelwrap"));
  } else if (s.dataset.pick === "lab") {
    ST.labIndex = Number(s.value);
    statsLabShow(s.closest(".st-labwrap"));
  } else if (s.dataset.pick === "matrix") {
    const holder = s.closest(".st-switch");
    const m = holder._m;
    holder.querySelector(".st-switch-body").innerHTML = statsMatrix(m, s.value, holder._names, holder._cell, holder._labelW);
  }
}

// ------------------------------------------------------------------ the data

async function statisticsRefresh() {
  statsBuild();
  const s = await invoke("stats_scope");
  ST.groups = s.groups;
  ST.counts = { sunni: s.sunni, shia: s.shia };
  ST.graderBooks = s.grader_books;
  if (!ST.terms.length) {
    ST.terms = await invoke("stats_terms");
    statsFillTerms();
  }
  const sel = $("st-scope");
  let html = "";
  if (s.shia) html += `<option value="@all">${esc(t("All books ({n})", { n: s.sunni + s.shia }))}</option>`;
  html += `<option value="">${esc(t("All Sunni books ({n})", { n: s.sunni }))}</option>`;
  if (s.shia) html += `<option value="@shia">${esc(t("All Shia books ({shia})", { shia: s.shia }))}</option>`;
  for (const g of s.groups) {
    html += `<optgroup label="${esc(t("{group} collections", { group: t(g.label) }))}">`;
    for (const b of g.books) html += `<option value="${esc(b.key)}">${esc(`${b.name} (${stTh(b.count)})`)}</option>`;
    html += "</optgroup>";
  }
  sel.innerHTML = html;
  const want = ST.scope == null ? "" : ST.scope;
  sel.value = [...sel.options].some((o) => o.value === want) ? want : "";
  ST.scope = sel.value === "" ? null : sel.value;
  const g = $("st-gbook");
  g.innerHTML = ST.graderBooks.map(([k, n, c]) => `<option value="${esc(k)}">${esc(t("{name} — {n} graders", { name: n, n: c }))}</option>`).join("");
  if (ST.gbook && ST.graderBooks.some((b) => b[0] === ST.gbook)) g.value = ST.gbook;
  ST.gbook = g.value || null;
  ST.names.people = {};
  ST.loaded = true;
  statsCompute(false);
  statsFollowScope();
}

function statsPoll(on, where) {
  clearInterval(ST.poll);
  if (!on) return;
  ST.poll = setInterval(async () => {
    const p = await invoke("stats_progress");
    if (p) $(where).textContent = p + "…";
  }, 300);
}

async function statsCompute(recompute) {
  if (ST.busy) { ST.again = true; return; }
  ST.busy = true;
  ST.again = false;
  const scope = ST.scope;
  $("st-status").textContent = recompute ? t("Computing…") : t("Reading the saved results…");
  $("st-recompute").disabled = true;
  statsPoll(true, "st-status");
  try {
    const both = await invoke("stats_corpus", { scope, recompute });
    if (scope !== ST.scope) ST.again = true;
    else { await statsNamesFor(both); statsShowCorpus(both); }
  } catch (e) {
    $("st-status").textContent = t("Could not compute: {error}", { error: String(e) });
  } finally {
    statsPoll(false);
    ST.busy = false;
    $("st-recompute").disabled = false;
    if (ST.again) { ST.again = false; statsCompute(false); }
  }
}

// the names the page shows, asked for once per result: graders, books, narrators
async function statsNamesFor(both) {
  const graders = new Set(), books = new Set(), people = new Map();
  const walk = (one) => {
    const r = one.corpus || {};
    for (const g of r.grades || []) { graders.add(g.grader); books.add(g.book); }
    for (const b of ((r.books || {}).rows) || []) books.add(b.name);
    const n = r.narrators || {};
    for (const k of ["pillars", "weak_pillars", "central"]) for (const p of n[k] || []) people.set(p.id, p.name);
    const ch = r.chains || {};
    for (const s of [...(ch.gap_suspects || []), ...(ch.order_suspects || [])]) {
      people.set(s.student.id, s.student.name);
      people.set(s.teacher.id, s.teacher.name);
    }
    for (const m of ((one.models || {}).models) || []) { graders.add(m.grader); books.add(m.book_name); }
    for (const s of ((one.models || {}).shared) || []) { books.add(s.book); for (const g of s.graders) graders.add(g[0]); }
    const lab = one.lab || {};
    for (const m of lab.hadith || []) { books.add(m.book); for (const n2 of m.names) if (n2.startsWith("Grade: ")) graders.add(n2.slice(7)); }
    for (const x of lab.graders || []) { graders.add(x.a); graders.add(x.b); }
    for (const x of lab.brant || []) graders.add(x.grader);
  };
  for (const one of both.roads || [both]) walk(one);
  const need = [...people].filter(([id]) => !(String(id) in ST.names.people));
  const got = await invoke("stats_names", { graders: [...graders], books: [...books], people: need });
  Object.assign(ST.names.short, got.short);
  Object.assign(ST.names.full, got.full);
  Object.assign(ST.names.books, got.books);
  Object.assign(ST.names.people, got.people);
}

function statsBox(key) { return $("st-box-" + key); }

function statsClear() {
  ST.tables.clear();
  for (const [k] of ST_SECTIONS) if (k !== "terms" && k !== "graders") statsBox(k).innerHTML = "";
}

function stLabel(text, cls, rich) {
  return `<div class="${cls || "lead"}">${rich ? text : esc(text)}</div>`;
}

function statsShowCorpus(both) {
  statsClear();
  if (both.roads) {
    const heads = [[t("Sunni books"), t("Measured on Ibn Hajar's ranks; the chains reach the Prophet ﷺ.")],
      [t("Shia books"), t("Measured apart, on al-Najashi's Imami categories; the chains reach a Ma'sum. The two roads are never measured together.")]];
    both.roads.forEach((road, i) => {
      for (const [k] of ST_SECTIONS) {
        if (k === "terms" || k === "graders") continue;
        statsBox(k).insertAdjacentHTML("beforeend", `<div class="lead st-roadhead"><b>${esc(heads[i][0])}</b> — ${esc(heads[i][1])}</div>`);
      }
      statsShowOne(road);
    });
    ST.result = both;
    return;
  }
  statsShowOne(both);
  ST.result = both;
}

function statsShowOne(both) {
  ST.current = both;
  const r = both.corpus;
  if (r.empty || !(r.overview || {}).chains) {
    for (const [k] of ST_SECTIONS) {
      if (k === "terms" || k === "graders") continue;
      statsBox(k).insertAdjacentHTML("beforeend", stLabel(t("Nothing to measure yet: import a hadith collection (File → Data Sources), and a book of narrators (Ibn Hajar's Taqrib) to identify them.")));
    }
    $("st-status").textContent = "";
    return;
  }
  $("st-status").textContent = (r.from_file ? t("From the saved results — computed once, reused while the data is unchanged.") : t("Computed and saved."))
    + " " + t("Intervals: {boot} bootstrap samples of hadith.", { boot: r.boot });
  statsOverview(r);
  statsNarrators(r);
  statsChains(r);
  statsGrades(r);
  statsModelSection(both.models || {});
  statsLabSection(both.lab || {});
  statsBooks(r);
}

// ------------------------------------------------------------------ building blocks

function statsSay(lines, terms) { ST.pending = [lines.filter((x) => x), terms || []]; }

function statsPlain(lines, terms) {
  let html = `<div class="st-plain"><div class="st-plain-head">${esc(t("In plain words"))}</div>`;
  for (const line of lines) html += `<div class="lead">• ${line}</div>`;
  if (terms && terms.length) {
    html += `<div class="st-plain-terms"><span class="caption">${esc(t("What do these words mean?"))}</span>`;
    for (const key of terms) {
      const info = ST.terms.find((x) => x.key === key);
      if (info) html += `<button class="linkbtn" data-act="term" data-term="${key}" data-tip="${esc(info.what + "\n\n" + info.read)}">${esc(info.name)}</button>`;
    }
    html += "</div>";
  }
  return html + "</div>";
}

function statsTermHtml(info) {
  return `<h3>${esc(info.name)}</h3><p><b>${esc(t("What it measures"))}</b><br>${esc(info.what)}</p>
    <p><b>${esc(t("How to read it"))}</b><br>${esc(info.read)}</p>
    <p><b>${esc(t("What it means for hadith science"))}</b><br>${esc(info.hadith)}</p>
    <p><b>${esc(t("Example"))}</b><br><i>${esc(info.example)}</i></p>`;
}

function statsTerm(key) {
  const info = ST.terms.find((x) => x.key === key);
  if (info) infoDialog(info.name, statsTermHtml(info));
}

function statsFillTerms() {
  const box = statsBox("terms");
  box.innerHTML = stLabel(t("Every figure on these pages, in everyday words: what it measures, how to read it, what it means for the science of hadith, and an example. The same words open from “What do these words mean?” under each finding."))
    + ST.terms.map((info) => `<div class="st-card st-term" data-name="${esc(info.name)}"><div class="lead">${statsTermHtml(info)}</div></div>`).join("");
}

// a finding: a kicker, the sentence that says what the numbers show, the charts, "How to read it" and "Save as CSV…"
function statsFinding(key, kicker, headline, widgets, how, table, into) {
  const id = "c" + (++ST.cardN);
  const name = headline ? headline.replace(/<[^>]+>/g, "").slice(0, 90) : kicker;
  if (table) ST.tables.set(id, { kicker, name, header: table[0], rows: table[1] });
  const pending = ST.pending;
  ST.pending = null;
  let html = `<div class="st-card" data-card="${id}" data-name="${esc(name)}"><button class="st-arm" data-act="arm" data-tip="${esc(t("Export this finding: its chart as a picture or PDF, its figures as a table"))}">⤓</button>
    <div class="st-kicker">${esc(kicker.toUpperCase())}</div><div class="st-headline">${headline}</div>`;
  if (pending && pending[0].length) html += statsPlain(pending[0], pending[1]);
  html += widgets.join("");
  if (how || table) {
    html += `<div class="st-foot">${how ? `<button class="linkbtn" data-act="how">${esc(t("How to read it ▸"))}</button>` : ""}${table ? `<button class="linkbtn" data-act="csv" data-tip="${esc(t("The figures behind this finding, as a table any spreadsheet or statistics program opens"))}">${esc(t("Save as CSV…"))}</button>` : ""}</div>`;
    if (how) html += `<div class="caption st-how" hidden>${how}</div>`;
  }
  html += "</div>";
  (into || statsBox(key)).insertAdjacentHTML("beforeend", html);
}

// ------------------------------------------------------------------ Overview

function statsOverview(r) {
  const o = r.overview, n = r.narrators, w = stWords();
  const shareId = o.links ? o.identified / o.links : 0;
  const reach = o.chains ? o.reaching / o.chains : 0;
  const tiles = stTiles([
    [t("Hadith"), stTh(o.hadith), n_("in {n} book", "in {n} books", o.books)],
    [t("Chains read"), stTh(o.chains), t("{share} reach {end}", { share: stPct(reach), end: w.end })],
    [t("Names identified"), stPct(shareId), t("{identified:,} of {links:,} names in the chains", { identified: o.identified, links: o.links })],
    [t("Narrators"), stTh(o.narrators), t("different people identified in the chains")],
  ]);
  statsSay([esc(t("{share} of the names in the chains are matched to one definite narrator of {rijal}; the other names are shown in the chains but are not counted in the figures below.", { share: stPct(shareId), rijal: w.rijal })),
    esc(t("{reach} of the chains reach {end}.", { reach: stPct(reach), end: w.end }))], ["share"]);
  statsFinding("overview", t("The corpus"), esc(t("{hadith:,} hadith, {chains:,} chains, {narrators:,} narrators identified.", { hadith: o.hadith, chains: o.chains, narrators: o.narrators })), [tiles],
    t("A chain is <b>read</b> when isnady split its text into names; a name is <b>identified</b> when it is matched to one person of {rijal}. Everything below rests on the identified names, so their share is the first thing to know: the measures describe the identified part.", { rijal: w.rijal }),
    [["measure", "value"], [["hadith", o.hadith], ["books", o.books], ["chains", o.chains], ["chains reaching the Prophet", o.reaching],
      ["names in chains", o.links], ["names identified", o.identified], ["narrators", o.narrators], ["chains fully identified", o.full]]]);
  const cats = stGroupCats(r);
  const sumL = n.rank_links.reduce((a, b) => a + b, 0) || 1;
  const strong = (n.rank_links[0] + n.rank_links[1]) / sumL;
  let rows;
  if (stShia()) {
    rows = [[t("All narrators of al-Najashi"), n.rank_all], [t("The narrators in these chains"), n.rank_people], [t("Every name in these chains"), n.rank_links]];
    statsSay([esc(t("Of every 100 times a narrator stands in a chain, {n} are a narrator al-Najashi calls trustworthy or praises.", { n: (100 * strong).toFixed(0) })),
      esc(t("For the science of hadith: the hadith of these books rest mostly on narrators the critics accepted."))], ["share"]);
    statsFinding("overview", t("Reliability"), esc(t("{strong} of the judged transmission passes through narrators al-Najashi calls trustworthy or praises.", { strong: stPct(strong) })), [stStacked(rows, cats)],
      t("al-Najashi's words read in the four Imami categories — trustworthy (thiqa), praised (mamduh), a thiqa of another school (muwaththaq), weak — never on Ibn Hajar's scale. The first bar is everyone he judged; the second, the narrators who appear in these chains; the third counts each narrator as often as he appears. Many of his entries give no judgment (an author listed for his books): those are left out of the bars. Counted, never averaged."),
      [["series", ...cats.map((c) => c[0])], rows.map(([l, v]) => [l, ...v])]);
  } else {
    rows = [[t("All narrators of the Taqrib"), n.rank_all], [t("The narrators in these chains"), n.rank_people], [t("Every name in these chains"), n.rank_links]];
    statsSay([esc(t("Of every 100 times a narrator stands in a chain, {n} are a Companion or a trustworthy narrator.", { n: (100 * strong).toFixed(0) })),
      esc(t("For the science of hadith: the hadith of these books rest mostly on narrators the critics accepted."))], ["share"]);
    statsFinding("overview", t("Reliability"), esc(t("{strong} of all transmission passes through Companions and trustworthy narrators — far more than their share of the Taqrib.", { strong: stPct(strong) })), [stStacked(rows, cats)],
      t("Ibn Hajar's twelve ranks in six groups, from Companions to the rejected. The first bar is everyone Ibn Hajar judged; the second, the narrators who appear in these chains; the third counts each narrator as often as he appears — what the hadith actually rest on. The ranks are ordered categories: counted, never averaged."),
      [["series", ...cats.map((c) => c[0])], rows.map(([l, v]) => [l, ...v])]);
  }
  if (r.grades.length) {
    const gcats = stGradeCats(r);
    const grows = r.grades.map((g) => [`${stShort(g.grader)} — ${stBk(g.book)}`, g.distribution]);
    statsSay([esc(t("Each bar is one scholar and one book: how many of its hadith he put in each grade, from the strongest on the left."))]);
    statsFinding("overview", t("Grades"), esc(t("How each scholar graded each book.")), [stStacked(grows, gcats, 260)],
      stShia() ? t("Each bar is one scholar's grades of one book, in the four Imami categories — sahih (every narrator an Imami thiqa), hasan (a praised Imami), muwaththaq (a thiqa of another school), da'if — as al-Majlisi and al-Behbudi wrote them; majhul, mursal and marfu' count among the da'if. al-Behbudi's Sahih al-Kafi is his selection: he names only the hadith he holds sahih.")
        : t("Each bar is one scholar's grades of one book, in five ordered groups. Grades of the chain alone (\"Isnaad Sahih\") are left out: they judge another thing than a grade of the hadith."),
      [["grader", "book", ...r.grade_labels], r.grades.map((g) => [stShort(g.grader), g.book, ...g.distribution])]);
  }
}

// ------------------------------------------------------------------ Narrators

function statsNarrators(r) {
  const n = r.narrators, c = n.concentration, w = stWords();
  if (c.gini != null) {
    const tiles = stTiles([[t("Busiest 1%"), stPct(c.top1), t("of all identified names")],
      [t("Busiest 10%"), stPct(c.top10), t("of all identified names")],
      [t("Half of all names"), stTh(c.half_by), t("narrators, of {people:,}", { people: c.people })],
      [t("Gini"), c.gini.toFixed(2), t("0 equal shares, 1 one man carries all")]]);
    statsSay([esc(t("The busiest tenth of the narrators carry {top10} of all appearances in the chains.", { top10: stPct(c.top10) })),
      esc(t("Gini {g:.2f}: {words}.", { g: c.gini, words: statsWord("gini", c.gini) })),
      esc(t("For the science of hadith: these few are the pivots (madar) the critics speak of — the great narrators most hadith pass through."))], ["gini", "lorenz"]);
    statsFinding("narrators", t("Concentration"), esc(t("The busiest 10% of narrators carry {top10} of the transmission; {half_by:,} narrators carry half of it.", { top10: stPct(c.top10), half_by: c.half_by })),
      [tiles, stLorenz(c.curve)],
      t("The curve sorts the narrators from the least to the most often seen and adds up their share of the identified names. The diagonal would be every narrator carrying the same; the further the curve sags below it, the more the corpus rests on a few — the madar (pivot) narrators of the classical critics. The Gini coefficient is the area between the two, doubled."),
      [["share of narrators", "share of names"], c.curve]);
  }
  if (n.pillars.length) {
    statsSay([esc(t("The longer the bar, the more often the narrator stands in these chains; {first} comes first, {times:,} times.", { first: stName(n.pillars[0].id, n.pillars[0].name), times: n.pillars[0].links }))]);
  }
  statsFinding("narrators", t("The pillars"), esc(t("The narrators the most hadith pass through.")), [stPeople(n.pillars.slice(0, 20))],
    t("Counted from the identified names of these chains: how often a narrator appears, and (on hover) in how many hadith. The dot is {critic}'s judgment of him. Click a name to open him.", { critic: w.critic }),
    [["narrator", "arabic", "rank", "tabaqa", "times", "hadith"], n.pillars.map((p) => [stName(p.id, p.name), p.name, p.rank, p.tabaqa, p.links, p.hadith])]);
  if (n.weak_pillars.length) {
    statsSay([esc(t("These are the weak or unknown narrators met most often. A hadith that passes through one of them is accepted only when other routes support it."))], ["weakest"]);
    statsFinding("narrators", t("Weak narrators"), esc(t("The weak or unknown narrators the most hadith pass through — where a grade would hang on one man.")), [stPeople(n.weak_pillars)],
      stShia() ? t("Narrators al-Najashi calls weak (da'if, confused, accused, extremist), by how often they appear. A hadith that passes through one of them is graded on its other routes — or stays weak.")
        : t("Narrators of rank 7 or weaker (mastur, da'if, majhul and below), by how often they appear. A hadith that passes through one of them is graded on its other routes — or stays weak."),
      [["narrator", "arabic", "rank", "times", "hadith"], n.weak_pillars.map((p) => [stName(p.id, p.name), p.name, p.rank, p.links, p.hadith])]);
  }
  if (n.central.length) {
    statsSay([esc(t("The higher the figure, the more chains run through the narrator on their way to {end}; {first} leads.", { end: w.end, first: stName(n.central[0].id, n.central[0].name) }))], ["pagerank"]);
    statsFinding("narrators", t("The network"), esc(t("The narrators the transmission flows through, towards {end}.", { end: w.end })), [stPeople(n.central, (p) => 100 * p.score, " ‰")],
      t("PageRank on the graph of who narrates from whom (each pair of neighbours in a chain, weighted by how often): a narrator ranks high when many chains lead to him and through him, not only when he appears often. The figure is his share of the flow in thousandths."),
      [["narrator", "arabic", "pagerank", "times"], n.central.map((p) => [stName(p.id, p.name), p.name, Number(p.score.toFixed(6)), p.links])]);
  }
  const cats = stGroupCats(r);
  const tab = (WIN.people || {}).tabaqat || {};
  const rows = [];
  for (let k = 1; k <= 12; k++) {
    const row = n.tabaqa_groups[k - 1];
    if (row.reduce((a, b) => a + b, 0)) rows.push([`${k} · ${t(tab[k] || "").split(" (")[0]}`, row]);
  }
  if (rows.length) {
    statsSay([esc(t("Each bar is one generation (tabaqa); the blue part is its accepted narrators, the red part the rejected ones."))]);
    statsFinding("narrators", t("Generations"), esc(t("Reliability generation by generation (tabaqa).")), [stStacked(rows, cats, 250)],
      t("Ibn Hajar's twelve generations, from the Companions (1) to the teachers of the six books' compilers (10–12); each bar shows how the narrators of that generation found in these chains are ranked."),
      [["tabaqa", ...cats.map((c) => c[0])], rows.map(([l, v]) => [l, ...v])]);
  }
}

// the plain-words sentences (stats_words), computed here as there
function statsWord(kind, v) {
  const a = Math.abs(v);
  const pick = {
    gini: v < 0.3 ? "evenly shared" : v < 0.5 ? "somewhat uneven" : v < 0.7 ? "uneven: a minority carries most" : "very uneven: a few carry most of it",
    alpha: v >= 0.8 ? "strong agreement" : v >= 0.667 ? "enough agreement for tentative conclusions" : "weak agreement",
    strength: v == null ? "not measurable" : a < 0.1 ? "hardly any relation" : a < 0.3 ? "a weak relation" : a < 0.5 ? "a moderate relation" : "a strong relation",
    r2: v < 0.1 ? "a small part" : v < 0.2 ? "a usual share for human judgement" : "a very good share for human judgement",
    somers: v < 0.2 ? "an order hardly better than chance" : v < 0.4 ? "a fair order" : v < 0.6 ? "a good order" : "a very good order",
    cliff: a < 0.15 ? "a negligible difference" : a < 0.33 ? "a small difference" : a < 0.47 ? "a medium difference" : "a large difference",
  }[kind];
  return t(pick);
}
function statsSure(lo, hi) {
  if (lo == null || hi == null) return "";
  return lo > 0 || hi < 0 ? t("too clear to be chance") : t("could still be chance");
}
function statsOdds(x) {
  if (x >= 1.05) return t("{x:.1f} times the odds", { x });
  if (x <= 0.95) return t("about {share} of the odds", { share: (100 * x).toFixed(0) + "%" });
  return t("no change in the odds");
}

// ------------------------------------------------------------------ Chains

function statsChains(r) {
  const ch = r.chains, w = stWords();
  const lengths = new Map(ch.lengths.map(([k, v]) => [Number(k), v]));
  const unsplit = lengths.get(0) || 0;
  lengths.delete(0);
  const maxK = Math.max(1, ...lengths.keys());
  const bins = [];
  for (let k = 1; k <= maxK; k++) bins.push([String(k), lengths.get(k) || 0]);
  const counts = [];
  for (const [k, v] of [...lengths].sort((a, b) => a[0] - b[0])) for (let i = 0; i < v; i++) counts.push(k);
  const median = counts.length ? counts[Math.floor(counts.length / 2)] : 0;
  statsSay([esc(t("Half of the chains have {median} narrators or fewer.", { median })),
    esc(t("For the science of hadith: fewer narrators is a 'high' chain (isnad 'ali), prized by the muhaddithun; more is a 'low' one (nazil)."))], ["median"]);
  statsFinding("chains", t("Length"), esc(t("Most chains have {median} narrators between the compiler and {end}.", { median, end: w.end })),
    [stColumns(bins, " " + t("chains"))],
    t("The number of names in each chain. A short chain (isnad 'ali) has fewer links between the compiler and {end}; a long one (nazil) more. {unsplit:,} chains could not be split into names yet and are not counted.", { end: w.End, unsplit }),
    [["names in the chain", "chains"], [...bins.map(([k, v]) => [k, v]), ["not split", unsplit]]]);
  const cats = stGroupCats(r);
  const sumW = ch.weakest.reduce((a, b) => a + b, 0) || 1;
  const good = ch.weakest.slice(0, stShia() ? 2 : 3).reduce((a, b) => a + b, 0) / sumW;
  const rows = [[t("Chains with a judged narrator"), ch.weakest], [t("Chains fully identified"), ch.weakest_full]];
  statsSay([esc(t("In {good} of the chains every identified narrator is at least acceptable; the rest have at least one weaker man, and are graded on their other routes — or stay weak.", { good: stPct(good) }))], ["weakest"]);
  statsFinding("chains", t("The weakest link"),
    esc(stShia() ? t("In {good} of the chains the weakest narrator is trustworthy or praised.", { good: stPct(good) })
      : t("In {good} of the chains the weakest narrator is truthful or better.", { good: stPct(good) })),
    [stStacked(rows, cats)],
    t("A chain is as strong as its weakest narrator: for each chain, the weakest rank among its identified narrators. The first bar counts every chain with at least one ranked narrator — its weakest may be among the names not yet identified; the second only the chains whose every name is identified."),
    [["series", ...cats.map((c) => c[0])], rows.map(([l, v]) => [l, ...v])]);
  if (!ch.gaps.some(([, v]) => v) && !(ch.order_ok + ch.order_bad)) return;
  const gaps = ch.gaps.map(([k, v]) => [String(k), v]);
  const marked = new Set(ch.gaps.map(([k], i) => (k > 100 || k <= -60 ? i : -1)).filter((i) => i >= 0));
  const outside = ch.gaps.reduce((a, [, v], i) => a + (marked.has(i) ? v : 0), 0);
  statsSay([esc(t("{outside:,} pairs of teacher and student lived too far apart to have met. Each is a name matched to the wrong man, a wrong year, or a break in the chain — to be checked.", { outside }))], ["time_gap"]);
  statsFinding("chains", t("Time"), esc(t("{outside:,} teacher–student pairs lie too far apart in time — each points to a wrongly identified name or a wrong year.", { outside })),
    [stColumns(gaps, " " + t("pairs"), marked, 2), statsSuspects(ch.gap_suspects, true)],
    t("For two neighbours in a chain whose years of death are known: the student's year minus the teacher's. Most students die 20–60 years after their teachers. More than 110 years, or a student dying 60 years or more before his teacher (gold), is hard to believe: the list shows the pairs behind them — check them on the Narrators page (double-click)."),
    [["student", "teacher", "years apart", "times"], ch.gap_suspects.map((s) => [stName(s.student.id, s.student.name), stName(s.teacher.id, s.teacher.name), s.gap, s.times])]);
  const total = ch.order_ok + ch.order_bad;
  if (total) {
    statsSay([esc(t("In {value} of the pairs the student is of the same or a later generation than his teacher, as it must be; the pairs listed are not, and point to wrong identifications.", { value: stPct(ch.order_ok / total) }))], ["tabaqa_order"]);
    statsFinding("chains", t("Generations in order"), esc(t("{value} of the identified pairs are in the right order of generations.", { value: stPct(ch.order_ok / total) })), [statsSuspects(ch.order_suspects, false)],
      t("A student belongs to the same or a later generation (tabaqa) than his teacher. A pair where the student is of an EARLIER generation than his teacher is a sign of a wrong identification; the list shows those met most often."),
      [["student", "student tabaqa", "teacher", "teacher tabaqa", "times"], ch.order_suspects.map((s) => [stName(s.student.id, s.student.name), s.student.tabaqa, stName(s.teacher.id, s.teacher.name), s.teacher.tabaqa, s.times])]);
  }
}

function statsSuspects(list, gap) {
  const rows = list.slice(0, 10).map((s) => ({
    attrs: ` data-narr2="${s.student.id}"`,
    cells: [{ text: stName(s.student.id, s.student.name), tip: s.student.name, narr: s.student.id },
      { text: stName(s.teacher.id, s.teacher.name), tip: s.teacher.name, narr: s.teacher.id },
      gap ? stSigned(s.gap, 0) : `${s.student.tabaqa} → ${s.teacher.tabaqa}`, String(s.times)],
  }));
  return stTable([t("Student"), t("Teacher"), gap ? t("Years apart") : t("Generations"), t("Times")], rows);
}

// ------------------------------------------------------------------ Grades and chains

function statsGrades(r) {
  const w = stWords(), box = statsBox("grades");
  const rows = r.grades.filter((g) => g.weakest && g.weakest.tau != null);
  const one = r.grades.filter((g) => !rows.includes(g) && g.distribution.filter((x) => x).length === 1);
  if (one.length) {
    box.insertAdjacentHTML("beforeend", stLabel(one.map((g) => t("{grader} gives one grade only in {book} — {grade} for all {n:,} he names",
      { grader: stShort(g.grader), book: stBk(g.book), n: g.graded, grade: t(r.grade_labels[g.distribution.findIndex((x) => x)]) })).join("; ")
      + ". " + t("A single grade cannot go with anything, so he is left out below; how he stands beside the other graders is in Graders."), "caption"));
  }
  if (!rows.length) {
    box.insertAdjacentHTML("beforeend", stLabel(t("No grades to compare: import a collection whose hadith are graded.")));
    return;
  }
  const plot = rows.map((g) => [`${stShort(g.grader)} — ${stBk(g.book)}`, g.weakest.tau, g.weakest.tau_ci || [g.weakest.tau, g.weakest.tau]]);
  const lo = Math.min(...plot.map((p) => p[1])), hi = Math.max(...plot.map((p) => p[1]));
  const clear = plot.filter((p) => p[2][0] > 0);
  const span = plot.length === 1 ? lo.toFixed(2) : `${lo.toFixed(2)} – ${hi.toFixed(2)}`;
  let headline;
  if (plot.length === 1) {
    const who = esc(stShort(rows[0].grader));
    headline = clear.length ? t("The weaker a chain's weakest narrator, the lower {who}'s grade (Kendall's τ<sub>b</sub> {tau}).", { who, tau: span })
      : t("{who}'s grade does not follow the weakest narrator beyond chance (τ<sub>b</sub> {tau}).", { who, tau: span });
  } else if (clear.length === plot.length) {
    headline = t("The weaker a chain's weakest narrator, the lower the grade — for every grader (Kendall's τ<sub>b</sub> {tau}).", { tau: span });
  } else {
    headline = t("The weaker a chain's weakest narrator, the lower the grade — for {n} of {total} graders beyond chance (τ<sub>b</sub> {tau}).", { n: clear.length, total: plot.length, tau: span });
  }
  statsSay([...plot.slice(0, 5).map(([label, tau, ci]) => t("{who}: τb {tau:.2f} — {strength}, {sure}.", { who: esc(label), tau, strength: statsWord("strength", tau), sure: statsSure(ci[0], ci[1]) })),
    esc(t("A positive value: the weaker the chain's weakest narrator, the weaker the scholar's grade. It is not near 1 because a scholar also weighs other routes, hidden defects and the text."))],
  ["kendall", "interval", "bootstrap", "gamma"]);
  statsFinding("grades", t("The weakest link and the grade"), headline, [stInterval(plot, t("weaker narrator, higher grade"), t("weaker narrator, lower grade"))],
    t("For each graded hadith: the weakest identified narrator of its best chain against the scholar's grade, both as ordered categories. Kendall's τ<sub>b</sub> is a rank correlation that allows ties: 0 is no relation, 1 a perfect one. The dot is the value, the line its 95% interval (a bootstrap over hadith). A relation well above zero shows the grades follow the narrators — and how much they do not shows what else a scholar weighed: other routes (mutaba'at, shawahid), hidden defects ('ilal)."),
    [["grader", "book", "hadith", "tau-b", "tau-b low", "tau-b high", "gamma"],
      rows.map((g) => [stShort(g.grader), g.book, g.weakest.n, Number(g.weakest.tau.toFixed(4)), ...(g.weakest.tau_ci || []).map((x) => Number(x.toFixed(4))),
        g.weakest.gamma != null ? Number(g.weakest.gamma.toFixed(4)) : ""])]);
  const groupNames = r.groups.map(([n]) => t(n));
  let grid = "";
  for (const g of rows.slice(0, 6)) {
    const table = g.weakest.table;
    const cells = table.map((row) => {
      const total = row.reduce((a, b) => a + b, 0);
      return row.map((cnt) => [total ? cnt / total : null, [total ? `${(100 * cnt / total).toFixed(0)}%` : "—"]]);
    });
    const keep = table.map((row, k) => (row.reduce((a, b) => a + b, 0) ? k : -1)).filter((k) => k >= 0);
    grid += stLabel(`<b>${esc(stShort(g.grader))}</b> — ${esc(stBk(g.book))}`, "lead", true)
      + stHeat(keep.map((k) => groupNames[k]), r.grade_labels.map((x) => t(x)), keep.map((k) => cells[k]),
        { cw: 80, ch: 40, labelW: 170, headH: 52, corner: t("weakest narrator ↓\ngrade →") });
  }
  statsSay([esc(t("Read a row: of the hadith whose weakest narrator is of that level, the share the scholar put in each grade. A sahih grade in a weak row is a hadith the scholar found supported by other routes."))], ["weakest"]);
  statsFinding("grades", t("Cross-tables"), esc(t("How each grade is spread over the weakest narrator.")), [grid],
    t("Rows: the weakest narrator of the hadith's best chain; columns: the grade. Each row adds up to 100%. A sahih grade where the weakest narrator is weak is a hadith strengthened by other routes, or {otherwise}.", { otherwise: w.otherwise }) + (stShia() ? " " + t(ST_PRAISED_NOTE) : ""),
    [["grader", "book", "weakest narrator", ...r.grade_labels], rows.flatMap((g) => groupNames.map((gn, k) => [stShort(g.grader), g.book, gn, ...g.weakest.table[k]]))]);
  const lengths = rows.filter((g) => g.length && g.length.tau != null);
  if (lengths.length) {
    const lp = lengths.map((g) => [`${stShort(g.grader)} — ${stBk(g.book)}`, g.length.tau, g.length.tau_ci || [g.length.tau, g.length.tau]]);
    const weakTau = new Map(rows.map((g) => [g.grader + g.book, g.weakest.tau]));
    const lower = lp.filter((p) => p[2][0] > 0), higher = lp.filter((p) => p[2][1] < 0);
    const smaller = lengths.every((g) => Math.abs(g.length.tau) < Math.abs(weakTau.get(g.grader + g.book) || 0));
    let h;
    if (!lower.length && !higher.length) h = t("The length of a chain does not go with the grade beyond chance.");
    else if (lower.length === lp.length) h = smaller ? t("Longer chains are graded a little lower — much less than weaker narrators are.") : t("Longer chains are graded a little lower.");
    else if (higher.length === lp.length) h = smaller ? t("Longer chains are graded a little higher — a smaller relation than the weakest narrator's.") : t("Longer chains are graded a little higher.");
    else h = smaller ? t("Length goes with the grade for {n} of {total} graders, in different directions — always less than the weakest narrator.", { n: lower.length + higher.length, total: lp.length })
      : t("Length goes with the grade for {n} of {total} graders, in different directions.", { n: lower.length + higher.length, total: lp.length });
    statsSay(lp.slice(0, 5).map(([label, tau, ci]) => t("{who}: τb {tau:.2f} — {strength}, {sure}.", { who: esc(label), tau, strength: statsWord("strength", tau), sure: statsSure(ci[0], ci[1]) })), ["kendall", "interval"]);
    statsFinding("grades", t("Length and grade"), esc(h), [stInterval(lp, t("longer, higher grade"), t("longer, lower grade"))],
      t("The same rank correlation between the number of names in the hadith's chain and its grade. Its size beside the weakest-link relation shows which of the two the grades follow."),
      [["grader", "book", "tau-b", "low", "high"], lengths.map((g) => [stShort(g.grader), g.book, Number(g.length.tau.toFixed(4)), ...(g.length.tau_ci || []).map((x) => Number(x.toFixed(4)))])]);
  }
}

// ------------------------------------------------------------------ The hadith model

function statsModelSection(models) {
  const box = statsBox("model");
  const ms = models.models || [];
  if (!ms.length) {
    box.insertAdjacentHTML("beforeend", stLabel(t("No model yet: it needs a book whose hadith are graded and whose chains are split into names (at least a hundred graded hadith, in two grades or more).")));
    return;
  }
  if (stShia()) {
    box.insertAdjacentHTML("beforeend", stLabel(t("Road B: the grade is one of the four Imami categories (sahih, hasan, muwaththaq, da'if), the narrators are in al-Najashi's categories, a chain ends at a Ma'sum. al-Behbudi gives one grade only (his book is a selection of what he holds sahih), so there is no model of his grades."), "caption"));
  }
  const idx = Math.min(ST.modelIndex, ms.length - 1);
  box.insertAdjacentHTML("beforeend", `<div class="st-modelwrap"><div class="st-pick"><span class="np-ftitle">${esc(t("Model of"))}</span>
    <select data-pick="model">${ms.map((m, i) => `<option value="${i}"${i === idx ? " selected" : ""}>${esc(`${stShort(m.grader)} — ${stBk(m.book_name)}`)}</option>`).join("")}</select></div>
    <div class="st-modelbox"></div></div>`);
  const wrap = box.lastElementChild;
  wrap._ms = ms;
  wrap._own = ST.current;
  statsModelShow(wrap);
  statsShared(models.shared || []);
}

function statsModelShow(wrap) {
  const shown = ST.current;
  ST.current = wrap._own;
  const into = wrap.querySelector(".st-modelbox");
  for (const c of into.querySelectorAll(".st-card")) ST.tables.delete(c.dataset.card);
  into.innerHTML = "";
  const i = Number(wrap.querySelector("select").value);
  statsModelFindings(wrap._ms[i], wrap._ms, into);
  ST.current = shown;
}

function statsModelFindings(m, all, into) {
  const name = stShort(m.grader), w = stWords();
  const coefs = m.coefficients;
  const sig = coefs.filter((c) => c.p < 0.05 && c.beta > 0);
  const top = sig.length ? sig.reduce((a, b) => (b.beta > a.beta ? b : a)) : null;
  let headline;
  if (top && top.name.startsWith("Weakest: ")) headline = t("For {name}, a weakest narrator who is {level} multiplies the odds of a weaker grade by {odds:.1f}.", { name, level: t(top.name.slice(9)).toLowerCase(), odds: top.or });
  else if (top) headline = t("For {name}, {factor} multiplies the odds of a weaker grade by {odds:.1f}.", { name, factor: stFactor(top.name).toLowerCase(), odds: top.or });
  else headline = t("For {name}, no factor of the chain moves the grade beyond chance.", { name });
  const plot = coefs.map((c) => [stFactor(c.name), c.beta, [c.beta - 1.96 * c.se, c.beta + 1.96 * c.se]]);
  const fmt = (v, lo, hi) => `OR ${Math.exp(v).toFixed(2)}  [${Math.exp(lo).toFixed(2)}–${Math.exp(hi).toFixed(2)}]`;
  statsSay([...[...sig].sort((a, b) => b.beta - a.beta).slice(0, 4).map((c) => t("{factor}: {odds} of a weaker grade — {sure}.", { factor: esc(stFactor(c.name)), odds: statsOdds(c.or), sure: statsSure(c.or_ci[0] - 1, c.or_ci[1] - 1) })),
    esc(t("Each factor is measured with the others held equal: it is credited only with its own part."))], ["odds", "ordinal", "interval"]);
  statsFinding("model", t("What the grade follows"), esc(headline), [stInterval(plot, t("a stronger grade"), t("a weaker grade"), fmt, 250)],
    t("An ordinal (proportional-odds) regression of the scholar's grade on the hadith's best chain. Each row is one factor; the dot is its effect on the log-odds of a WEAKER grade, the line its 95% interval, and the figure at the right the odds ratio: 2 doubles the odds of a weaker grade, 0.5 halves them, 1 is no effect. The weakest narrator is compared with a chain whose weakest is trustworthy (thiqa); the other factors are per unit. All are measured together, so each is its effect with the others held equal.") + (stShia() ? " " + t(ST_PRAISED_NOTE) : ""),
    [["factor", "beta", "se", "odds ratio", "low", "high", "p"], coefs.map((c) => [c.name, Number(c.beta.toFixed(5)), Number(c.se.toFixed(5)), Number(c.or.toFixed(4)), Number(c.or_ci[0].toFixed(4)), Number(c.or_ci[1].toFixed(4)), c.p])], into);
  const tests = [...m.tests].sort((a, b) => b.chi2 - a.chi2);
  const matter = tests.filter((x) => x.p_holm < 0.05);
  const total = m.lr_all.chi2 || 1;
  const table = stTable([t("Factor"), "χ²", "df", t("p (Holm)"), [t("Share explained"), t("The factor's χ² beside the whole chain's: its share of what the chain explains")]],
    tests.map((x) => ({ cells: [stFactor(x.label), x.chi2.toFixed(1), String(x.df), x.p_holm < 0.001 ? "< 0.001" : x.p_holm.toFixed(3), stPct(x.chi2 / total)].map((v) => ({ text: v, muted: x.p_holm >= 0.05 })) })));
  const lead = tests[0];
  statsSay([...(lead ? [t("{factor} decides the most: {share} of what the chain explains.", { factor: esc(stFactor(lead.label)), share: stPct(lead.chi2 / total) })] : []),
    esc(t("Factors in grey could be chance; the others matter beyond chance."))], ["lr", "pvalue"]);
  statsFinding("model", t("Which factors matter"),
    esc(lead ? t("{matter} of {tests} factors matter; {factor} alone carries {share} of what the chain explains.", { matter: matter.length, tests: tests.length, factor: stFactor(lead.label).toLowerCase(), share: stPct(lead.chi2 / total) }) : t("No factor to test.")),
    [table],
    t("For each factor the model is fitted again without it; the likelihood-ratio χ² is how much worse it then fits, with its degrees of freedom. p is adjusted for testing several factors at once (Holm). The last column is the factor's χ² beside the whole chain's (all factors against none) — overlapping factors can add up to more than 100%."),
    [["factor", "chi2", "df", "p", "p holm"], tests.map((x) => [x.label, Number(x.chi2.toFixed(3)), x.df, x.p, x.p_holm])], into);
  const tiles = stTiles([
    [t("Hadith"), stTh(m.n), t("graded by {name}, with a chain", { name })],
    [t("Right"), stPct(m.accuracy), t("against {share} by always saying “{grade}”", { share: stPct(m.base_rate), grade: stGrade(m.base_label || m.labels[0]) })],
    ["McFadden R²", m.mcfadden != null ? m.mcfadden.toFixed(3) : "—", t("0.2–0.4 is a very good fit")],
    ["Somers' D", m.somers_d != null ? m.somers_d.toFixed(2) : "—", t("ordering: 0 none, 1 perfect")],
  ]);
  statsSay([esc(t("The model's most likely grade is the scholar's own in {right} of the hadith; always saying the most common grade would give {base}.", { right: stPct(m.accuracy), base: stPct(m.base_rate) })),
    m.mcfadden != null ? esc(t("R² {r2:.3f}: {words}.", { r2: m.mcfadden, words: statsWord("r2", m.mcfadden) })) : "",
    m.somers_d != null ? esc(t("Somers' D {d:.2f}: {words}.", { d: m.somers_d, words: statsWord("somers", m.somers_d) })) : "",
    esc(t("The rest is what the scholar weighed beyond this one chain: other routes, hidden defects, the text."))], ["r2", "somers", "calibration"]);
  statsFinding("model", t("How much the chain explains"), esc(t("The chain explains part of {name}'s grades, not most: the rest is what a scholar weighs beyond one chain.", { name })),
    [tiles, stCalibration(m.calibration, t("Calibration: the model's probability of “{grade}” (across) against the share of hadith that got it (up). Points on the diagonal: the model's probabilities can be taken at their word.", { grade: stGrade(m.labels[0]) }))],
    t("McFadden's R² compares the model's likelihood with a model that knows only how often each grade is given; for a model of human judgement values of 0.1–0.2 are usual and 0.2–0.4 very good. 'Right' counts the hadith whose most likely grade is the scholar's. Somers' D measures how well the model's expected grade orders the hadith. ")
      + (stShia() ? t("'Right' near the always-the-same figure does not mean the chain tells nothing — Somers' D and R² show how far it orders the grades. On this road the grade is of the chain itself, so what the model misses is mostly narrators the scholar judges otherwise than al-Najashi, and names isnady did not identify.")
        : t("That the chain explains only part of the grade is itself the finding: a scholar grades a hadith on all its routes and on its text, not on one chain.")),
    [["predicted (mean)", "observed", "hadith"], m.calibration], into);
  const surprises = stShia() ? [
    ["graded_higher", "Graded higher than the chain foretells", "{n:,} hadith {name} grades stronger than their chain, as isnady reads it, would foretell.",
      "Hadith whose grade is STRONGER than the model's most likely grade, the least expected first: the probability is the model's for this grade or a stronger one. The Imami categories judge the chain by its narrators, so the usual reasons are a narrator al-Majlisi holds stronger than al-Najashi's words (he also draws on al-Tusi, al-Kashshi and the later critics, and on the shaykhs of ijaza), or a name isnady matched to the wrong man of the same name. Double-click to read it in its book."],
    ["graded_lower", "Graded lower than the chain foretells", "{n:,} hadith {name} grades weaker than their chain, as isnady reads it, would foretell.",
      "Hadith whose grade is WEAKER than the model's most likely grade, the least expected first. The usual reasons are a narrator al-Majlisi holds weaker or unknown (majhul) where al-Najashi is silent or praises, a break in the chain (mursal, marfu') that the split into names does not show, or a wrong match. Double-click to read it in its book."],
  ] : [
    ["graded_higher", "Graded higher than the chain foretells", "{n:,} hadith {name} grades stronger than their chain alone would — candidates for strengthening by other routes (mutaba'at, shawahid).",
      "Hadith whose grade is STRONGER than the model's most likely grade, the least expected first: the probability is the model's for this grade or a stronger one. A weak chain graded sahih or hasan is usually a hadith strengthened by its other routes (hasan li-ghayrihi, sahih li-ghayrihi) — or a narrator the scholar judges otherwise than Ibn Hajar. Double-click to read it in its book."],
    ["graded_lower", "Graded lower than the chain foretells", "{n:,} hadith {name} grades weaker than their chain alone would — candidates for a hidden defect ('illa) or an irregular text (shudhudh).",
      "Hadith whose grade is WEAKER than the model's most likely grade, the least expected first. A chain of trustworthy narrators graded weak points to what the chain does not show: a hidden defect ('illa), a contradiction of stronger narrators (shudhudh), a break the names do not reveal (irsal khafi, tadlis). Double-click to read it in its book."],
  ];
  for (const [side, title, sentence, how] of surprises) {
    const items = m[side];
    if (!items.length) continue;
    const strong = m[side + "_strong"] || 0;
    const text = t(sentence, { n: m[side + "_total"], name }) + " " + (strong ? t("{strong:,} of them clearly (the model gave that grade 5% or less).", { strong })
      : t("None clearly: the model gave each grade more than 5%."));
    statsSay([esc(t("Each row is a hadith worth reading: the scholar saw in it more than its one chain shows. Double-click to open it."))], ["surprise"]);
    statsFinding("model", t(title), esc(text), [statsSurpriseTable(items, m.labels)], t(how),
      [["hadith", "grade", "model's most likely", "probability", "weakest narrator", "chain length", "chains"],
        items.map((d) => [d.number, d.grade, d.likely, Number(d.p.toFixed(5)), d.factors.weakest, d.factors.length, d.factors.routes])], into);
  }
  const same = all.filter((x) => x.book === m.book);
  if (same.length > 1) {
    const rows = [];
    for (const cName of coefs.filter((c) => c.block === "weakest").map((c) => c.name)) {
      for (const x of same) {
        const c = x.coefficients.find((c2) => c2.name === cName);
        if (c) rows.push([`${t(cName.replace("Weakest: ", ""))} — ${stShort(x.grader)}`, c.beta, [c.beta - 1.96 * c.se, c.beta + 1.96 * c.se]]);
      }
    }
    statsSay([esc(t("The further a scholar's line lies towards “a weaker grade”, the more his grade drops when the narrator is weak."))], ["odds"]);
    statsFinding("model", t("The scholars compared"), esc(t("How strongly each scholar's grade follows the weakest narrator.")),
      [stInterval(rows, t("a stronger grade"), t("a weaker grade"), fmt, 300)],
      t("The same model fitted to each scholar's grades of this book. Where one scholar's odds ratio for a weak narrator is larger than another's, he leans more on {critic}'s verdicts on the narrators; a smaller one shows a scholar who more often finds the hadith strengthened elsewhere — or who judges the narrator otherwise.", { critic: w.critic }),
      [["level", "grader", "beta", "low", "high"], rows.map((x) => [x[0].split(" — ")[0], x[0].split(" — ")[1], Number(x[1].toFixed(5)), Number(x[2][0].toFixed(5)), Number(x[2][1].toFixed(5))])], into);
  }
}

function statsSurpriseTable(items, labels) {
  const rows = items.slice(0, 25).map((d) => {
    const probs = labels.map((lab, i) => `${stGrade(lab)} ${(100 * d.probs[i]).toFixed(0)}%`).join(" · ");
    const tip = t("The model's grades for this hadith: {probs}", { probs });
    const f = d.factors;
    return { attrs: ` data-hadith="${d.hadith_id}"`, cells: [String(d.number), ...[stGrade(d.grade), stGrade(d.likely), (100 * d.p).toFixed(1) + "%", t(f.weakest), `${f.length} / ${f.unidentified}`].map((v) => ({ text: v, tip }))] };
  });
  return stTable([t("Hadith"), t("Grade"), t("Model's most likely"), t("Probability"), t("Weakest narrator"), t("Names / not identified")], rows);
}

function statsShared(shared) {
  if (!shared.length) return;
  const rows = shared.slice(0, 30).map((s) => ({
    attrs: ` data-hadith="${s.hadith_id}"`,
    cells: [{ text: `${s.number}  ${s.side === "graded_higher" ? "▲" : "▼"}`, tip: t("▲ graded higher than the chain foretells · ▼ graded lower") }, stBk(s.book),
      { text: s.graders.map(([g, gr]) => `${stShort(g)}: ${stGrade(gr)}`).join("; "),
        tip: s.graders.map(([g, gr, likely, p]) => t("{grader}: {grade} — the model expected {likely} ({share:.1f}% for this or further)", { grader: stShort(g), grade: stGrade(gr), likely: stGrade(likely), share: 100 * p })).join("\n") },
      t(s.factors.weakest)],
  }));
  const up = shared.filter((s) => s.side === "graded_higher").length;
  statsSay([esc(t("When several scholars are surprised by the same hadith in the same way, the reason lies in the hadith itself: its other routes, or a defect they all saw."))], ["surprise"]);
  statsFinding("model", t("Surprises the scholars share"), esc(t("{shared} hadith surprise two scholars or more in the same way — {up} graded higher than their chain, {value} lower.", { shared: shared.length, up, value: shared.length - up })),
    [stTable([t("Hadith"), t("Book"), t("Scholars and their grades"), t("Weakest narrator")], rows)],
    t("One scholar's surprise may be his own judgement or a slip; the same surprise in several scholars' grades points to the hadith itself — its other routes, or a defect they all saw. ▲ higher than the chain foretells, ▼ lower. Double-click to read it in its book."),
    [["hadith", "side", "book", "graders", "weakest narrator"], shared.map((s) => [s.number, s.side, s.book, s.graders.map(([g, gr]) => `${g}: ${gr}`).join("; "), s.factors.weakest])]);
}

// ------------------------------------------------------------------ Correlations and tests

function stPairsTable(m, which) {
  const rows = [];
  const grid = m[which] || [];
  for (let i = 0; i < grid.length; i++) {
    for (let j = i + 1; j < grid.length; j++) {
      const c = grid[i][j];
      if (c && c.rho != null) rows.push([m.names[i], m.names[j], c.rho, c.n, c.p, c.q ?? null]);
    }
  }
  return rows.map((r, i) => [r, i]).sort((a, b) => Math.abs(b[0][2]) - Math.abs(a[0][2]) || a[1] - b[1]).map((x) => x[0]);
}

function stAdjustedTable(m) {
  const rows = [];
  (m.adjusted || []).forEach((a, j) => {
    const col = m.factors + j;
    a.cells.forEach((c, i) => {
      const plain = m.cells[i][col];
      rows.push([a.outcome, m.names[i], plain ? plain.rho : null, c ? c.rho : null, c ? c.n : null, c ? c.q : null]);
    });
  });
  return rows;
}

function statsLabSection(r) {
  const box = statsBox("lab");
  const ms = r.hadith || [];
  if (!ms.length && !r.narrators) {
    box.insertAdjacentHTML("beforeend", stLabel(t("Nothing to correlate yet: it needs a book whose chains are split into names.")));
    return;
  }
  box.insertAdjacentHTML("beforeend", stLabel(t("Every measure against every other, by rank (Spearman's ρ): +1 the two rise together, −1 one falls as the other rises, 0 no relation. Ranks and grades are ordered, never averaged. With so many pairs some look related by chance, so each ρ carries a q-value (Benjamini–Hochberg): below 0.05 it is shown in colour, otherwise only outlined.")));
  const idx = Math.min(ST.labIndex, Math.max(0, ms.length - 1));
  box.insertAdjacentHTML("beforeend", `<div class="st-labwrap">${ms.length > 1 ? `<div class="st-pick"><span class="np-ftitle">${esc(t("Book"))}</span>
    <select data-pick="lab">${ms.map((m, i) => `<option value="${i}"${i === idx ? " selected" : ""}>${esc(stBk(m.book))}</option>`).join("")}</select></div>` : ""}
    <div class="st-labbox"></div></div>`);
  const wrap = box.lastElementChild;
  wrap._r = r;
  wrap._own = ST.current;
  wrap._idx = idx;
  statsLabShow(wrap);
  if (r.narrators) statsLabNarrators(r.narrators);
  if (r.books) statsLabBooks(r.books);
}

function statsLabShow(wrap) {
  const shown = ST.current;
  ST.current = wrap._own;
  const into = wrap.querySelector(".st-labbox");
  for (const c of into.querySelectorAll(".st-card")) ST.tables.delete(c.dataset.card);
  into.innerHTML = "";
  const sel = wrap.querySelector("select");
  const i = sel ? Number(sel.value) : wrap._idx;
  const ms = wrap._r.hadith || [];
  if (ms.length) {
    statsLabBook(ms[i], (wrap._r.graders || []).filter((x) => x.book === ms[i].book), into);
    statsLabBrant((wrap._r.brant || []).filter((b) => b.book === ms[i].book), into);
  }
  ST.current = shown;
}

function stRhoCell(c, extra) {
  if (!c || c.rho == null) return [null, ["—"]];
  const text = stSigned(c.rho, 2);
  if (c.q != null && c.q < ST_ALPHA) return [c.rho, [text, extra || stQ(c.q)]];
  return [null, [text]];
}

function stRhoTip(a, b, c, held) {
  if (!c || c.rho == null) return t("{a} × {b}: too few cases", { a, b });
  const sure = c.q != null && c.q < ST_ALPHA ? t("a relation beyond chance") : t("could be chance");
  return `${a} × ${b}` + (held ? " — " + t("the others held equal") : "") + "\n"
    + t("Spearman's ρ {rho:+.3f} on {n:,} cases", { rho: c.rho, n: c.n }) + `\np ${stG(c.p, 2)} · ${stQ(c.q)} — ${sure}`;
}

function statsMatrix(m, which, names, cell, labelW) {
  const grid = m[which];
  const k = names.length;
  const cells = [], tips = [];
  for (let i = 0; i < k; i++) {
    cells.push(names.map((_n, j) => (i === j ? [null, []] : stRhoCell(grid[i][j]))));
    tips.push(names.map((_n, j) => (i === j ? "" : stRhoTip(names[i], names[j], grid[i][j], which === "partial"))));
  }
  return stHeat(names, names, cells, { low: -1, high: 1, diverging: true, cw: cell[0], ch: cell[1], labelW, tips, headH: 60 });
}

function statsSwitchable(m, names, cell, labelW) {
  const id = "sw" + (++ST.cardN);
  setTimeout(() => {
    const el = document.getElementById(id);
    if (el) Object.assign(el, { _m: m, _names: names, _cell: cell, _labelW: labelW });
  }, 0);
  return `<div class="st-switch" id="${id}"><div class="st-pick"><span class="np-ftitle">${esc(t("Show"))}</span><select data-pick="matrix">
    <option value="cells">${esc(t("Each pair alone"))}</option>${m.partial ? `<option value="partial">${esc(t("The others held equal (partial)"))}</option>` : ""}</select></div>
    <div class="st-switch-body">${statsMatrix(m, "cells", names, cell, labelW)}</div></div>`;
}

function statsLabBook(m, tests, into) {
  const k = m.factors;
  const factors = m.names.slice(0, k);
  const shown = factors.map(stFactor);
  const named = new Map(factors.map((f, i) => [f, shown[i]]));
  const adjusted = m.adjusted || [];
  if (adjusted.length) {
    const rows = adjusted.map((a) => stOutcome(a.outcome));
    const cells = [], tips = [];
    adjusted.forEach((a, j) => {
      const line = [], tip = [];
      a.cells.forEach((c, i) => {
        const plain = m.cells[i][k + j];
        line.push(stRhoCell(c, plain && plain.rho != null ? t("alone {rho:+.2f}", { rho: plain.rho }) : ""));
        tip.push(stRhoTip(rows[j], shown[i], c, true) + (plain && plain.rho != null ? "\n" + t("Alone (nothing held equal): ρ {rho:+.3f}", { rho: plain.rho }) : ""));
      });
      cells.push(line);
      tips.push(tip);
    });
    const best = new Map();
    adjusted.forEach((a, j) => {
      if (!a.outcome.startsWith("Grade: ")) return;
      const sure = a.cells.map((c, i) => [c, i]).filter(([c]) => c && c.q < ST_ALPHA && c.rho > 0).map(([c, i]) => [c.rho, factors[i]]);
      if (sure.length) best.set(rows[j], sure.reduce((x, y) => (y[0] > x[0] || (y[0] === x[0] && y[1] > x[1]) ? y : x)));
    });
    const tops = new Set([...best.values()].map((v) => v[1]));
    let headline;
    if (best.size > 1 && tops.size === 1) {
      const f = [...tops][0];
      const vals = [...best.values()].map((v) => v[0]);
      const lo = Math.min(...vals), hi = Math.max(...vals);
      const rest = adjusted.filter((a) => a.outcome.startsWith("Grade: ")).flatMap((a) => a.cells.map((c, i) => [c, i]).filter(([c, i]) => c && factors[i] !== f && c.q < ST_ALPHA).map(([c]) => c.rho));
      headline = rest.length ? t("With the other factors equal, <i>{factor}</i> still goes with a weaker grade for every scholar (ρ {lo:.2f}–{hi:.2f}); no other factor passes {most:.2f}.", { factor: esc(named.get(f).toLowerCase()), lo, hi, most: Math.max(...rest.map(Math.abs)) })
        : t("With the other factors equal, <i>{factor}</i> still goes with a weaker grade for every scholar (ρ {lo:.2f}–{hi:.2f}); nothing else holds.", { factor: esc(named.get(f).toLowerCase()), lo, hi });
    } else if (best.size) {
      headline = t("With the other factors equal: {each}.", { each: [...best].map(([g, [v, f]]) => t("for {grader}, <i>{factor}</i> (ρ {rho:.2f})", { grader: esc(g), factor: esc(named.get(f).toLowerCase()), rho: v })).join("; ") });
    } else {
      headline = esc(t("With the other factors equal, no factor of the chain goes with the grades beyond chance."));
    }
    statsSay([esc(t("Each cell asks: does this factor go with a weaker grade when everything else is the same? Coloured: too clear to be chance; only outlined: could be chance.")),
      esc(t("Blue: more of the factor, a weaker grade; gold: a stronger one."))], ["spearman", "partial", "pvalue"]);
    statsFinding("lab", t("What goes with a weaker grade"), headline,
      [stHeat(rows, shown, cells, { low: -0.5, high: 0.5, diverging: true, cw: 84, ch: 50, labelW: 190, tips, headH: 60 })],
      t("Each row is a scholar's grade of the hadith ({scale}: a higher rank is a weaker grade), the last row how far the scholars are apart on it; each column a factor of the hadith's best chain. The figure is the PARTIAL Spearman ρ: the relation left when every other factor is held equal, so a long chain is not credited with what its weak narrator does. Below it, the plain ρ, nothing held equal. Blue: a weaker grade with more of the factor; gold: a stronger one. Outlined: could be chance (BH q ≥ 0.05). It is the rank counterpart of the hadith model, and does not assume its shape.", { scale: stWords().scale }),
      [["outcome", "factor", "rho alone", "rho others equal", "n", "q"], stAdjustedTable(m)], into);
  }
  const fm = { names: factors, cells: m.cells.slice(0, k).map((row) => row.slice(0, k)), partial: m.partial };
  const pairs = (fm.partial && stPairsTable(fm, "partial").length) ? stPairsTable(fm, "partial") : stPairsTable(fm, "cells");
  if (pairs.length) {
    const [a, b, rho] = pairs[0];
    const values = { a: esc((named.get(a) || a).toLowerCase()), b: esc((named.get(b) || b).toLowerCase()), rho };
    const headline = fm.partial ? t("Of the chain's own factors, <i>{a}</i> and <i>{b}</i> are the most bound together (ρ {rho:+.2f}, the others equal).", values)
      : t("Of the chain's own factors, <i>{a}</i> and <i>{b}</i> are the most bound together (ρ {rho:+.2f}).", values);
    statsSay([t("{a} and {b}: ρ {rho:+.2f} — {strength}.", Object.assign({}, values, { strength: statsWord("strength", rho) }))], ["spearman", "partial"]);
    statsFinding("lab", t("The factors with each other"), headline, [statsSwitchable(fm, shown, [84, 46], 190)],
      shown.map((n, i) => `<b>${esc(n)}</b> — ${esc(t(m.explain[i]))}`).join("<br>") + t("<br><br>Factors that rise together share their credit: the hadith model and the first finding hold the others equal for this reason. “The others held equal” shows each pair with the rest removed."),
      [["a", "b", "rho alone", "n", "p", "q"], stPairsTable(fm, "cells")], into);
  }
  const graders = m.names.slice(k).filter((n) => n.startsWith("Grade: "));
  if (graders.length >= 2) {
    const idx = graders.map((g) => m.names.indexOf(g));
    const gm = { names: graders.map(stOutcome), cells: idx.map((i) => idx.map((j) => m.cells[i][j])) };
    const widgets = [statsMatrix(gm, "cells", gm.names, [118, 50], 200)];
    let headline, values;
    if (tests.length) {
      widgets.push(stTable([t("Scholars"), t("Same hadith"), t("First grades higher"), t("First grades lower"), t("Alike overall?")], tests.map((x) => {
        const alike = x.p_holm < ST_ALPHA ? t("no — χ² {chi2:.0f}, {p}", { chi2: x.chi2, p: stQ(x.p_holm).replace("q", "p") }) : t("cannot tell apart");
        const tip = t("Stuart–Maxwell χ² {chi2:.1f} on {df} df; p {p}, Holm-adjusted {p_holm}", { chi2: x.chi2, df: x.df, p: stG(x.p, 2), p_holm: stG(x.p_holm, 2) })
          .replace("{p:.2g}", stG(x.p, 2)).replace("{p_holm:.2g}", stG(x.p_holm, 2));
        return { cells: [`${stShort(x.a)} / ${stShort(x.b)}`, stTh(x.n), stTh(x.a_stronger), stTh(x.a_weaker), alike].map((v) => ({ text: v, tip })) };
      }), { first: true }));
      const far = tests.reduce((a, b) => (Math.abs(b.a_stronger - b.a_weaker) / b.n > Math.abs(a.a_stronger - a.a_weaker) / a.n ? b : a));
      const [hi, lo] = far.a_stronger >= far.a_weaker ? [far.a, far.b] : [far.b, far.a];
      const more = Math.max(far.a_stronger, far.a_weaker), less = Math.min(far.a_stronger, far.a_weaker);
      const differ = tests.filter((x) => x.p_holm < ST_ALPHA).length;
      values = { hi: esc(stShort(hi)), lo: esc(stShort(lo)), more, less };
      headline = differ === tests.length ? t("No two scholars grade alike overall: on the same hadith, {hi} grades higher than {lo} {more:,} times, lower {less:,}.", values)
        : t("{differ} of {pairs} pairs of scholars do not grade alike overall: on the same hadith, {hi} grades higher than {lo} {more:,} times, lower {less:,}.", Object.assign({ differ, pairs: tests.length }, values));
    } else {
      headline = esc(t("How closely the scholars' grades follow each other."));
    }
    statsSay([...(tests.length ? [t("{hi} grades the same hadith higher than {lo} {more:,} times and lower {less:,} times: {hi} is the milder of the two.", values)] : []),
      esc(t("In the matrix, the nearer ρ is to 1, the more the two put the hadith in the same order."))], ["spearman", "stuart"]);
    statsFinding("lab", t("Scholar against scholar"), headline, widgets,
      t("The matrix: Spearman's ρ between two scholars' grades of the same hadith — how far one's order of the hadith follows the other's. The table asks another question: even when they rank the hadith alike, does one give stronger grades overall? Stuart–Maxwell's test compares the two distributions of grades on the same hadith (paired, so the hadith each graded do not differ); “no” means one is milder beyond chance (Holm-adjusted over the pairs). The counts are the hadith the first scholar grades higher or lower."),
      [["book", "a", "b", "hadith", "a grades higher", "a grades lower", "chi2", "df", "p", "p holm"],
        tests.map((x) => [x.book, x.a, x.b, x.n, x.a_stronger, x.a_weaker, Number(x.chi2.toFixed(3)), x.df, x.p, x.p_holm])], into);
  }
}

function statsLabBrant(tests, into) {
  if (!tests.length) return;
  const rows = [], csv = [], broken = [];
  for (const b of tests) {
    const steps = b.thresholds.map((j) => `${stGrade(b.labels[j])} | ${stGrade(b.labels[j + 1])}`);
    const o = b.overall;
    const shown = [[t("all factors together"), null, o], ...b.factors.filter((f) => f.p_holm < ST_ALPHA).map((f) => [stFactor(f.name), f.slopes, f])];
    for (const [name, slopes, x] of shown) {
      const p = x.p_holm ?? x.p;
      const same = p < ST_ALPHA ? t("no — {p}", { p: stQ(p).replace("q", "p") }) : t("yes, within chance");
      const tip = t("Steps: {steps}", { steps: steps.join(" ; ") }) + `\nWald χ² ${x.chi2.toFixed(1)} · df ${x.df} · p ${stG(x.p, 2)}`
        + (b.thresholds.length === b.of ? "" : "\n" + t("{n} step(s) left out: a factor never seen beyond them", { n: b.of - b.thresholds.length }));
      rows.push({ cells: [stShort(b.grader), name, slopes ? slopes.map((s) => Math.exp(s).toFixed(2)).join(" → ") : "", same].map((v) => ({ text: v, tip })) });
    }
    for (const f of b.factors) if (f.p_holm < ST_ALPHA) broken.push([b.grader, f.name]);
    for (const f of b.factors) csv.push([b.book, b.grader, f.name, f.chi2, f.df, f.p, f.p_holm, f.slopes.map((s) => Math.exp(s).toFixed(4)).join(" ")]);
  }
  let headline;
  if (broken.length) {
    const say = (n) => (n.startsWith("Weakest: ") ? t("a weakest narrator who is {level}", { level: t(n.slice(9)).toLowerCase() }) : t(n).toLowerCase());
    const names = [...new Set(broken.map((x) => x[1]))].sort();
    const who = [...new Set(broken.map((x) => stShort(x[0])))].sort();
    const what = names.length <= 3 ? names.map((n) => `<i>${esc(say(n))}</i>`).join(", ") : t("{n} factors (the table)", { n: names.length });
    headline = t("The hadith model gives each factor one weight for every step of the grades; for {who}, {what} weigh differently from step to step.", { who: who.map(esc).join(", "), what });
  } else {
    headline = esc(t("The hadith model's shape holds: each factor weighs the same at every step of the grades."));
  }
  statsSay([esc(broken.length ? t("For these scholars a factor weighs differently at different steps of the grades: the model's single number for it is an average, still right about its direction.")
    : t("Each factor weighs the same at every step of the grades: the model's numbers can be read as they are."))], ["brant", "odds"]);
  statsFinding("lab", t("Does the model's shape hold?"), headline,
    [stTable([t("Scholar"), t("Factor"), t("Odds ratio at each step"), t("Same at every step?")], rows, { first: true, max: 14 * 30 + 34 })],
    t("The hadith model (an ordinal, proportional-odds regression) gives each factor ONE weight for every step of the grades — from sahih to hasan as from hasan to da'if. Brant's test fits each step on its own and asks whether the weights differ beyond chance (Wald χ², Holm-adjusted over the factors). Where they do, the odds ratios at each step show how: a factor that weighs more at the later step decides between hasan and da'if more than between sahih and hasan. The model's odds ratio is then an average of the steps, still telling the direction. A step a factor never reaches cannot be estimated and is left out."),
    [["book", "scholar", "factor", "chi2", "df", "p", "p holm", "odds ratio at each step"], csv], into);
}

function statsLabNarrators(m) {
  const names = m.names, shown = names.map((n) => t(n)), w = stWords();
  const cell = names.includes(w.rank) && names.includes("Times in the chains") ? m.cells[names.indexOf(w.rank)][names.indexOf("Times in the chains")] : null;
  let headline;
  if (cell && cell.rho != null && (cell.q ?? 1) < ST_ALPHA) {
    headline = cell.rho < 0 ? t("The narrators the chains use most are the stronger ones (ρ {rho:+.2f} between {rank} and how often they appear).", { rho: cell.rho, rank: w.rank_shown })
      : t("The narrators the chains use most are the weaker ones (ρ {rho:+.2f} between {rank} and how often they appear).", { rho: cell.rho, rank: w.rank_shown });
    headline = esc(headline);
  } else {
    const [a, b, rho] = stPairsTable(m, "cells")[0];
    headline = t("Among the narrators, {a} and {b} go together most (ρ {rho:+.2f}).", { a: esc(t(a).toLowerCase()), b: esc(t(b).toLowerCase()), rho });
  }
  statsSay([...(cell && cell.rho != null ? [esc(t("ρ {rho:+.2f} between {rank} and how often a narrator appears: {strength}.", { rho: cell.rho, rank: w.rank_shown, strength: statsWord("strength", cell.rho) }))] : []),
    esc(t("Rank runs from the strongest to the weakest, so a minus sign means: the more of the other, the stronger the narrator."))], ["spearman", "partial"]);
  statsFinding("lab", t("The narrators' measures"), headline,
    [stLabel(t("{people:,} narrators identified in the chains; {complete:,} with every measure known (the partial view uses only these).", { people: m.people, complete: m.complete }), "caption"),
      statsSwitchable(m, shown, [84, 46], 180)],
    shown.map((n, i) => `<b>${esc(n)}</b>${m.explain[i] ? " — " + esc(t(m.explain[i])) : ""}`).join("<br>")
      + (stShia() ? t("<br><br>al-Najashi's category runs from trustworthy (1) to weak (4), so a NEGATIVE ρ with it means the more of the other, the stronger the narrator. His Rijal gives no generations (tabaqat), so there is no generation here.")
        : t("<br><br>Rank runs from the Companions (1) to the liars (12), so a NEGATIVE ρ with it means the more of the other, the stronger the narrator. Generation and year of death move together by their nature; held equal, each shows what the other does not.")),
    [["a", "b", "rho alone", "n", "p", "q"], stPairsTable(m, "cells")]);
}

function statsLabBooks(comparisons) {
  const rows = [], csv = [];
  let strongest = null;
  for (const c of comparisons) {
    const kw = c.kruskal;
    if (!kw) continue;
    const far = c.pairs.length ? c.pairs.reduce((a, b) => (Math.abs(b.delta) > Math.abs(a.delta) ? b : a)) : null;
    if (far && far.p_holm < ST_ALPHA && (!strongest || Math.abs(far.delta) > Math.abs(strongest[1].delta))) strongest = [c.measure, far];
    const differ = c.pairs.filter((p) => p.p_holm < ST_ALPHA).length;
    rows.push({ cells: [t(c.measure), `H ${kw.H.toFixed(1)}, ${stQ(kw.p).replace("q", "p")}`, kw.epsilon2.toFixed(3), t("{n} of {total}", { n: differ, total: c.pairs.length }),
      far ? `${stSigned(far.delta, 2)}  (${far.a} / ${far.b})` : "—"] });
    for (const p of c.pairs) csv.push([c.measure, p.a, p.b, p.U, p.z, p.p, p.p_holm, p.delta]);
  }
  if (!rows.length) return;
  const headline = strongest ? t("The books differ most in {measure}: {a} against {b}, Cliff's δ {delta:+.2f}.", { measure: esc(t(strongest[0]).toLowerCase()), a: esc(strongest[1].a), b: esc(strongest[1].b), delta: strongest[1].delta })
    : t("The books' chains cannot be told apart in {measures}.", { measures: comparisons.filter((c) => c.kruskal).map((c) => esc(t(c.measure).toLowerCase())).join(", ") });
  statsSay([...(strongest ? [t("{a} and {b} differ most in {measure}: δ {delta:+.2f}, {words}.", { a: esc(strongest[1].a), b: esc(strongest[1].b), measure: esc(t(strongest[0]).toLowerCase()), delta: strongest[1].delta, words: statsWord("cliff", strongest[1].delta) })] : []),
    esc(t("H and p: do the books differ at all; ε² and δ: by how much."))], ["kruskal", "cliff"]);
  statsFinding("lab", t("Are the books different?"), headline,
    [stTable([t("Measure"), t("Kruskal–Wallis"), t("Effect (ε²)"), t("Pairs that differ"), t("Furthest apart (Cliff's δ)")], rows)],
    t("Kruskal–Wallis asks whether the books' chains could come from one distribution (ε² is its size: 0.01 small, 0.06 medium, 0.14 large). Mann–Whitney compares each pair, Holm-adjusted; Cliff's δ is how often a chain of the first book is above one of the second minus how often it is below (−1 … +1).") + " "
      + (stShia() ? t("Weakest narrator: al-Najashi's category, higher is weaker.") : t("Weakest narrator: Ibn Hajar's rank group, higher is weaker.")),
    [["measure", "a", "b", "U", "z", "p", "p holm", "cliffs delta"], csv]);
}

// ------------------------------------------------------------------ Books

function statsBooks(r) {
  const b = r.books, rows = b.rows, w = stWords();
  const table = stTable([t("Book"), t("Chains"), t("Median length"), t("Names identified"), w.reach, t("Narrators")],
    rows.map((row) => ({ cells: [stBk(row.name), stTh(row.chains), String(row.median_length || "—"), stPct(row.identified), stPct(row.reaching), stTh(row.narrators)] })), { first: true });
  const cats = stGroupCats(r);
  statsSay([esc(t("Each row is one book: how many chains, how long they usually are, how many of their names are identified, how many reach {end}.", { end: w.end }))], ["median", "share"]);
  statsFinding("books", t("The books side by side"), esc(t("Each book's chains and the narrators they rest on.")),
    [table, stStacked(rows.map((row) => [stBk(row.name), row.groups]), cats, 230)],
    t("The table compares the books' chains; the bars, the narrators each book's chains pass through, each counted as often as he appears."),
    [["book", "chains", "median length", "identified", w.reach.toLowerCase(), "narrators", ...cats.map((c) => c[0])],
      rows.map((row) => [row.name, row.chains, row.median_length, Number(row.identified.toFixed(4)), Number(row.reaching.toFixed(4)), row.narrators, ...row.groups])]);
  if (b.names.length > 1) {
    const cells = b.overlap.map((row, i) => row.map((v, j) => (i === j ? [null, []] : [v, [`${(100 * v).toFixed(0)}%`]])));
    statsSay([esc(t("A cell of 40% means four in ten of the narrators of the two books appear in both."))], ["jaccard"]);
    statsFinding("books", t("Shared narrators"), esc(t("How many narrators each pair of books shares.")),
      [stHeat(b.names.map(stBk), b.names.map(stBk), cells, { cw: Math.max(56, Math.min(96, Math.floor(640 / b.names.length))), ch: 46, labelW: 200 })],
      t("The Jaccard share: narrators found in both books' chains, out of those found in either. Books of one school and one period share most."),
      [["", ...b.names], b.names.map((n, i) => [n, ...b.overlap[i].map((v) => Number(v.toFixed(4)))])]);
  }
}

// ------------------------------------------------------------------ Graders

function statsFollowScope() {
  const scope = ST.scope;
  const books = ST.graderBooks;
  const keys = books.map((b) => b[0]);
  const sel = $("st-scope");
  const name = (sel.options[sel.selectedIndex] || {}).text || "";
  let message = null;
  const g = $("st-gbook");
  if ((scope == null || scope === "@shia" || scope === "@all") && books.length) {
    const road = scope === "@shia" ? "shia" : "sunni";
    const mine = books.filter((b) => scope === "@all" || b[3] === road);
    if (!mine.length) message = t("No book of this road has two graders to compare. The books that have: {books}.", { books: books.map((b) => b[1]).join(", ") });
    else if (!mine.some((b) => b[0] === g.value)) g.value = mine[0][0];
  } else if (scope && !keys.includes(scope)) {
    message = t("{name} has no two graders to compare. The books that have: {books}.", { name: name.split(" (")[0].trim(), books: books.map((b) => b[1]).join(", ") || t("none imported yet") });
  } else if (!books.length) {
    message = t("No grades yet: import a collection with graded hadith (Data Sources).");
  }
  for (const id of ["st-gbook", "st-gtitle", "st-grecompute"]) $(id).hidden = message != null;
  if (message) {
    statsBox("graders").innerHTML = stLabel(message);
    $("st-gstatus").textContent = "";
    return;
  }
  if (scope && scope !== "@shia" && scope !== "@all" && g.value !== scope) g.value = scope;
  ST.gbook = g.value;
  statsGraders(false);
}

async function statsGraders(recompute) {
  const key = $("st-gbook").value;
  if (!key) return;
  if (ST.gbusy) { ST.gagain = true; return; }
  ST.gbusy = true;
  ST.gagain = false;
  $("st-gstatus").textContent = recompute ? t("Computing…") : t("Reading the saved results…");
  $("st-grecompute").disabled = true;
  statsPoll(true, "st-gstatus");
  try {
    const r = await invoke("stats_graders", { key, recompute });
    if (key !== $("st-gbook").value) ST.gagain = true;
    else {
      const gs = new Set([...(r.graders || []), ...Object.keys(r.too_few || {})]);
      for (const s of r.selections || []) { gs.add(s.grader); for (const o of Object.keys(s.others)) gs.add(o); }
      const got = await invoke("stats_names", { graders: [...gs], books: [], people: [] });
      Object.assign(ST.names.short, got.short);
      Object.assign(ST.names.full, got.full);
      ST.gresult = r;
      statsShowGraders(r);
    }
  } catch (e) {
    $("st-gstatus").textContent = t("Could not compute: {error}", { error: String(e) });
  } finally {
    statsPoll(false);
    ST.gbusy = false;
    $("st-grecompute").disabled = false;
    if (ST.gagain) { ST.gagain = false; statsGraders(false); }
  }
}

function stCard(title, body) {
  return `<div class="st-card st-gcard" data-name="${esc(title)}">${title ? `<div class="st-ctitle">${esc(title)}</div>` : ""}${body}</div>`;
}
function stHow(text) { return `<div class="caption">${t("<b>How to read it</b> — ")}${text}</div>`; }

function statsShowGraders(r) {
  const box = statsBox("graders");
  box.innerHTML = "";
  const graders = r.graders;
  const names = graders.map(stShort);
  const labels = (r.labels && r.labels.length ? r.labels : ["fabricated", "very weak", "da'if", "hasan", "sahih"]).map((x) => t(x));
  const order = [...(r.categories && r.categories.length ? r.categories : [0, 1, 2, 3, 4])].reverse();
  const shia = r.tradition === "shia";
  $("st-gstatus").textContent = (r.from_file ? t("From the saved results ({file}).", { file: r.from_file }) : t("Computed and saved.")) + " "
    + t("Intervals: {boot} bootstrap samples of hadith.", { boot: r.boot });
  let html = "";
  if (shia) html += stLabel(t("Road B: the graders of a Shia book compared on the four Imami categories — sahih, hasan, muwaththaq, da'if — in their own order, never on the Sunni grade groups."), "caption");
  const few = r.too_few || {};
  if (Object.keys(few).length) html += stLabel(t("Named, not measured — too few grades of this book to compare: {graders}.", { graders: Object.entries(few).map(([g, n]) => `${stShort(g)} (${n})`).join(", ") }), "caption");
  for (const sel of r.selections || []) html += statsSelection(sel, labels, order);
  if (graders.length < 2) {
    if (!(r.selections || []).length) {
      const dist = graders.length ? (r.distribution[graders[0]] || []) : [];
      html += stCard(t("One grader only"), stLabel(t("{grader} graded this book; agreement and the model need two or more. His grades:", { grader: names[0] || "—" }))
        + stLabel(order.filter((k) => k < dist.length).map((k) => `${labels[k]} ${stTh(dist[k])}`).join(" · ")));
    } else if (graders.length) {
      html += stCard(t("No second grader to measure against"), stLabel(t("Besides the selection above, only {grader} grades this book in more than one grade, so agreement beyond chance, strictness and the model of the true grade (which need two graders who could each have said otherwise) cannot be measured here. His grades: {grades}.",
        { grader: names[0], grades: order.map((k) => `${labels[k]} ${stTh(r.distribution[graders[0]][k])}`).join(" · ") })));
    }
    box.innerHTML = html;
    return;
  }
  const a = r.alpha, c = r.certainty;
  let body = stLabel(t("<b>Krippendorff's alpha {alpha:.3f}</b> (95% interval {lo:.3f}–{hi:.3f}) over {units:,} hadith graded by two or more — {words}.<br>The model is sure (≥95%) of the true grade of <b>{high:,}</b> hadith ({high_share:.1f}%), fairly sure of {middle:,}, and unsure (<60%) of <b>{low:,}</b> ({low_share:.1f}%).",
    { alpha: a.value, lo: a.ci[0], hi: a.ci[1], units: a.units, words: statsWord("alpha", a.value), high: c.high, high_share: 100 * c.high / c.total, middle: c.middle, low: c.low, low_share: 100 * c.low / c.total }), "lead", true);
  const left = Object.entries(r.chain_only || {}).filter(([, n]) => n);
  if (left.length) {
    body += stLabel(t("Grades of the <b>chain only</b> (\"Isnaad Sahih\": the chain is sound, the text is not judged) answer another question than a grade of the hadith, so they are left out of every comparison here: {graders}.",
      { graders: left.map((x, i) => [x, i]).sort((p, q) => q[0][1] - p[0][1] || p[1] - q[1]).map(([[g, n]]) => `${esc(stShort(g))} ${stTh(n)}`).join(", ") }), "caption", true);
  }
  for (const ov of r.one_voice || []) {
    body += `<div class="caption st-warn">${esc(t("⚠ {kept} and {left_out} agree on {value:.1f}% — too often for two scholars grading on their own; the source probably filled one column from the other. In the model they are one voice ({kept}); {left_out} is measured against the model's result without moving it.",
      { kept: stShort(ov.kept), left_out: stShort(ov.left_out), value: 100 * ov.agree }))}</div>`;
  }
  body += statsPlain([esc(t("α {a:.2f}: {words}.", { a: a.value, words: statsWord("alpha", a.value) })),
    esc(t("The model is sure of the grade of {high:.0f}% of the hadith; for {low:.0f}% the scholars' grades point different ways — there ijtihad differs.", { high: 100 * c.high / c.total, low: 100 * c.low / c.total }))], ["alpha", "dawid", "interval"]);
  body += stHow(t("alpha measures agreement beyond chance for all graders at once, by the order of the grades (a near miss counts less than a far one): 1 is complete agreement, 0 no better than chance; above 0.8 is strong, 0.67–0.8 allows tentative conclusions, below 0.67 is weak."));
  html += stCard(t("All graders together"), body);
  const pairOf = (ga, gb) => r.pairs[`${ga}|${gb}`] || r.pairs[`${gb}|${ga}`];
  const matrix = (field, fmt, ciFmt) => graders.map((ga, i) => graders.map((gb, j) => {
    if (i === j) return [null, []];
    const p = pairOf(ga, gb);
    if (!p) return [null, []];
    const [lo, hi] = p.ci[field];
    return [p[field], [fmt(p[field]), ciFmt(lo, hi) + ` · n ${stTh(p.common)}`]];
  }));
  body = stLabel(t("<b>Same grade</b>"), "lead", true)
    + stHeat(names, names, matrix("agree", (v) => (100 * v).toFixed(1) + "%", (x, y) => `${(100 * x).toFixed(1)}–${(100 * y).toFixed(1)}`), { low: 0.5, high: 1, labelW: 210 })
    + stLabel(t("<b>Ordinal (weighted) kappa</b>"), "lead", true)
    + stHeat(names, names, matrix("wkappa", (v) => `κw ${v.toFixed(2)}`, (x, y) => `${x.toFixed(2)}–${y.toFixed(2)}`), { low: 0, high: 1, labelW: 210 })
    + statsPlain([esc(t("Each cell is two scholars on the hadith both graded: above, how often they give the very same grade; below, their agreement once the agreement chance would give is taken away."))], ["kappa", "interval"])
    + stHow(t("each cell compares the row's grader with the column's on the hadith both graded (n). Left: how often they give the same grade. Right: Cohen's kappa weighted by order — agreement beyond chance, where calling a hasan hadith da'if is a smaller disagreement than calling it fabricated. The small figures are 95% intervals."));
  html += stCard(t("Agreement between each pair of graders"), body);
  const rows = names.map((n, i) => [n, r.model.strictness[graders[i]].value, r.model.strictness[graders[i]].ci]).map((x, i) => [x, i]).sort((p, q) => p[0][1] - q[0][1] || p[1] - q[1]).map((x) => x[0]);
  const pairCells = graders.map((ga, i) => graders.map((gb, j) => {
    let p = r.pairs[`${ga}|${gb}`], sign = 1;
    if (!p) { p = r.pairs[`${gb}|${ga}`]; sign = -1; }
    if (i === j || !p) return [null, []];
    const v = -sign * p.order;
    const [lo, hi] = [-sign * p.ci.order[0], -sign * p.ci.order[1]].sort((x, y) => x - y);
    return [v, [stSigned(v, 3), `${stSigned(lo, 3)} … ${stSigned(hi, 3)}`]];
  }));
  body = stInterval(rows, t("stricter (mutashaddid)"), t("more lenient (mutasahil)"))
    + stLabel(t("<b>Pair by pair</b>: P(row grades higher) − P(row grades lower)"), "lead", true)
    + stHeat(names, names, pairCells, { low: -0.25, high: 0.25, diverging: true, labelW: 210 })
    + statsPlain([t("{strict} is the strictest ({v1:+.3f}), {lenient} the most lenient ({v2:+.3f}).", { strict: esc(rows[0][0]), v1: rows[0][1], lenient: esc(rows[rows.length - 1][0]), v2: rows[rows.length - 1][1] }),
      esc(t("For the science of hadith: the classical labels mutashaddid (strict) and mutasahil (lenient), put in numbers on the very same hadith."))], ["strictness", "interval"])
    + stHow(t("from the model: the chance a grader gives a hadith a LOWER grade than its true grade, minus the chance he gives it a HIGHER one — by order only. Below zero (gold) he is stricter, above zero (blue) more lenient; the line is the 95% interval. The 'true grade' is what these graders' grades imply together, so the measure is relative to them, not absolute. Pair by pair needs no model: who grades the same hadith higher, who lower."));
  html += stCard(t("Strictness — mutashaddid and mutasahil, measured"), body);
  const fewAll = new Set(order.filter((k) => r.model.prior[k] * c.total < 15));
  const rowsK = order.filter((k) => !fewAll.has(k));
  body = `<div class="st-cgrid2">` + graders.map((g, n) => {
    const th = r.model.theta[g];
    const name = names[n];
    const cells = rowsK.map((k) => order.map((l) => [th[k][l], [`${(100 * th[k][l]).toFixed(0)}%`]]));
    const tips = rowsK.map((k) => order.map((l) => t("A hadith whose true grade is {true}: {grader} calls it {said} in {share} of cases.", { true: labels[k], grader: name, said: labels[l], share: `${(100 * th[k][l]).toFixed(0)}%` })));
    return `<div>${stLabel(`<b>${esc(name)}</b>`, "lead", true)}${stHeat(rowsK.map((k) => labels[k]), order.map((k) => labels[k]), cells, { cw: 78, ch: 40, labelW: 118, headH: 52, corner: t("true grade ↓\nthe grader says →"), tips })}</div>`;
  }).join("") + "</div>";
  const prior = r.model.prior;
  body += stLabel(t("How common each true grade is, by the model: {shares}.", { shares: order.map((k) => `${labels[k]} ${(100 * prior[k]).toFixed(1)}%`).join(" · ") })
    + (fewAll.size ? " " + t("No row for {grades}: too few hadith of that true grade to measure how a grader treats it.", { grades: order.filter((k) => fewAll.has(k)).map((k) => labels[k]).join(", ") }) : ""));
  body += statsPlain([esc(t("Read a row: for a hadith whose grade (by all the graders together) is this, how often each grader gives each grade. Much outside the diagonal is a grader who sees it otherwise."))], ["dawid"]);
  body += stHow(t("rows are the true grade, columns what the grader says. A strong diagonal is a grader who sees the grade the others see; weight to the right of it (towards weaker grades) is strictness, to the left leniency. This is the Dawid–Skene model: it learns each grader's matrix and the true grades together."));
  html += stCard(t("How each grader grades a hadith of each true grade (the model's confusion matrices)"), body);
  html += statsDisputed(r, graders, labels);
  box.innerHTML = html;
}

function statsSelection(sel, labels, order) {
  const who = stShort(sel.grader), grade = labels[sel.grade];
  let body = "";
  const others = Object.entries(sel.others);
  if (!others.length) body += stLabel(t("No other grader graded the hadith {who} selected.", { who }));
  const colors = (order.length === 4 ? ST_IMAMI_COLORS : ST_GRADE_COLORS)[!!DARK];
  for (const [o, x] of others) {
    const dist = x.distribution;
    const same = x.common ? dist[sel.grade] / x.common : 0;
    body += stLabel(t("Of the <b>{common:,}</b> hadith {who} holds {grade} that {o} grades too, {o} calls <b>{same}</b> {grade}.", { common: x.common, who: esc(who), grade: esc(grade), o: esc(stShort(o)), same: stPct(same) }), "lead", true);
    body += stStacked([[t("{other} on {who}'s {grade}", { other: stShort(o), who, grade }), order.map((k) => dist[k])]], order.map((k, i) => [labels[k], colors[i]]), 260);
  }
  body += stHow(esc(t("{who} names only the hadith he holds {grade}: a selection, not a grading of every hadith, so he never says anything else and agreement beyond chance (kappa, alpha) or a strictness cannot be measured for him. What can be measured is how the other graders grade the hadith he selected. The two need not differ in method only: they may read a narrator differently, or grade different versions of the chain.", { who, grade })));
  return stCard(t("{who}'s selection — {count:,} hadith, all {grade}", { who, count: sel.count, grade }), body);
}

function statsDisputed(r, graders, labels) {
  const voices = new Map((r.one_voice || []).map((o) => [o.left_out, o.kept]));
  const heads = [t("Hadith"), t("Model's grade"), [t("Certainty"), t("The model's probability for its grade")], t("Voices weighed"),
    ...graders.map((g) => [stShort(g) + (voices.has(g) ? " ≈" : ""), stFull(g) + (voices.has(g) ? "\n" + t("≈ one voice with {other} in the model (they agree too often to count twice): his grade is shown, but does not move the model's grade", { other: stShort(voices.get(g)) }) : "")])];
  const rows = r.disputed.slice(0, 150).map((d) => {
    const given = Object.fromEntries(graders.map((g) => [g, d.grades[g] ?? null]));
    const voted = graders.filter((g) => given[g] != null && !voices.has(g));
    const why = voted.map((g) => `${stShort(g)} ${labels[given[g]]}`).join(", ");
    const tip = t("The voices the model weighs on this hadith: {why}.", { why }) + "\n"
      + (voted.length === 2 && new Set(voted.map((g) => given[g])).size === 2 ? t("One voice against one: the model sides with the grader it finds closer to the others over the whole book, so its certainty is modest.")
        : t("The model's grade is the grade these voices most probably mean, given how each grades the whole book."));
    const tally = new Map();
    for (const g of voted) tally.set(given[g], (tally.get(given[g]) || 0) + 1);
    const tallyText = [...tally].sort((a, b) => b[0] - a[0]).map(([k, n]) => `${labels[k]} ${n}`).join(" · ");
    return { attrs: ` data-hadith="${d.hadith_id}"`, cells: [String(d.number || d.hadith_id), { text: labels[d.consensus], tip }, { text: `${(100 * d.certainty).toFixed(0)}%`, tip }, { text: tallyText, tip },
      ...graders.map((g) => {
        const v = given[g];
        if (v == null) return { text: t("not graded"), muted: true, tip: t("{grader} gives no grade of this hadith (or a grade of its chain only)", { grader: stShort(g) }) };
        if (voices.has(g)) return { text: labels[v], muted: true, tip: t("One voice with {other}: shown, not weighed", { other: stShort(voices.get(g)) }) };
        return labels[v];
      })] };
  });
  const same = [...voices].map(([k, v]) => `${stShort(k)} ≈ ${stShort(v)}`);
  const body = stTable(heads, rows, { stretch: false, max: 360 })
    + statsPlain([esc(t("{n:,} hadith where the scholars are two grades apart or the model is unsure — the places where their ijtihad differs most.", { n: r.disputed_total }))], ["dawid"])
    + stHow(t("the hadith where the scholars are furthest apart come first. <i>Certainty</i> is the model's probability for its grade: where only two voices weigh and they disagree, it sits a little above one half — the model sides with the grader it finds closer to the others over the whole book, which is why many such rows show the same figure. Near 100% against a dissent means the dissenting grade is one that grader gives, over the whole book, to hadith the others place elsewhere — the model holds it to be his own reading, not the grade. <i>Voices weighed</i>: the grades the model counts. <i>not graded</i>: that scholar gave no grade of the hadith (or graded its chain only).") + " "
      + (same.length ? t("≈ marks a grader who is one voice with another in the model ({same}): his grade is shown, muted, but not weighed.", { same: same.join(", ") }) + " " : "")
      + t("Hover a cell for the details; double-click a row to read the hadith in its book."));
  return stCard(t("The disputed hadith — {disputed_total:,} where the graders are two steps apart or the model is unsure", { disputed_total: r.disputed_total }), body);
}

// ------------------------------------------------------------------ export (stats_export.py)

function statsAskPath(title, file, done) {
  invoke("stats_home").then((home) => {
    const sep = home.includes("\\") ? "\\" : "/";
    const back = document.createElement("div");
    back.className = "dialog-back";
    back.innerHTML = `<div class="dialog small" role="dialog"><h2>${esc(title)}</h2><div class="dialog-body">
      <input type="text" class="st-path" spellcheck="false" value="${esc(home + sep + file)}"></div>
      <div class="dialog-buttons"><button class="primary" data-a="ok">${esc(t("Save"))}</button> <button class="quiet" data-a="no">${esc(t("Cancel"))}</button></div></div>`;
    const input = back.querySelector("input");
    const go = () => { const p = input.value.trim(); back.remove(); if (p) done(p); };
    back.onclick = (e) => {
      const b = e.target.closest("[data-a]");
      if (!b) return;
      if (b.dataset.a === "ok") go(); else back.remove();
    };
    input.onkeydown = (e) => { if (e.key === "Enter") go(); if (e.key === "Escape") back.remove(); };
    document.body.appendChild(back);
    input.focus();
    input.setSelectionRange(input.value.lastIndexOf(sep) + 1, input.value.length);
  });
}

function statsSaid(p) {
  $("st-status").textContent = t("Written: {path}", { path: p });
}

function statsFail(e) { infoDialog(t("Export"), `<p>${esc(String(e))}</p>`); }

function statsExportMenu(button) {
  closeMenus();
  const r = button.getBoundingClientRect();
  const items = [
    act("This section as PDF…", () => statsExportPage("section", "pdf")),
    act("This section as a web page (HTML)…", () => statsExportPage("section", "html")),
    act("This section as one picture (PNG)…", () => statsPicture("png", t(ST_SECTIONS[ST.section][1]), t("isnady statistics — {scope}", { scope: statsScopeName() }) + " — " + t(ST_SECTIONS[ST.section][1]), statsSectionCards("section"))),
    SEP,
    act("Every section as PDF…", () => statsExportPage("all", "pdf")),
    act("Every section as a web page (HTML)…", () => statsExportPage("all", "html")),
    SEP,
    act("Every table as Excel (a sheet each)…", () => statsAskPath(t("Export"), "isnady-" + stSafe(statsScopeName()) + ".xlsx", (path) =>
      invoke("stats_workbook", { path, tables: statsTables() }).then(statsSaid, statsFail))),
    act("Every table as CSV files (a folder)…", () => statsExportPage("all", "csv")),
    act("The computed results (JSON)…", () => statsExportPage("all", "json")),
  ];
  openMenu(() => items, { x: isRtl() ? r.right : r.left, y: r.bottom, top: r.top, flipUp: true, alignEnd: isRtl() });
}

function statsArm(card, button) {
  closeMenus();
  const r = button.getBoundingClientRect();
  const name = card.dataset.name;
  const tb = ST.tables.get(card.dataset.card);
  const items = [
    act("Picture (PNG)…", () => statsPicture("png", name, "", [["", [card]]])),
    act("Vector picture (SVG)…", () => statsPicture("svg", name, "", [["", [card]]])),
    act("PDF…", () => statsAskPath(t("Export"), `isnady-${stSafe(name)}.pdf`, (path) => statsPdf(path, name, [["", [card]]], "finding, pdf"))),
    act("Web page (HTML)…", () => statsAskPath(t("Export"), `isnady-${stSafe(name)}.html`, (path) =>
      invoke("stats_write", { path, text: statsHtml(name, [["", [card]]]), what: "finding, html" }).then(statsSaid, statsFail))),
  ];
  if (tb) {
    items.push(SEP, act("Figures (CSV)…", () => statsAskPath(t("Export"), `isnady-${stSafe(name)}.csv`, (path) =>
      invoke("stats_csv", { path, header: tb.header, rows: tb.rows }).then(statsSaid, statsFail))));
    items.push(act("Figures (Excel)…", () => statsAskPath(t("Export"), `isnady-${stSafe(name)}.xlsx`, (path) =>
      invoke("stats_xlsx", { path, name, header: tb.header, rows: tb.rows }).then(statsSaid, statsFail))));
  }
  openMenu(() => items, { x: isRtl() ? r.left : r.right - 220, y: r.bottom, top: r.top, flipUp: true, alignEnd: isRtl() });
}

function statsScopeName() {
  const sel = $("st-scope");
  return (sel.options[sel.selectedIndex] || {}).text || t("Statistics");
}

function statsSectionCards(which) {
  const out = [];
  ST_SECTIONS.forEach(([k, n], i) => {
    if (which === "section" && i !== ST.section) return;
    const section = $("st-stack").children[i];
    const cards = [...section.querySelectorAll(".st-card")];
    if (cards.length) out.push([t(n), cards]);
  });
  return out;
}

// one page: each section's heading, each finding as it is drawn here (its charts stay charts), its figures under it
function statsHtml(title, sections) {
  const css = [...document.styleSheets].map((s) => { try { return [...s.cssRules].map((r) => r.cssText).join("\n"); } catch (_) { return ""; } }).join("\n");
  const vars = getComputedStyle(document.documentElement);
  const root = ["window", "surface", "border", "ink", "muted", "lapis", "lapis-soft", "gold", "ui", "ui-size", "heading", "scale"]
    .map((k) => `--${k}:${vars.getPropertyValue("--" + k)}`).join(";");
  let body = `<h1>${esc(title)}</h1><p class="caption">isnady-rs ${esc(APP.version)} · ${esc(new Date().toLocaleString())}</p>`;
  for (const [heading, cards] of sections) {
    if (heading) body += `<h2>${esc(heading)}</h2>`;
    for (const card of cards) {
      const copy = card.cloneNode(true);
      for (const x of copy.querySelectorAll(".st-arm, .st-foot")) x.remove();
      for (const x of copy.querySelectorAll(".st-how")) x.hidden = false;
      body += copy.outerHTML;
      const tb = ST.tables.get(card.dataset.card);
      if (tb) {
        body += `<details><summary>${esc(t("The figures"))} — ${esc(card.dataset.name)}</summary><table class="figs"><tr>${tb.header.map((h) => `<th>${esc(h)}</th>`).join("")}</tr>`
          + tb.rows.map((row) => `<tr>${row.map((v) => `<td>${esc(v == null ? "" : v)}</td>`).join("")}</tr>`).join("") + "</table></details>";
      }
    }
  }
  return `<!doctype html><html><head><meta charset="utf-8"><title>${esc(title)}</title><style>${css}
:root{${root}} body{overflow:auto;display:block;max-width:1100px;margin:2em auto;padding:0 1em} h1{font-size:1.6em} h2{font-size:1.3em;margin-top:2em;border-bottom:1px solid var(--border)}
.st-card{margin-bottom:16px} table.figs{border-collapse:collapse;font-size:.85em;margin:.4em 0 1.6em} table.figs td,table.figs th{border:1px solid var(--border);padding:3px 8px} details{margin-bottom:1.4em}</style></head><body>${body}</body></html>`;
}

// A finding (or a section) as a picture (stats_export.save_png, save_svg): the cards as the page draws them, with
// the page's styles and the typefaces it uses, in an SVG (vector, readable in any browser); PNG is that SVG drawn on
// a canvas at twice the size.
const ST_FONTS = new Map();

async function stFontData(url) {
  if (!ST_FONTS.has(url)) {
    ST_FONTS.set(url, fetch(url).then((r) => r.blob()).then((b) => new Promise((ok) => {
      const fr = new FileReader();
      fr.onload = () => ok(fr.result);
      fr.readAsDataURL(b);
    })).catch(() => url));
  }
  return ST_FONTS.get(url);
}

async function statsSvg(title, sections) {
  const page = getComputedStyle(document.body);
  const used = (page.fontFamily + "," + getComputedStyle(document.documentElement).getPropertyValue("--heading") + ",Amiri").toLowerCase();
  let css = [...document.styleSheets].map((s) => { try { return [...s.cssRules].map((r) => r.cssText).join("\n"); } catch (_) { return ""; } }).join("\n");
  // the typefaces the cards are drawn in go inside the picture; the others are left out
  const faces = [...css.matchAll(/@font-face\s*{[^}]*}/g)].map((m) => m[0]);
  for (const face of faces) {
    const family = (face.match(/font-family:\s*"?([^";]+)"?/) || [])[1] || "";
    const src = (face.match(/url\("?([^")]+)"?\)/) || [])[1];
    if (!src || !used.includes(family.toLowerCase())) { css = css.replace(face, ""); continue; }
    css = css.replace(src, await stFontData(new URL(src, location.href).href));
  }
  const vars = getComputedStyle(document.documentElement);
  const root = ["window", "surface", "border", "ink", "muted", "lapis", "lapis-soft", "gold", "ui", "ui-size", "heading", "scale"]
    .map((k) => `--${k}:${vars.getPropertyValue("--" + k)}`).join(";");
  const width = Math.max(...sections.flatMap(([, cards]) => cards.map((c) => c.getBoundingClientRect().width)), 400);
  const box = document.createElement("div");
  box.style.cssText = `position:fixed;left:-20000px;top:0;width:${width}px;padding:16px;box-sizing:content-box;background:var(--window);color:var(--ink);font-family:${page.fontFamily}`;
  box.innerHTML = title ? `<div style="font-weight:700;font-size:1.2em;margin:0 0 10px">${esc(title)}</div>` : "";
  for (const [heading, cards] of sections) {
    if (heading) box.insertAdjacentHTML("beforeend", `<div style="font-weight:700;margin:14px 0 6px">${esc(heading)}</div>`);
    for (const card of cards) {
      const copy = card.cloneNode(true);
      for (const x of copy.querySelectorAll(".st-arm, .st-foot")) x.remove();
      copy.style.marginBottom = "12px";
      box.appendChild(copy);
    }
  }
  document.body.appendChild(box);
  const w = Math.ceil(box.getBoundingClientRect().width), h = Math.ceil(box.getBoundingClientRect().height);
  box.style.cssText = `width:${width}px;padding:16px;background:${vars.getPropertyValue("--window")};color:${vars.getPropertyValue("--ink")};font-family:${page.fontFamily};${root}`;
  const style = document.createElement("style");
  style.textContent = css;
  box.insertBefore(style, box.firstChild);
  const xhtml = new XMLSerializer().serializeToString(box);
  box.remove();
  return { svg: `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}"><foreignObject x="0" y="0" width="${w}" height="${h}">${xhtml}</foreignObject></svg>`, w, h };
}

async function statsPng(title, sections) {
  const { svg, w, h } = await statsSvg(title, sections);
  const img = new Image();
  img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
  await img.decode();
  const canvas = document.createElement("canvas");
  canvas.width = w * 2;
  canvas.height = h * 2;
  const g = canvas.getContext("2d");
  g.scale(2, 2);
  g.drawImage(img, 0, 0);
  return canvas.toDataURL("image/png");
}

// Findings as PDF (stats_export.save_pdf): each drawn as the page draws it, at twice the size, as a JPEG picture
async function statsPdf(path, title, sections, what) {
  document.body.classList.add("busy");
  try {
    const parts = [];
    for (const [heading, cards] of sections) {
      const pictures = [];
      for (const card of cards) {
        const { svg, w, h } = await statsSvg("", [["", [card]]]);
        const img = new Image();
        img.src = "data:image/svg+xml;charset=utf-8," + encodeURIComponent(svg);
        await img.decode();
        const canvas = document.createElement("canvas");
        canvas.width = w * 2;
        canvas.height = h * 2;
        const g = canvas.getContext("2d");
        g.fillStyle = "#ffffff";
        g.fillRect(0, 0, canvas.width, canvas.height);
        g.scale(2, 2);
        g.drawImage(img, 0, 0);
        pictures.push([canvas.toDataURL("image/jpeg", 0.92), w * 0.75]);
      }
      parts.push([heading, pictures]);
    }
    statsSaid(await invoke("stats_pdf", { path, title, parts, what }));
  } catch (e) { statsFail(e); } finally { document.body.classList.remove("busy"); }
}

function statsPicture(kind, name, title, sections) {
  statsAskPath(t("Export"), `isnady-${stSafe(name)}.${kind}`, async (path) => {
    document.body.classList.add("busy");
    try {
      const args = kind === "png" ? { png: await statsPng(title, sections) } : { svg: (await statsSvg(title, sections)).svg };
      statsSaid(await invoke("stats_picture", { path, ...args, what: kind }));
    } catch (e) { statsFail(e); } finally { document.body.classList.remove("busy"); }
  });
}

function statsTables() {
  const tables = [];
  for (const [, cards] of statsSectionCards("all")) {
    for (const card of cards) {
      const tb = ST.tables.get(card.dataset.card);
      if (tb) tables.push([card.dataset.name, tb.header, tb.rows]);
    }
  }
  return tables;
}

function statsExportPage(which, fmt) {
  const scope = statsScopeName();
  let title = t("isnady statistics — {scope}", { scope });
  if (which === "section") title += " — " + t(ST_SECTIONS[ST.section][1]);
  const base = "isnady-" + stSafe(title.replace("isnady", ""));
  if (fmt === "html") {
    statsAskPath(t("Export"), base + ".html", (path) =>
      invoke("stats_write", { path, text: statsHtml(title, statsSectionCards(which)), what: `${which}, html` }).then(statsSaid, statsFail));
  } else if (fmt === "csv") {
    statsAskPath(t("Export: the folder for the files"), base, (folder) => {
      const tables = [];
      for (const [, cards] of statsSectionCards("all")) {
        for (const card of cards) {
          const tb = ST.tables.get(card.dataset.card);
          if (tb) tables.push([card.dataset.name, tb.header, tb.rows]);
        }
      }
      invoke("stats_csv_folder", { folder, tables }).then(statsSaid, statsFail);
    });
  } else if (fmt === "pdf") {
    statsAskPath(t("Export"), base + ".pdf", (path) => statsPdf(path, title, statsSectionCards(which), `${which}, pdf`));
  } else {
    statsAskPath(t("Export"), base + ".json", (path) =>
      invoke("stats_json", { path, results: { scope, corpus: ST.result, graders: ST.gresult } }).then(statsSaid, statsFail));
  }
}
