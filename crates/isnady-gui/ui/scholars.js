"use strict";
// Hadith Scholars (gui/scholars_page.py): the compilers, graders and critics present in the imported data, with what
// the data measures of their work — and, on the fourth segment, al-Dhahabi's huffaz generation by generation (HFZ1).
//
// [Hadith Scholars]                 | the name card, his roles, his books, his life
// [All] [Sunni] [Shia] [Huffaz]     | what a compiler / grader / critic is
// the list, by role                 | his collection · his grades and the agreement · his verdicts on narrators

const SC = { built: false, tradition: "sunni", current: null, hafiz: null, tabaqa: "", people: [], seq: 0 };

const SC_ROLE_TEXT = { compiler: "compiler of a collection", grader: "grades hadith", critic: "judges narrators" };
const SC_ROLE_ASK = { compiler: "What is a compiler (musannif)?", grader: "What is a grader?", critic: "What is a critic of narrators?" };
const SC_HIGH_AGREEMENT = 0.95;   // above this between two independent graders, the source data is suspected
const SC_GROUP_COLORS = { "sahih": 2, "hasan": 4, "da'if": 8, "very weak": 10, "fabricated": 12 };
const SC_IMAMI_COLORS = { "sahih": 2, "hasan": 4, "muwaththaq": 5, "da'if": 9 };
const SC_SHIA_RANK_COLORS = { 1: 2, 2: 4, 3: 5, 4: 9 };
const SC_MEASURE = {
  imami: "The Imami grades: sahih — every narrator an Imami thiqa; hasan — a praised Imami among them; muwaththaq — a thiqa of another school among them; da'if — the rest (al-Majlisi's majhul, mursal and marfu' counted here). Compared only among the Shia graders, never with the Sunni ones.",
  agreement: "Agreement: on how many of the hadith both scholars graded they gave the same group (sahih, hasan, da'if, very weak, fabricated).",
  kappa: "Cohen's kappa: agreement after removing what chance alone would give. 1 = always the same, 0 = no better than chance; above 0.8 is very strong, 0.6–0.8 strong, 0.4–0.6 moderate.",
  strictness: "Strictness: comparing his grade with each other grader's on the same hadith, how often his is LOWER minus how often it is HIGHER — by order only (grades are ordered categories, not numbers). Below zero he is stricter (classically mutashaddid), above zero more lenient (mutasahil). Statistics → Graders measures it against a model of the true grade, with intervals.",
};
const SC_NOTE = "The scholars whose work is in the imported data, Sunni and Shia, and what the data measures of it — each by his own tradition's measure.";
const SC_HUFFAZ_NOTE = "Al-Dhahabi's Tadhkirat al-Huffaz (d. 748 AH): the masters of hadith memorisation and criticism, generation by generation, each tied to his narrator entry in the Taqrib where he is there.";

function hasArabic(text) {
  return /[؀-ۿ]/.test(text || "");
}

// one horizontal bar (Bar): the label, a filled share, the number
function scBar(label, value, max, color, suffix, attrs) {
  const share = Math.max(0, Math.min(1, value / Math.max(1, max)));
  const lab = hasArabic(label)
    ? `<span class="sb-label ar">${arabicSpan(label, 0.62)}</span>`
    : `<span class="sb-label">${esc(label)}</span>`;
  return `<div class="sbar"${attrs || ""}>${lab}<span class="sb-track"><span class="sb-fill" style="width:${(share * 100).toFixed(3)}%;background:${color}"></span></span><span class="sb-num">${esc(value.toLocaleString("en-US") + (suffix || ""))}</span></div>`;
}

function scCard(title, body) {
  return `<div class="ncard">${title ? `<div class="ncard-title">${esc(title)}</div>` : ""}${body}</div>`;
}

