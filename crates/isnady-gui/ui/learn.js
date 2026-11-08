"use strict";
// Learn (gui/learn_page.py, G7, UI8, LRN-H1): the terms of hadith and its sciences, in colour — each category with its
// own colour and sign (the chips, the dot of each term, the band of its card, the box of its short definition, its
// related terms) — and History and method: the articles, with blocks of numbers counted live from this database.
//
// Learn · {n} terms                 | ━━ ✔ Grades ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// [📜 History and method]           |  sahih                               صحيح  ☆
// [English] [Türkçe] [العربية]      |  Türkçe: sahîh
// [Search …]                        |  ┃ IN SHORT  the one sentence
// (All) (✔ Grades) (⛓ …)            |  IN DETAIL  the paragraph · 📜 Source
// the tree: categories and terms    | Related terms · More in Grades

const LN = { built: false, lang: null, category: null, current: null, list: null, blocks: {}, seq: 0 };
const LN_STYLE = {
  grades: ["#2E7D5B", "✔"], chain_kinds: ["#1D63A8", "⛓"], chain: ["#6A4C93", "⟷"], transmission: ["#0F7C8C", "🗣"],
  critics: ["#A23B54", "⚖"], generations: ["#8A5A1E", "⌛"], names: ["#B5651D", "✎"], shia: ["#7A6A12", "☀"],
  books: ["#4E6A86", "📚"], measures: ["#C0392B", "📊"], history: ["#8B4A2B", "📜"],
};
const LN_ARTICLE = "article:";

function lnStyle(cat) {
  return LN_STYLE[cat] || ["#5B6878", "•"];
}

function hexRgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

// a light (or, in a dark theme, deep) tint of a colour, for backgrounds
function lnSoft(colour, amount) {
  const a = (amount || 0.13) * (DARK ? 1.6 : 1);
  const base = DARK ? [0x18, 0x20, 0x2a] : [255, 255, 255];
  const c = hexRgb(colour);
  return `rgb(${base.map((b, i) => Math.round(b + (c[i] - b) * a)).join(",")})`;
}

// QColor.lighter(150) for a dark theme
function lnBright(colour) {
  if (!DARK) return colour;
  const [r, g, b] = hexRgb(colour);
  const max = Math.max(r, g, b) / 255, min = Math.min(r, g, b) / 255;
  let h = 0;
  const d = max - min;
  if (d) {
    if (max === r / 255) h = ((g - b) / 255 / d) % 6;
    else if (max === g / 255) h = (b - r) / 255 / d + 2;
    else h = (r - g) / 255 / d + 4;
  }
  h = (h * 60 + 360) % 360;
  const s = max ? d / max : 0;
  let v = Math.min(1, max * 1.5);
  const s2 = max * 1.5 > 1 ? Math.max(0, s - (max * 1.5 - 1)) : s;
  const c = v * s2, x = c * (1 - Math.abs(((h / 60) % 2) - 1)), m = v - c;
  const [rr, gg, bb] = h < 60 ? [c, x, 0] : h < 120 ? [x, c, 0] : h < 180 ? [0, c, x] : h < 240 ? [0, x, c] : h < 300 ? [x, 0, c] : [c, 0, x];
  return `rgb(${[rr, gg, bb].map((q) => Math.round((q + m) * 255)).join(",")})`;
}

function lnChipStyle(colour, on) {
  return on
    ? `background:${colour};color:white;border:1px solid ${colour};font-weight:600`
    : `background:${lnSoft(colour)};color:${lnBright(colour)};border:1px solid ${lnSoft(colour, 0.35)}`;
}