function scholarsBuild() {
  if (SC.built) return;
  SC.built = true;
  const page = $("page-scholars");
  page.className = "page narrators-page scholars-page";
  page.innerHTML = `<div class="np">
    <div class="np-left sc-left">
      <div class="np-title">${esc(t("Hadith Scholars"))}</div>
      <div class="np-segs" id="sc-segs"></div>
      <div class="caption sc-note" id="sc-note"></div>
      <div class="sc-filters" id="sc-filters" hidden>
        <input id="sc-q" type="search" autocomplete="off" spellcheck="false" placeholder="${esc(t("Name — Arabic or Latin letters"))}">
        <button class="combo" id="sc-tabaqa"></button>
      </div>
      <div class="np-list" id="sc-list"></div>
    </div>
    <div class="np-right" id="sc-view"></div>
  </div>`;
  const segs = [["", "All"], ["sunni", "Sunni"], ["shia", "Shia"], ["huffaz", "Huffaz"]];
  $("sc-segs").innerHTML = segs.map(([k, l]) => `<button class="seg" data-road="${k}">${esc(t(l))}</button>`).join("");
  $("sc-segs").onclick = (e) => {
    const b = e.target.closest("button[data-road]");
    if (b) setScholarRoad(b.dataset.road);
  };
  let typing = null;
  $("sc-q").addEventListener("input", () => {
    clearTimeout(typing);
    typing = setTimeout(fillHuffaz, 220);
  });
  const tabaqat = () => [["", t("Every generation")], ...Array.from({ length: 21 }, (_, i) => [String(i + 1), t("Generation {n}", { n: i + 1 })])];
  const drawTabaqa = () => {
    const [, label] = tabaqat().find(([v]) => v === SC.tabaqa) || tabaqat()[0];
    $("sc-tabaqa").innerHTML = `<span class="lb">${esc(label)}</span><i class="chev"></i>`;
  };
  drawTabaqa();
  $("sc-tabaqa").onclick = () => comboPopup($("sc-tabaqa"), tabaqat(), SC.tabaqa, (v) => {
    SC.tabaqa = v;
    drawTabaqa();
    fillHuffaz();
  });
  $("sc-list").addEventListener("click", (e) => {
    const row = e.target.closest("[data-sid]");
    if (row) pickScholar(row.dataset.sid);
  });
  $("sc-list").addEventListener("contextmenu", (e) => {
    const row = e.target.closest("[data-sid]");
    if (row) scholarMenu(e, row.dataset.sid);
  });
  $("sc-view").addEventListener("click", scholarsClick);
}

function drawScholarSegs() {
  const road = SC.tradition === null ? "" : SC.tradition;
  for (const b of $("sc-segs").children) b.classList.toggle("on", b.dataset.road === road);
}

function setScholarRoad(road) {
  SC.tradition = road === "" ? null : road;
  scholarsRefresh();
}

// the page shown (show "scholars"): the list of the segment chosen, the scholar or the hafiz chosen in it
async function scholarsRefresh() {
  scholarsBuild();
  drawScholarSegs();
  const huffaz = SC.tradition === "huffaz";
  $("sc-filters").hidden = !huffaz;
  $("sc-note").textContent = t(huffaz ? SC_HUFFAZ_NOTE : SC_NOTE);
  if (huffaz) return fillHuffaz();
  const seq = ++SC.seq;
  const d = await invoke("scholars_list");
  if (seq !== SC.seq) return;
  const people = d.people.filter((s) => SC.tradition === null || s.tradition === SC.tradition);
  SC.people = people;
  const P = WIN.people;
  let html = "";
  for (const role of ["compiler", "grader", "critic"]) {
    const group = people.filter((s) => s.roles.includes(role));
    if (!group.length) continue;
    html += `<div class="sc-head">${esc(t(P.role_help[role][0]))}</div>`;
    for (const s of group) {
      const facts = [];
      if (s.tradition === "shia") facts.push(`<span style="color:var(--gold)"><b>${esc(t("Shia"))}</b></span>`);
      if (s.dates) facts.push(esc(s.dates));
      html += `<div class="nrow sc-row" data-sid="${esc(s.id)}" data-tip="${esc(`${s.full} — ${s.arabic}`)}">
        <div class="sc-name">${esc(s.short)}</div>
        ${s.fuller && s.fuller !== s.short ? `<div class="sc-fuller">${esc(s.fuller)}</div>` : ""}
        <div class="sc-ar">${arabicSpan(s.arabic, 0.6)}</div>
        ${facts.length ? `<div class="caption nr-line">${facts.join(" · ")}</div>` : ""}
      </div>`;
    }
  }
  $("sc-list").innerHTML = html;
  if (!people.length) {
    $("sc-view").innerHTML = scCard("", `<div class="lead">${esc(t("No scholars yet: import a collection (File → Data Sources)."))}</div>`);
    return;
  }
  const ids = people.map((p) => p.id);
  await showScholar(ids.includes(SC.current) ? SC.current : ids[0]);
}

function markScholarRow(sid) {
  for (const r of $("sc-list").querySelectorAll(".on")) r.classList.remove("on");
  const row = [...$("sc-list").querySelectorAll("[data-sid]")].find((r) => r.dataset.sid === sid);
  if (row) row.classList.add("on");
  return row;
}

// a row picked in the list: shown, and a place of its own for Back and Forward (NAV2b) — not a generation's heading
async function pickScholar(sid) {
  markScholarRow(sid);
  if (sid.startsWith("tabaqa:")) return showHuffazOverview(Number(sid.slice(7)));
  await place(["scholar", sid], () => showScholar(sid));
}

// a scholar or a hafiz opened from elsewhere (Back, a bookmark, Search): the page, its segment, him chosen
async function openScholarAt(sid) {
  show("scholars");
  scholarsBuild();
  if (sid.startsWith("hafiz:")) {
    SC.hafiz = Number(sid.slice(6));
    SC.tradition = "huffaz";
    return scholarsRefresh();
  }
  SC.current = sid;
  const d = await invoke("scholars_list");
  const s = d.people.find((p) => p.id === sid);
  if (s && SC.tradition !== null && s.tradition !== SC.tradition) SC.tradition = s.tradition;
  if (SC.tradition === "huffaz") SC.tradition = s ? s.tradition : "sunni";
  await scholarsRefresh();
}

function openScholar(sid) {
  return place(["scholar", sid], () => openScholarAt(sid));
}

async function scholarMenu(e, sid) {
  e.preventDefault();
  if (sid.startsWith("tabaqa:")) return;
  if (sid.startsWith("hafiz:")) {
    const r = await invoke("hafiz_page", { n: Number(sid.slice(6)) });
    if (r.error) return;
    popupAt(e, [
      act("Open", () => pickScholar(sid)),
      act("Open him as a narrator", r.person ? () => openNarratorFromScholars(r.person) : null, "", { disabled: !r.person }),
      SEP,
      act("Copy", null, "", { submenu: () => [
        act("His name in Arabic", () => copyText(r.head)),
        ...(r.reading ? [act("His name as read", () => copyText(r.reading))] : [])] }),
    ]);
    return;
  }
  const m = await invoke("scholar_menu", { id: sid });
  if (!m) return;
  popupAt(e, [
    act("Open", () => pickScholar(sid)),
    act("Open him as a narrator", m.narrator ? () => openNarratorFromScholars(m.narrator) : null, "", { disabled: !m.narrator }),
    ...m.books.map((b) => act(t("Open {book}", { book: b.title }), () => openBookFromScholars(b.key), "", { raw: true })),
    SEP,
    act(m.marked ? "★  " + t("Remove from bookmarks") : "☆  " + t("Add to bookmarks"),
      () => invoke("toggle_bookmark", { kind: "scholar", reference: sid, title: m.short_en, subtitle: m.arabic || "" }), "", { raw: true }),
    act("Copy", null, "", { submenu: () => [
      act("His name as read", () => copyText(m.short)),
      ...(m.arabic ? [act("His name in Arabic", () => copyText(m.arabic))] : []),
      ...(m.full ? [act("His full name", () => copyText(m.full))] : [])] }),
  ]);
}