function learnBuild() {
  if (LN.built) return;
  LN.built = true;
  const page = $("page-learn");
  page.className = "page narrators-page learn-page";
  page.innerHTML = `<div class="np">
    <div class="np-left ln-left">
      <div class="ln-title">${esc(t("Learn"))}</div>
      <div class="caption" id="ln-about"></div>
      <button class="primary" id="ln-history"></button>
      <div class="np-segs" id="ln-langs"></div>
      <input id="ln-q" type="search" autocomplete="off" spellcheck="false" placeholder="${esc(t("Search: sahih, mursal, tadlis, künye …"))}">
      <div class="ln-chips" id="ln-chips"></div>
      <div class="np-list ln-tree" id="ln-tree"></div>
    </div>
    <div class="np-right ln-right" id="ln-view"></div>
  </div>`;
  $("ln-history").onclick = () => pickTerm(LN_ARTICLE + "tadwin");
  $("ln-langs").onclick = (e) => {
    const b = e.target.closest("[data-lang]");
    if (!b) return;
    LN.lang = b.dataset.lang;
    invoke("learn_set_language", { lang: LN.lang });
    learnRefresh();
  };
  let typing = null;
  $("ln-q").addEventListener("input", () => {
    clearTimeout(typing);
    typing = setTimeout(learnList, 150);
  });
  $("ln-chips").onclick = (e) => {
    const c = e.target.closest("[data-cat]");
    if (!c) return;
    const key = c.dataset.cat || null;
    LN.category = LN.category === key ? null : key;
    learnList();
  };
  $("ln-tree").onclick = (e) => {
    const r = e.target.closest("[data-term]");
    if (r) pickTerm(r.dataset.term);
  };
  $("ln-tree").addEventListener("contextmenu", (e) => {
    const r = e.target.closest("[data-term]");
    if (!r) return;
    e.preventDefault();
    termMenu(e, r.dataset.term, r.textContent.replace(/\s+/g, " ").replace(/^●\s*/, "").trim());
  });
  $("ln-view").addEventListener("click", (e) => {
    const b = e.target.closest("[data-term]");
    if (b) showTerm(b.dataset.term);
  });
}

// the page shown, or its language chosen: the list again, and the term or article shown in that language
async function learnRefresh() {
  learnBuild();
  await learnList();
  await showTerm(LN.current || "sahih");
}

async function learnList() {
  const seq = ++LN.seq;
  const d = await invoke("learn_list", { lang: LN.lang, query: $("ln-q").value });
  if (seq !== LN.seq) return;
  LN.list = d;
  LN.lang = d.lang;
  $("ln-about").textContent = t("The terms of hadith and its sciences — {terms} terms, each with its Arabic, its meaning and its classical source.", { terms: d.count });
  const [, hSign] = lnStyle("history");
  $("ln-history").textContent = `${hSign}  ${d.history}`;
  $("ln-langs").innerHTML = d.languages.map(([c, n]) => `<button class="seg${c === d.lang ? " on" : ""}" data-lang="${c}">${esc(n)}</button>`).join("");
  let chips = `<button class="ln-chip" data-cat="" style="${lnChipStyle(getComputedStyle(document.documentElement).getPropertyValue("--lapis").trim() || "#1D4777", LN.category === null)}">${esc(t("All"))}</button>`;
  const names = [["history", d.history], ...d.category_names];
  for (const [key, name] of names) {
    const [colour, sign] = lnStyle(key);
    chips += `<button class="ln-chip" data-cat="${key}" style="${lnChipStyle(colour, LN.category === key)}" data-tip="${esc(t("Show this category alone"))}">${sign} ${esc(name)}</button>`;
  }
  $("ln-chips").innerHTML = chips;
  let html = "";
  if ((LN.category === null || LN.category === "history") && d.articles.length) {
    const [colour, sign] = lnStyle("history");
    html += `<div class="ln-head" style="color:${lnBright(colour)}">${sign}  ${esc(d.history)}   (${d.articles.length})</div>`;
    for (const a of d.articles) {
      html += `<div class="ln-row" data-term="${LN_ARTICLE}${esc(a.id)}" data-tip="${esc(a.lead)}">●  ${esc(a.title)}</div>`;
    }
  }
  for (const c of d.categories) {
    if (LN.category && c.key !== LN.category) continue;
    if (!c.terms.length) continue;
    const [colour, sign] = lnStyle(c.key);
    html += `<div class="ln-head" style="color:${lnBright(colour)}">${sign}  ${esc(c.name)}   (${c.terms.length})</div>`;
    for (const x of c.terms) {
      const second = d.lang !== "ar" ? x.ar : (x.ar ? x.en : "");
      const text = second && second !== x.name ? `●  ${x.name}   ${second}` : `●  ${x.name}`;
      html += `<div class="ln-row" data-term="${esc(x.id)}" data-tip="${esc(x.short)}">${esc(text)}</div>`;
    }
  }
  $("ln-tree").innerHTML = html;
  markTerm();
}

function markTerm() {
  for (const r of $("ln-tree").querySelectorAll(".ln-row")) {
    const on = r.dataset.term === LN.current;
    r.classList.toggle("on", on);
    if (on) r.scrollIntoView({ block: "nearest" });
  }
}