function openNarratorFromScholars(id) {
  return place(["narrator", id], async () => {
    NARR.id = id;
    show("narrators");
    await selectNarrator(id);
  });
}

function openBookFromScholars(key) {
  return place(["book", key], () => openBookAt(key));
}

function scholarsClick(e) {
  const a = e.target.closest("a[data-name], a[data-url], [data-person], [data-book-key], [data-act]");
  if (!a) return;
  if (a.dataset.name) return findNarrator(a.dataset.name);
  if (a.dataset.url) return invoke("open_url", { url: a.dataset.url });
  if (a.dataset.person) return openNarratorFromScholars(Number(a.dataset.person));
  if (a.dataset.bookKey) return openBookFromScholars(a.dataset.bookKey);
  if (a.dataset.act === "overview") return showHuffazOverview(Number(a.dataset.tabaqa));
}

// ------------------------------------------------------------------ one scholar (show_scholar)

function peopleBars(heading, rows) {
  if (!rows.length) return "";
  const top = Math.max(...rows.map((r) => r.count));
  let html = `<div class="n-flabel">${esc(heading)}</div>`;
  for (const r of rows) {
    const tip = r.reading ? t("{reading}\n{arabic}\n{name}\nClick to open this narrator", { reading: r.reading, arabic: r.arabic, name: r.name })
      : t("{name}\nClick to open this narrator", { name: r.name });
    html += scBar(r.label, r.count, top, "var(--lapis)", "", ` data-person="${r.id}" data-tip="${esc(tip)}"`);
  }
  return html;
}

async function showScholar(sid) {
  if (sid.startsWith("hafiz:")) {
    if (SC.tradition !== "huffaz") {
      SC.hafiz = Number(sid.slice(6));
      SC.tradition = "huffaz";
      return scholarsRefresh();
    }
    return showHafiz(Number(sid.slice(6)));
  }
  if (sid.startsWith("tabaqa:")) return showHuffazOverview(Number(sid.slice(7)));
  const seq = ++SC.seq;
  const d = await invoke("scholar_page", { id: sid });
  if (seq !== SC.seq || d.error) return;
  if (SC.tradition && d.tradition !== SC.tradition) {
    SC.tradition = d.tradition;           // a scholar of the other tradition: his own
    SC.current = sid;
    return scholarsRefresh();
  }
  SC.current = sid;
  markScholarRow(sid);
  const shia = d.tradition === "shia";
  const P = WIN.people;
  let html = nameCard(d.view, { marked: d.marked });
  // his tradition, his dates and roles, his works, his books
  const facts = [d.dates, d.roles.map((r) => t(SC_ROLE_TEXT[r])).join(", ")].filter((x) => x);
  let body = `<div class="sc-trow"><span class="${shia ? "tag-gold" : "tag-muted"}" data-tip="${esc(t(shia
    ? "Measured along the road of his own tradition: his chains end at a Ma'sum, his grades are compared only with the Shia graders on the Imami categories, his verdicts are read in the Imami terms."
    : "Measured along the Sunni road: chains to the Prophet, the Sunni grade groups, Ibn Hajar's twelve ranks."))}">${esc(t(shia ? "Shia scholar" : "Sunni scholar"))}</span>
    <span class="lead">${esc(facts.join(" · "))}</span></div>`;
  if (d.works.length) body += `<div class="caption">${esc(t("Works: ") + d.works.join("; "))}</div>`;
  if (d.books.length) {
    body += `<div class="sc-buttons">${d.books.map((b) => `<button class="quiet" data-book-key="${esc(b.key)}">${esc(t("Open {book}", { book: b.title }))}</button>`).join("")}</div>`;
  }
  if (d.narrator) body += `<div class="sc-buttons"><button class="quiet" data-person="${d.narrator}">${esc(t("Open him as a narrator"))}</button></div>`;
  html += scCard("", body);
  if (d.life) html += lifeCard(d.life, d.written);
  for (const role of d.roles) {
    html += `<div class="sc-help"><div class="sc-help-title">${esc(t(SC_ROLE_ASK[role]))}</div><div class="lead">${esc(t(P.role_help[role][1]))}</div></div>`;
  }
  const pct = (a, b) => (b ? `${(100 * a / b).toFixed(1)}%` : "—");
  for (const st of d.compiler) {
    const values = { hadith: st.hadith, chains: st.chains, split: pct(st.split, st.chains), reach: pct(st.reach, st.chains), identified: pct(st.identified, st.links) };
    let c = `<div class="lead">${st.tradition === "shia"
      ? t("<b>{hadith:,}</b> hadith · <b>{chains:,}</b> chains read, {split} split into narrators, {reach} reaching a Ma'sum (the Prophet or an Imam) · {identified} of the narrators identified in al-Najashi", values)
      : t("<b>{hadith:,}</b> hadith · <b>{chains:,}</b> chains read, {split} split into narrators, {reach} reaching the Prophet · {identified} of the narrators identified in the Taqrib", values)}</div>`;
    c += peopleBars(t("His teachers — the first link of his chains"), st.teachers);
    c += peopleBars(t("The Companions his chains end with most"), st.companions);
    if (st.ends.length) {
      c += `<div class="n-flabel">${esc(t("Whom his chains reach"))}</div>`;
      const top = Math.max(...st.ends.map((x) => x.count));
      for (const x of st.ends.slice(0, 10)) c += scBar(x.label, x.count, top, "var(--gold)");
    }
    html += scCard(t("His collection: {book}", { book: st.book }), c);
  }
  const g = d.grader;
  if (g) {
    const imami = g.tradition === "shia";
    const colors = imami ? SC_IMAMI_COLORS : SC_GROUP_COLORS;
    const top = Math.max(1, ...g.distribution.map(([, n]) => n));
    let c = "";
    for (const [label, n] of g.distribution) {
      c += scBar(t(label), n, top, rankColor(colors[label] || 8), `  (${pyRound(100 * n / Math.max(1, g.graded))}%)`);
    }
    if (imami) c += `<div class="caption">${esc(t(SC_MEASURE.imami))}</div>`;
    const ar = scriptOf("arabic");
    const words = g.wordings.map(([w, n]) => (hasArabic(w)
      ? `<span style="font-family:${esc(ar.family)}">${esc(w)}</span> (${n.toLocaleString("en-US")})`
      : `${esc(w)} (${n.toLocaleString("en-US")})`)).join(", ");
    c += `<div class="caption">${imami
      ? t("Grouped in the Imami categories for comparison; the wording is kept. His most frequent wordings: {words}", { words })
      : t("Grouped for comparison; the wording is kept. His most frequent wordings: {words}", { words })}</div>`;
    html += scCard(t("His grades: {graded:,} hadith in {books}", { graded: g.graded, books: g.books.join(", ") }), c);
    c = "";
    if (!g.agreement.length) c += `<div class="caption">${esc(t("No other grader of his tradition graded the same hadith in the data imported here."))}</div>`;
    c += `<div class="sc-help small">${["agreement", "kappa", "strictness"].map((k) => `<div class="caption">${esc(t(SC_MEASURE[k]))}</div>`).join("")}</div>`;
    for (const a of g.agreement) {
      const kappa = a.kappa === null ? "—" : a.kappa.toFixed(2);
      c += `<div class="lead">${t("<b>{other}</b>: same group on <b>{agree:.1f}%</b> of {common:,} hadith (within one step {within:.1f}%) · Cohen's kappa {kappa} · he grades lower on {lower:.1f}%, higher on {higher:.1f}%",
        { other: esc(a.other), agree: 100 * a.agree, common: a.common, within: 100 * a.within_one, kappa, lower: 100 * a.lower, higher: 100 * a.higher })}</div>`;
      if (a.agree >= SC_HIGH_AGREEMENT) {
        c += `<div class="caption" style="color:var(--gold)">${esc(t("⚠ {value:.1f}% agreement is unusually high for two scholars grading on their own. The source may have filled one column from the other; treat this pair as not independent until the source is checked.", { value: 100 * a.agree }))}</div>`;
      }
    }
    if (g.strictness !== null) {
      const s = g.strictness;
      const word = s < -0.05 ? t("stricter") : s > 0.05 ? t("more lenient") : t("close to the others");
      c += `<div class="lead">${t("<b>Strictness {s:+.2f}</b> — {word} than the other graders on the same hadith: his grade lower in {value:.1f}%, higher in {value2:.1f}% of {compared:,} comparisons. ",
        { s, word, value: 100 * g.lower, value2: 100 * g.higher, compared: g.compared })}${esc(imami
        ? t("By the order of the Imami categories only, among the Shia graders.")
        : t("The classical mutashaddid / mutasahil, by order only. Statistics → Graders measures it against a model of the true grade, with intervals."))}</div>`;
    }
    html += scCard(imami ? t("Agreement with the other Shia graders on the same hadith") : t("Agreement with the other graders on the same hadith"), c);
  }
  const cr = d.critic;
  if (cr) {
    const top = Math.max(1, ...cr.ranks.map(([, , n]) => n));
    let c = "";
    const first = (label) => t(label).split(" (")[0];
    if (cr.tradition === "shia") {
      c += `<div class="n-flabel">${esc(t("Reliability, in the Imami terms of his own words (never Ibn Hajar's twelve ranks)"))}</div>`;
      for (const [rank, label, n] of cr.ranks) c += scBar(first(label), n, top, rankColor(SC_SHIA_RANK_COLORS[rank] || 8));
      if (cr.creeds.length) {
        c += `<div class="n-flabel">${esc(t("Creed, where he states it"))}</div>`;
        const ctop = Math.max(...cr.creeds.map(([, n]) => n));
        for (const [label, n] of cr.creeds) c += scBar(t(label), n, ctop, "var(--gold)");
      }
      if (cr.unranked) c += `<div class="caption">${esc(t("{unranked:,} entries give no judgment in these words (often an author listed for his books); their words are kept.", { unranked: cr.unranked }))}</div>`;
    } else {
      for (const [rank, label, n] of cr.ranks) c += scBar(`${rank}. ${first(label)}`, n, top, rankColor(rank));
      if (cr.unranked) c += `<div class="caption">${esc(t("{unranked:,} verdicts do not fit one of his twelve ranks; their words are kept.", { unranked: cr.unranked }))}</div>`;
    }
    html += scCard(t("His verdicts on narrators: {verdicts:,} in {works}", { verdicts: cr.verdicts, works: cr.works.join(", ") }), c);
  }
  const view = $("sc-view");
  view.innerHTML = html;
  view.scrollTop = 0;
  const star = view.querySelector(".nc-top .star");
  if (star) wireStar(star, "scholar", d.id, d.short_en, d.arabic || "");
  if (d.life) fillEntries(d.life, "#sc-view");
}