function termMenu(e, id, name) {
  const article = id.startsWith(LN_ARTICLE);
  const entries = [act("Open", () => pickTerm(id)), SEP];
  if (!article) {
    invoke("learn_term", { lang: LN.lang, id }).then((d) => {
      if (d) entries.push(bookmarkAct("term", id, d.name_en, d.ar, d.marked));
      entries.push(act("Copy the name", () => copyText(name)));
      popupAt(e, entries);
    });
    return;
  }
  entries.push(act("Copy the name", () => copyText(name)));
  popupAt(e, entries);
}

// a term or an article picked in the list: shown, and a place in Back and Forward (NAV2b)
function pickTerm(id) {
  return place(["term", id], () => showTerm(id));
}

// a term opened from elsewhere (a bookmark, Back)
async function openTermAt(id) {
  show("learn");
  learnBuild();
  LN.current = id;
  if (!LN.list) await learnList();
  await showTerm(id);
}

function openTerm(id) {
  return place(["term", id], () => openTermAt(id));
}

function lnHeading(text, colour) {
  return `<div class="ln-heading" style="color:${colour}">${esc(text.toUpperCase())}</div>`;
}

async function showTerm(id) {
  if (id && id.startsWith(LN_ARTICLE)) return showArticle(id.slice(LN_ARTICLE.length));
  const d = await invoke("learn_term", { lang: LN.lang, id });
  if (!d) return;
  LN.current = id;
  const [colour, sign] = lnStyle(d.cat);
  const bright = lnBright(colour);
  const arabic = d.lang === "ar";
  const ar = scriptOf("arabic");
  const scale = Number(getComputedStyle(document.documentElement).getPropertyValue("--scale")) || 1;
  const arFont = (f, bold) => `font-family:${esc(ar.family)};font-size:${(ar.size * f * scale).toFixed(1)}pt;${bold ? "font-weight:700;" : ""}`;
  const dir = arabic ? ' dir="rtl"' : "";
  let h = `<div class="ncard ln-card"><div class="ln-band" style="background:${colour}">  ${sign}  ${esc(d.category)}</div><div class="ln-inner">`;
  h += `<div class="ln-top"><div class="ln-name" style="color:${bright};${arabic ? arFont(1.4, true) : ""}">${esc(d.name)}</div>`;
  if (d.ar && !arabic) h += `<div class="ln-ar" dir="rtl" style="color:${esc(ar.color)};${arFont(1.45, true)}">${esc(d.ar)}</div>`;
  h += `<button class="star${d.marked ? " on" : ""}" id="ln-star">${d.marked ? "★" : "☆"}</button></div>`;
  h += `<div class="caption">${d.others.map(([n, v]) => `${esc(n)}: <b>${esc(v)}</b>`).join(" &nbsp;·&nbsp; ")}</div>`;
  h += `<div class="ln-short" style="background:${lnSoft(colour)};border-left-color:${colour}">${lnHeading(t("In short"), bright)}
    <div class="ln-short-text"${dir} style="${arabic ? `font-family:${esc(ar.family)}` : ""}">${esc(d.short)}</div></div>`;
  if (d.long) {
    h += lnHeading(t("In detail"), bright);
    h += `<div class="ln-long"${dir} style="${arabic ? `font-family:${esc(ar.family)}` : ""}">${esc(d.long)}</div>`;
  }
  if (d.source) h += `<div class="caption ln-source">📜 <b>${esc(t("Source"))}</b>: ${esc(d.source)}</div>`;
  h += "</div></div>";
  if (d.related.length) {
    h += `<div class="ncard ln-more">${lnHeading(t("Related terms"), "var(--muted)")}<div class="ln-flow">`;
    for (const r of d.related) {
      const [rc, rs] = lnStyle(r.cat);
      const label = r.ar && r.ar !== r.name ? `${rs} ${r.name}  ${r.ar}` : `${rs} ${r.name}`;
      h += `<button class="ln-chip" data-term="${esc(r.id)}" style="${lnChipStyle(rc, false)}" data-tip="${esc(r.category + "\n" + r.short)}">${esc(label)}</button>`;
    }
    h += "</div></div>";
  }
  if (d.same.length) {
    h += `<div class="ncard ln-more">${lnHeading(t("More in {category}", { category: d.category }), "var(--muted)")}<div class="ln-flow">`;
    h += d.same.map((x) => `<button class="chip" data-term="${esc(x.id)}" data-tip="${esc(x.short)}">${esc(x.name)}</button>`).join("");
    h += "</div></div>";
  }
  const view = $("ln-view");
  view.innerHTML = h;
  view.scrollTop = 0;
  wireStar($("ln-star"), "term", d.id, d.name_en, d.ar);
  markTerm();
}