// ------------------------------------------------------------------ the huffaz (HFZ1)

function hafizDied(r) {
  if (!r.year) return "";
  let text = t("d. {year} AH", { year: r.year });
  if (r.whose === "taqrib") text += " " + t("(Taqrib)");
  else if (r.taqrib_death && r.taqrib_death !== r.year) text += " " + t("(Taqrib: {year})", { year: r.taqrib_death });
  return text;
}

// the list: each generation's heading, then its huffaz
async function fillHuffaz() {
  if (SC.tradition !== "huffaz") return;
  const seq = ++SC.seq;
  const d = await invoke("huffaz_list", { text: $("sc-q").value.trim(), tabaqa: SC.tabaqa ? Number(SC.tabaqa) : null });
  if (seq !== SC.seq) return;
  const gens = Object.fromEntries(d.generations.map((g) => [g.tabaqa, g]));
  const arabicFirst = APP.ui_language === "ar";
  let html = "", last = null, current = false;
  for (const r of d.rows) {
    if (r.tabaqa !== last) {
      last = r.tabaqa;
      const g = gens[last];
      const span = g.median ? t(" · d. {start}–{end} AH", { start: g.from, end: g.to }) : "";
      html += `<div class="sc-gen" data-sid="tabaqa:${last}">${esc(t("Generation {n}", { n: last }) + ` (${g.count})` + span)}</div>`;
    }
    const first = arabicFirst || !r.reading ? r.name : r.reading;
    const second = [String(r.n), hafizDied(r), first === r.name ? "" : r.name].filter((x) => x).join(" · ");
    const on = r.n === SC.hafiz;
    if (on) current = true;
    html += `<div class="sc-hafiz${on ? " on" : ""}" data-sid="hafiz:${r.n}" data-tip="${esc(r.head)}"><div>${esc(first)}</div><div>${esc(second)}</div></div>`;
  }
  $("sc-list").innerHTML = html;
  if (current) {
    const row = $("sc-list").querySelector(".sc-hafiz.on");
    if (row) row.scrollIntoView({ block: "nearest" });
    await showHafiz(SC.hafiz);
  } else {
    await showHuffazOverview(SC.tabaqa ? Number(SC.tabaqa) : null);
  }
}

// the book at a glance: how many huffaz each generation holds, when they died, how many the Taqrib has too
async function showHuffazOverview(tabaqa) {
  const d = await invoke("huffaz_overview");
  let html = scCard(t("Al-Dhahabi, Tadhkirat al-Huffaz"),
    `<div class="lead">${t("<b>{count:,}</b> huffaz in <b>{generations}</b> generations, from Abu Bakr al-Siddiq to al-Dhahabi's own teachers · a year of death for {dated:,} · <b>{linked:,}</b> of them are narrators of the Taqrib, and open in the Narrators page.",
      { count: d.count, generations: d.generations.length, dated: d.dated, linked: d.linked })}</div>
    <div class="caption">${esc(t("A hafiz is not every narrator: al-Dhahabi lists the masters who memorised the hadith in great numbers and judged their narrators. The many narrators of the first centuries are in the Narrators page (Ibn Hajar's Taqrib)."))}</div>`);
  const top = Math.max(...d.generations.map((g) => g.count));
  let c = "";
  for (const g of d.generations) {
    const label = g.median ? t("Gen. {n} · {start}–{end} AH", { n: g.tabaqa, start: g.from, end: g.to }) : t("Generation {n}", { n: g.tabaqa });
    const stated = g.stated ? t(" — al-Dhahabi's own count: {n}", { n: g.stated }) : "";
    const tip = t("{count} huffaz; {linked} in the Taqrib; half of them died between {start} and {end} AH (median {median}){stated}",
      { count: g.count, linked: g.in_taqrib, start: g.from || "—", end: g.to || "—", median: g.median || "—", stated });
    c += scBar(label, g.count, top, g.tabaqa === tabaqa ? "var(--gold)" : "var(--lapis)", "", ` data-tip="${esc(tip)}"`);
  }
  c += `<div class="caption">${esc(t("The years are the middle half of each generation's deaths; the counts are read from the book, and where al-Dhahabi gives a generation's count himself it is in the tooltip."))}</div>`;
  html += scCard(t("Generation by generation"), c);
  const ctop = Math.max(...d.centuries.map(([, n]) => n));
  html += scCard(t("When they died, by century of the Hijra"), d.centuries.map(([cn, n]) => scBar(t("{n}. century AH", { n: cn }), n, ctop, "var(--lapis)")).join(""));
  const view = $("sc-view");
  view.innerHTML = html;
  view.scrollTop = 0;
}