// ------------------------------------------------------------------ an article of History and method (LRN-H1)

async function showArticle(id) {
  const a = await invoke("learn_article", { lang: LN.lang, id });
  if (!a) return;
  LN.current = LN_ARTICLE + id;
  const [colour, sign] = lnStyle("history");
  const bright = lnBright(colour);
  const arabic = a.lang === "ar";
  const ar = scriptOf("arabic");
  const scale = Number(getComputedStyle(document.documentElement).getPropertyValue("--scale")) || 1;
  const arFont = (f, bold) => arabic ? `font-family:${esc(ar.family)};font-size:${(ar.size * f * scale).toFixed(1)}pt;${bold ? "font-weight:700;" : ""}` : "";
  const dir = arabic ? ' dir="rtl"' : "";
  let h = `<div class="ncard ln-card"><div class="ln-band" style="background:${colour}">  ${sign}  ${esc(a.labels.history)}</div><div class="ln-inner">`;
  h += `<div class="ln-name ln-article-title"${dir} style="color:${bright};${arFont(1.25, true)}">${esc(a.title)}</div>`;
  h += `<div class="ln-short" style="background:${lnSoft(colour)};border-left-color:${colour}"><div class="ln-short-text"${dir} style="${arabic ? `font-family:${esc(ar.family)}` : ""}">${esc(a.lead)}</div></div>`;
  const blocks = [];
  for (const sec of a.sections) {
    h += `<div class="ln-gap"></div>${lnHeading(sec.heading, bright)}`;
    for (const p of sec.text) h += `<div class="ln-long"${dir} style="${arabic ? `font-family:${esc(ar.family)}` : ""}">${esc(p)}</div>`;
    if (sec.block) {
      h += `<div class="ln-numbers" data-block="${esc(sec.block)}" style="border-color:${lnSoft(colour, 0.45)}"><div class="caption">⏳ ${esc(a.labels.counting)}</div></div>`;
      blocks.push(sec.block);
    }
  }
  if (a.sources) h += `<div class="caption ln-source">📜 <b>${esc(a.labels.sources)}</b>: ${esc(a.sources)}</div>`;
  h += "</div></div>";
  if (a.others.length) {
    h += `<div class="ncard ln-more">${lnHeading(a.labels.more, "var(--muted)")}<div class="ln-flow">`;
    h += a.others.map((x) => `<button class="chip" data-term="${LN_ARTICLE}${esc(x.id)}">${esc(x.title)}</button>`).join("");
    h += "</div></div>";
  }
  const view = $("ln-view");
  view.innerHTML = h;
  view.scrollTop = 0;
  markTerm();
  const shownFor = LN.current, lang = a.lang;
  for (const kind of blocks) {
    const key = `${kind}:${lang}`;
    const fill = (rows) => {
      if (LN.current !== shownFor) return;
      for (const box of view.querySelectorAll(`.ln-numbers[data-block="${kind}"]`)) {
        box.innerHTML = numbersHtml(rows, kind !== "timeline", colour, a.labels);
        if (arabic) box.setAttribute("dir", "rtl");
      }
    };
    if (LN.blocks[key]) fill(LN.blocks[key]);
    else invoke("learn_block", { kind, lang }).then((rows) => {
      LN.blocks[key] = rows || [];
      fill(LN.blocks[key]);
    });
  }
}

// a box of live numbers: a heading row per tradition, then (label, value, note); the timeline as years and events
function numbersHtml(rows, live, colour, labels) {
  if (!rows.length) return `<div class="caption">${esc(labels.no_data)}</div>`;
  let h = `<div class="ln-grid${live ? "" : " timeline"}">`;
  for (const [label, value, note] of rows) {
    if (!value) h += `<div class="ln-road" style="color:${lnBright(colour)}">${esc(label)}</div>`;
    else if (!live) h += `<div class="ln-when"><b>${esc(label)}</b></div><div class="ln-what">${esc(value)}</div>`;
    else {
      const big = /^\d/.test(value) ? " big" : "";
      h += `<div class="ln-label">${esc(label)}</div><div class="ln-value${big}"><b>${esc(value)}</b></div><div class="caption ln-note">${esc(note)}</div>`;
    }
  }
  h += "</div>";
  if (live) h += `<div class="caption">ⓘ ${esc(labels.live)}</div>`;
  return h;
}