async function showHafiz(n) {
  const r = await invoke("hafiz_page", { n });
  if (r.error) return;
  SC.hafiz = n;
  for (const x of $("sc-list").querySelectorAll(".sc-hafiz.on")) x.classList.remove("on");
  const row = $("sc-list").querySelector(`[data-sid="hafiz:${n}"]`);
  if (row) row.classList.add("on");
  let c = "";
  if (r.reading) {
    c += `<div class="np-title">${esc(r.reading)}</div>`;
    if (r.taqrib_reading && r.taqrib_reading !== r.reading) c += `<div class="sc-fuller">${esc(r.taqrib_reading)}</div>`;
  }
  c += `<div class="sc-head-ar">${arabicSpan(r.head, 0.85)}</div>`;
  const facts = [t("Tadhkirat al-Huffaz no. {n}", { n: r.n }), t("generation {n} of 21", { n: r.tabaqa })];
  const died = hafizDied(r);
  if (died) facts.push(died);
  c += `<div class="lead">${esc(facts.join(" · "))}</div>`;
  if (r.whose === "dhahabi") c += `<div class="caption">${esc(t("The year of death is al-Dhahabi's, read from his words in the entry."))}</div>`;
  const g = r.generation;
  if (g && g.median) c += `<div class="caption">${esc(t("Half of his generation died between {start} and {end} AH.", { start: g.from, end: g.to }))}</div>`;
  if (r.rumuz.length) {
    c += `<div class="lead">${esc(t("Narrates in: {books}", { books: r.rumuz_text.join("; ") }))}   <span style="color:var(--muted)">[${esc(r.rumuz.join(" "))}]</span></div>`;
    c += `<div class="caption">${esc(t("Al-Mizzi's signs, which al-Dhahabi puts before the name: in whose book of the six his hadith are."))}</div>`;
  }
  let html = scCard("", c);
  if (r.person) {
    c = `<div class="sc-head-ar">${arabicSpan(r.taqrib_name || "", 0.7)}</div>`;
    const lines = (r.verdicts || []).map((v) => t("{critic}: rank {rank} of 12, {label}", { critic: v.critic, rank: v.rank, label: v.label }));
    if (r.in_chains) lines.push(n_("In the books imported here: in {links:,} chain link", "In the books imported here: in {links:,} chain links", r.in_chains, { links: r.in_chains }));
    for (const line of lines) c += `<div class="lead">${esc(line)}</div>`;
    c += `<div class="caption">${esc(t("Tied by the name, the father and the grandfather and the two years of death; where these do not agree he is left untied."))}</div>`;
    c += `<div class="sc-buttons"><button class="quiet" data-person="${r.person}">${esc(t("Open him as a narrator"))}</button></div>`;
    html += scCard(t("In Ibn Hajar's Taqrib"), c);
  }
  const e = r.entry;
  html += scCard(t("His entry in the Tadhkira"),
    `<div class="lc-entry" data-entry="0"><div class="lc-entry-head"><b>${esc(t("In the source"))}:</b> ${esc(e.title)}, ${esc(e.where)}</div><div class="lc-entry-body"></div></div>`);
  html += `<div class="sc-buttons"><button class="quiet" data-act="overview" data-tabaqa="${r.tabaqa}">${esc(t("All generations"))}</button></div>`;
  const view = $("sc-view");
  view.innerHTML = html;
  view.scrollTop = 0;
  fillEntries({ entries: [e] }, "#sc-view");
}
