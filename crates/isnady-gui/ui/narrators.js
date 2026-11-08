// The Narrators page, as the Python window draws it (gui/narrators_page.py, name_card.py, life_card.py): every
// narrator of the imported rijal works, Sunni and Shia on one page but never mixed — each by his own tradition's
// measure. Left: the roads, the overview, the search, the famous ones, the filters, the list. Right: the narrator —
// his name laid out, his life, what the critics say, teachers and students, the hadith he is in.

const LIST_LIMIT = 200;
const ROWS_FIRST = 30;         // rows built at once; the rest a batch at a time after the window has drawn them
const ROWS_BATCH = 40;
const SHIA_DOT = { 1: 2, 2: 4, 3: 5, 4: 9 };   // the Shia categories drawn in the rank colours of like standing
const SIX_BOOKS = [["bukhari", "al-Bukhari"], ["muslim", "Muslim"], ["abudawud", "Abu Dawud"],
  ["tirmidhi", "al-Tirmidhi"], ["nasai", "al-Nasa'i"], ["ibnmajah", "Ibn Maja"]];
const SPELLED = "A word of this name is not in isnady's list of names yet, so it is spelled letter by letter from the Arabic: the consonants are right, the short vowels (not written in Arabic) are a guess.";
const NAME_EXPLAIN = "An Arabic name is made of parts: the <b>name</b> (ism), the <b>lineage</b> (nasab — b. means son of), the <b>kunya</b> (Abu … , father of …), a <b>by-name</b> (laqab) and <b>nisbas</b> telling where a man was from, his tribe or his trade. A narrator is usually known by one of them — marked ★.";
const PART_LABELS = {
  ism: ["Name", "ism", "الاسم", "the given name"],
  nasab: ["Lineage", "nasab", "النسب", "son of … (b. = ibn, son of)"],
  kunya: ["Kunya", "kunya", "الكنية", "Abu … / Umm …: father or mother of …"],
  laqab: ["By-name", "laqab", "اللقب", "a nickname or title"],
  nisba: ["Nisbas", "nisba", "النسبة", "where he was from, his tribe or his trade"],
  wala: ["Client of", "wala'", "الولاء", "a freedman or client of a tribe"],
  also: ["Also called", "", "", "other names the work gives"],
};

let NARR = {
  loaded: false, road: "sunni", id: null, famous: false, filters: { tabaqa: "", rank: "", book: "" },
  token: 0, left: [], read: 0, total: 0, shown: new Set(), args: null, sel: null,
};

// ------------------------------------------------------------------ the left side

function narrBuild() {
  const page = $("page-narrators");
  page.innerHTML = `<div class="np">
    <div class="np-left">
      <div class="np-title">${esc(t("Narrators"))}</div>
      <div class="np-segs" id="n-roads"></div>
      <div class="caption np-overview" id="n-overview"></div>
      <input id="nq" type="search" autocomplete="off" spellcheck="false" placeholder="${esc(t("Name, kunya or nisba — Arabic or Latin letters"))}">
      <div class="np-row"><button class="nlink" id="n-famous"></button></div>
      <div class="np-filter" id="nf-tabaqa"><div class="np-ftitle"></div><button class="fcombo" data-f="tabaqa"></button></div>
      <div class="np-filter" id="nf-rank"><div class="np-ftitle"></div><button class="fcombo" data-f="rank"></button></div>
      <div class="np-filter" id="nf-book"><div class="np-ftitle"></div><button class="fcombo" data-f="book"></button></div>
      <div class="np-row end"><button class="nlink" id="n-clear" hidden>${esc(t("Clear filters"))}</button></div>
      <div class="np-list" id="n-list"></div>
      <div class="caption" id="n-count"></div>
    </div>
    <div class="np-right" id="n-view"></div>
  </div>`;
  const roads = [["", "All", "Sunni and Shia narrators together, each by his own tradition's measure"],
    ["sunni", "Sunni", "Narrators of the Sunni rijal works (Ibn Hajar's Taqrib): twelve ranks"],
    ["shia", "Shia", "Narrators of the Shia rijal works (al-Najashi …): the Imami terms"]];
  $("n-roads").innerHTML = roads.map(([k, l, tip]) =>
    `<button class="seg${NARR.road === k ? " on" : ""}" data-road="${k}" data-tip="${esc(t(tip))}">${esc(t(l))}</button>`).join("");
  $("n-roads").onclick = (e) => {
    const b = e.target.closest("button[data-road]");
    if (b) setRoad(b.dataset.road);
  };
  let typing = null;
  $("nq").addEventListener("input", () => {
    if (NARR.famous) { NARR.famous = false; famousText(); }
    clearTimeout(typing);
    typing = setTimeout(listNarrators, 250);
  });
  $("n-famous").onclick = () => {
    NARR.famous = !NARR.famous;
    famousText();
    if (NARR.famous) showFamous(); else listNarrators();
  };
  $("n-clear").onclick = () => {
    NARR.filters = { tabaqa: "", rank: "", book: "" };
    filtersChanged();
  };
  for (const b of document.querySelectorAll("#page-narrators .fcombo")) {
    b.onclick = () => comboPopup(b, filterItems(b.dataset.f), NARR.filters[b.dataset.f], (v) => {
      NARR.filters[b.dataset.f] = v;
      filtersChanged();
    });
  }
  $("n-list").addEventListener("click", (e) => {
    const row = e.target.closest(".nrow[data-id]");
    if (!row) return;
    const id = Number(row.dataset.id);
    markRow(id);
    openNarrator(id);
  });
  $("n-list").addEventListener("contextmenu", (e) => {
    const row = e.target.closest(".nrow[data-id]");
    if (row) rowMenu(e, Number(row.dataset.id));
  });
  $("n-list").addEventListener("scroll", nearEnd);
  famousText();
  showFilters();
}

function famousText() {
  const b = $("n-famous");
  if (NARR.famous) b.textContent = "← " + t("All narrators");
  else b.textContent = "★ " + t(NARR.road === "shia" ? "Famous narrators" : "Famous Companions");
  b.dataset.tip = NARR.road !== "shia"
    ? t("The four Caliphs, the ten promised Paradise, the Mothers of the Believers, the Prophet's household, those who narrated most …")
    : t("Ashab al-Ijma'");
}

function filterItems(which) {
  const P = WIN.people;
  const first = (s) => s.split(" (")[0];
  if (which === "tabaqa") {
    return [["", t("All generations")], ...Object.keys(P.tabaqat).map((k) =>
      [k, t("Tabaqa {tabaqa}: {meaning}", { tabaqa: k, meaning: first(t(P.tabaqat[k])) })])];
  }
  if (which === "book") return [["", t("All books")], ...SIX_BOOKS.map(([k, n]) => [k, t("Narrates in {name}", { name: n })])];
  if (NARR.road === "shia") {
    return [["", t("All assessments")], ...Object.keys(P.shia_ranks).map((k) => {
      const en = t(P.shia_ranks[k][1]);
      return [k, en.charAt(0).toUpperCase() + en.slice(1)];
    }), ["-1", t("No judgment in the work")]];
  }
  return [["", t("All ranks")], ...Object.keys(P.ranks).map((k) =>
    [k, t("Rank {rank}: {meaning}", { rank: k, meaning: first(t(P.ranks[k][1])) })])];
}

// each filter under its own title, its choice coloured when one is made (UI6)
function showFilters() {
  const sunni = NARR.road === "sunni";
  $("nf-tabaqa").hidden = !sunni;
  $("nf-book").hidden = !sunni;
  $("nf-rank").hidden = !NARR.road;
  $("nf-tabaqa").firstElementChild.textContent = t("Generation (tabaqa)");
  $("nf-rank").firstElementChild.textContent = t(NARR.road === "shia" ? "Assessment (Imami terms)" : "Rank (Ibn Hajar)");
  $("nf-book").firstElementChild.textContent = t("Book");
  for (const b of document.querySelectorAll("#page-narrators .fcombo")) {
    const items = filterItems(b.dataset.f);
    const v = NARR.filters[b.dataset.f];
    const [, label] = items.find((x) => x[0] === v) || items[0];
    b.innerHTML = `<span class="lb">${esc(label)}</span><i class="chev"></i>`;
    b.classList.toggle("active", v !== "");
  }
}

function filtersChanged() {
  showFilters();
  const visible = (f) => !$("nf-" + f).hidden;
  $("n-clear").hidden = !["tabaqa", "rank", "book"].some((f) => NARR.filters[f] !== "" && visible(f));
  listNarrators();
}

function setRoad(road) {
  if (road === NARR.road) return;
  NARR.road = road;
  for (const b of $("n-roads").children) b.classList.toggle("on", b.dataset.road === road);
  NARR.filters = { tabaqa: "", rank: "", book: "" };
  NARR.id = null;
  NARR.famous = false;
  famousText();
  filtersChanged();
  overview();
}

async function overview() {
  const d = await invoke("narrators_overview", { tradition: NARR.road });
  let persons = 0;
  const lines = [];
  for (const o of d.roads) {
    persons += o.persons;
    if (!o.persons) continue;
    const share = o.links ? (100 * o.identified / o.links).toFixed(1) + "%" : "—";
    const shia = o.road === "shia";
    const tag = shia ? `<span style="color:var(--gold)"><b>${esc(t("Shia"))}</b></span>` : `<b>${esc(t("Sunni"))}</b>`;
    const chains = o.links
      ? t(shia ? "{identified:,} of {links:,} names in the Shia chains identified ({share})" : "{identified:,} of {links:,} names in the Sunni chains identified ({share})",
        { identified: o.identified, links: o.links, share })
      : t(shia ? "no Shia chains imported yet" : "no Sunni chains imported yet");
    lines.push(`${tag}: ` + esc(t("{persons:,} narrators from {works}; {chains}.", { persons: o.persons, works: o.works.join(", "), chains })));
  }
  $("n-overview").innerHTML = lines.join("<br>");
  return persons;
}

async function loadNarrators() {
  if (!NARR.loaded) {
    NARR.loaded = true;
    narrBuild();
  }
  const persons = await overview();
  if (!persons) {
    $("n-list").innerHTML = "";
    $("n-count").textContent = "";
    showEmpty();
    return;
  }
  const shown = NARR.id;
  if (NARR.famous) await showFamous(); else await listNarrators();
  if (shown) await openNarrator(shown, true);
}

async function listNarrators() {
  if (!NARR.loaded) return;
  if (NARR.famous) return showFamous();
  const f = NARR.filters;
  NARR.args = {
    text: $("nq").value, tradition: NARR.road, tabaqa: f.tabaqa ? Number(f.tabaqa) : null,
    rank: f.rank ? Number(f.rank) : null, book: f.book || null,
  };
  const token = ++NARR.token;
  const d = await invoke("narrators_list", Object.assign({ offset: 0, limit: LIST_LIMIT }, NARR.args));
  if (token !== NARR.token) return;
  NARR.read = d.rows.length;
  NARR.total = d.total;
  NARR.shown = new Set();
  NARR.sel = null;
  $("n-list").innerHTML = "";
  $("n-list").scrollTop = 0;
  NARR.left = d.rows.slice();
  addRows(token, ROWS_FIRST);
  showCount();
  if (NARR.id === null && d.rows.length) openNarrator(d.rows[0].id, true);
}

async function showFamous() {
  const token = ++NARR.token;
  const d = await invoke("narrators_famous", { tradition: NARR.road });
  if (token !== NARR.token) return;
  NARR.left = [];
  NARR.read = NARR.total = 0;
  NARR.shown = new Set();
  let html = "", first = null, shown = 0;
  for (const g of d.groups) {
    html += `<div class="nhead">${esc(t(g.title))}</div>`;
    for (const r of g.rows) {
      html += rowHtml(r);
      shown++;
      first = first || r.id;
    }
  }
  NARR.sel = null;
  $("n-list").innerHTML = html;
  $("n-list").scrollTop = 0;
  $("n-count").textContent = n_("{n:,} famous one", "{n:,} famous ones", shown);
  if (first && NARR.id === null) openNarrator(first, true);
}

function showCount() {
  const more = NARR.total > NARR.read ? " — " + t("{shown:,} listed; scroll down for the next", { shown: NARR.read }) : "";
  $("n-count").textContent = n_("{n:,} narrator", "{n:,} narrators", NARR.total) + more;
}

function addRows(token, count) {
  if (token !== NARR.token) return;
  const take = count ? NARR.left.splice(0, count) : NARR.left.splice(0);
  const html = take.filter((r) => !NARR.shown.has(r.id)).map(rowHtml).join("");
  $("n-list").insertAdjacentHTML("beforeend", html);
  if (NARR.sel !== null) markRow(NARR.sel);
  if (NARR.left.length) requestAnimationFrame(() => setTimeout(() => addRows(token, ROWS_BATCH), 0));
}

// NR-B1: the next narrators when the list is scrolled to its end
async function nearEnd() {
  const list = $("n-list");
  if (NARR.famous || NARR.left.length || NARR.busy || NARR.read >= NARR.total) return;
  if (list.scrollTop + list.clientHeight < list.scrollHeight - list.clientHeight / 2) return;
  NARR.busy = true;
  try {
    const token = NARR.token;
    const d = await invoke("narrators_list", Object.assign({}, NARR.args, { offset: NARR.read, limit: LIST_LIMIT }));
    if (token !== NARR.token) return;
    if (!d.rows.length) NARR.total = NARR.read;
    NARR.read += d.rows.length;
    NARR.left = d.rows.slice();
    addRows(token, ROWS_FIRST);
    showCount();
  } finally {
    NARR.busy = false;
  }
}

function dotOf(r) {
  const shia = r.tradition === "shia";
  return rankColor(shia ? SHIA_DOT[r.rank] : r.rank);
}

// one narrator of the list: the name he is known by and its Arabic, the full name, the facts
function rowHtml(r) {
  NARR.shown.add(r.id);
  const P = WIN.people;
  const v = r.view;
  const known = r.famous || v.reading || v.full_reading;
  const ar = scriptOf("arabic");
  let tip = v.how ? t("{full_reading}\nknown by {how}", { full_reading: v.full_reading, how: t(v.how) }) : v.full_reading;
  if (v.spelled) tip += "\n" + t(SPELLED);
  let html = `<div class="nrow${r.id === NARR.sel ? " on" : ""}" data-id="${r.id}" data-tip="${esc(r.name)}"><div class="nr-top">`;
  if (known) html += `<span class="nr-title"><span data-tip="${esc(tip)}">${esc(known)}</span></span>`;
  const arStyle = `font-family:${esc(ar.family)};font-size:${(ar.size * 0.7).toFixed(1)}pt;color:${esc(ar.color)}${known ? "" : ";font-weight:700"}`;
  html += `<span class="nr-ar${known ? "" : " alone"}" dir="rtl" style="${arStyle}">${esc(v.arabic || r.name)}</span></div>`;
  if (r.famous && (v.reading || v.full_reading)) html += `<div class="caption nr-line">${esc(v.full_reading || v.reading)}</div>`;
  else if (known && v.full_reading && v.full_reading !== known) html += `<div class="caption nr-line">${esc(v.full_reading)}</div>`;
  const facts = [];
  if (r.tradition === "shia") {
    facts.push(`<span style="color:var(--gold)"><b>${esc(t("Shia"))}</b></span>`);
    facts.push(r.rank ? `<span style="color:${dotOf(r)}">●</span> ` + esc(t(P.shia_ranks[r.rank][1]).split(" (")[0]) : esc(t("no judgment")));
    if (r.madhhab) facts.push(esc(t(P.madhhabs[r.madhhab] || r.madhhab)));
  } else if (r.rank) {
    facts.push(`<span style="color:${dotOf(r)}">●</span> ` + esc(t("rank {rank}", { rank: r.rank })));
  }
  if (r.tabaqa) facts.push(esc(t("tabaqa {tabaqa}", { tabaqa: r.tabaqa })));
  if (r.death) facts.push(esc(t("d. {year}", { year: r.death })));
  facts.push(esc(t("{n:,} in chains", { n: r.in_chains })));
  return html + `<div class="caption nr-line">${facts.join(" · ")}</div></div>`;
}

function markRow(id) {
  NARR.sel = id;
  for (const x of $("n-list").querySelectorAll(".nrow.on")) x.classList.remove("on");
  const row = id === null ? null : $("n-list").querySelector(`.nrow[data-id="${id}"]`);
  if (row) row.classList.add("on");
}

// right-click on a narrator of the list (CTX1): open, his trees, his entry, bookmark, copy his name
async function rowMenu(e, id) {
  e.preventDefault();
  const r = await invoke("narrator_names", { id });
  if (!r) return;
  popupAt(e, [
    act("Open", () => { markRow(id); openNarrator(id); }),
    act("His family", () => openGenealogy(id, "family")),
    act("His line of learning", () => openGenealogy(id, "learning")),
    act("His entry in the rijal work", () => openInBookOf(id)),
    SEP,
    bookmarkAct("narrator", r.key, r.title, r.arabic, r.marked),
    act("Copy", null, "", { submenu: () => [
      act("His name in Arabic", () => copyText(r.arabic || r.name_ar)),
      ...(r.reading ? [act("His name as read", () => copyText(r.reading))] : []),
      act("The whole entry heading", () => copyText(r.name_ar))] }),
  ]);
}

function bookmarkAct(kind, reference, title, subtitle, marked) {
  return act(marked ? "★  " + t("Remove from bookmarks") : "☆  " + t("Add to bookmarks"),
    () => invoke("toggle_bookmark", { kind, reference, title, subtitle: subtitle || "" }), "", { raw: true });
}

function openInBookOf(id) {
  return openPersonInBook(id);
}

// ------------------------------------------------------------------ the right side

function showEmpty() {
  let html = `<div class="ncard"><div class="hero-title np-hero">${esc(t("No narrators yet"))}</div>`;
  if (NARR.road !== "shia") html += `<div class="lead">${esc(t("Sunni narrators come from Ibn Hajar's Taqrib al-Tahdhib — 8,824 narrators with his verdict on each; every name in the chains of the Sunni books is matched to its narrator."))}</div>`;
  if (NARR.road !== "sunni") html += `<div class="lead">${esc(t("Shia narrators come from al-Najashi's Rijal — 1,266 authors and narrators with his judgment on each, read in the Imami terms (reliability and creed); the chains of the Shia books are matched to them."))}</div>`;
  html += `<div><button class="primary" id="n-sources">${esc(t("Open Data Sources"))}</button></div></div>`;
  $("n-view").innerHTML = html;
  $("n-sources").onclick = later(["&File", "Data Sources…"]);
}

// Qt draws a box square when its radius is more than half its height
function squareBoxes(root, selector) {
  for (const el of root.querySelectorAll(selector)) {
    const r = parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
    if (r > el.offsetHeight / 2) el.classList.add("square");
  }
}

function nCard(title, body) {
  return `<div class="ncard">${title ? `<div class="ncard-title">${esc(title)}</div>` : ""}${body}</div>`;
}

function arabicSpan(text, factor, extra) {
  const s = scriptOf("arabic");
  const scale = Number(getComputedStyle(document.documentElement).getPropertyValue("--scale")) || 1;
  return `<span class="n-ar ${extra || ""}" dir="rtl" style="font-family:${esc(s.family)};font-size:${(s.size * factor * scale).toFixed(1)}pt;color:${esc(s.color)}">${esc(text)}</span>`;
}

// the name card (NM1): the name he is known by, his full name, the parts of his name
function nameCard(view, ref) {
  const star = ref ? `<button class="star${ref.marked ? " on" : ""}" id="n-star">${ref.marked ? "★" : "☆"}</button>` : "";
  let html = `<div class="nc-top"><span class="nc-reading"${view.spelled ? ` data-tip="${esc(t(SPELLED))}"` : ""}>${esc(view.reading || view.full_reading || "")}</span>${arabicSpan(view.arabic, 1.25, "bold")}${star}</div>`;
  const sub = [];
  if (view.full_reading && view.full_reading !== view.reading) sub.push(esc(view.full_reading));
  if (view.how) sub.push(`<span style="color:var(--gold)">★</span> ` + esc(t("known by {how}", { how: t(view.how) })));
  if (view.spelled) sub.push(`<span style="color:var(--muted)">${esc(t("Latin letters spelled from the Arabic"))}</span>`);
  if (sub.length) html += `<div class="lead nc-sub"${view.spelled ? ` data-tip="${esc(t(SPELLED))}"` : ""}>${sub.join(" &nbsp;·&nbsp; ")}</div>`;
  if (view.rows.length) {
    html += `<div class="nc-rule"></div><div class="nc-grid">`;
    for (const [key, values, starred] of view.rows) {
      const [title, term, arTerm, meaning] = PART_LABELS[key];
      html += `<div class="nc-head" data-tip="${esc(t(meaning))}"><span style="color:var(--gold)">${starred ? "★" : "&nbsp;&nbsp;"}</span> <b>${esc(t(title))}</b>` +
        (term ? `<br><span class="nc-term">${esc(term)} · ${esc(arTerm)}</span>` : "") + "</div>";
      html += `<div class="nc-reads${starred ? " starred" : ""}">${values.map(([, r]) => r ? esc(r) : `<span style="color:gray">—</span>`).join(" &nbsp;·&nbsp; ")}</div>`;
      html += `<div class="nc-ars">${arabicSpan(values.map(([a]) => a).join("  ·  "), 0.72)}</div>`;
    }
    html += `</div><div class="caption nc-note">${t(NAME_EXPLAIN)}</div>`;
  }
  return `<div class="ncard namecard">${html}</div>`;
}

function nameLinks(value) {
  const body = value.slice(0, 40).includes(":") ? value.split(":").slice(1).join(":") : value;
  return body.replace(/\.\.\./g, "…").split(/[,،;؛]/).map((p) => p.trim().replace(/^[ .…]+|[ .…]+$/g, "")).filter((p) => p && p.length > 2);
}

// the card of a life (BIO2): written for isnady, or read from the rijal work and the chains
function lifeCard(life, written) {
  let html = `<div class="lc-head"><span class="ncard-title">${esc(t("Life"))}</span>`;
  if (written && life.title) html += `<span class="lc-name">${esc(life.title)}</span>`;
  if (life.kind) html += `<span class="${life.road === "shia" || written ? "tag-gold" : "tag-muted"}">${esc(life.kind)}</span>`;
  html += `</div>`;
  if (life.facts.length) {
    html += `<div class="lc-facts">`;
    life.facts.forEach(([label, value], i) => {
      const key = life.fact_keys[i] || "";
      let text = esc(value), tip = "";
      if ((key === "teachers" || key === "students") && nameLinks(value).length) {
        let linked = value;
        for (const name of nameLinks(value).sort((a, b) => b.length - a.length)) linked = linked.replace(name, `\x00${name}\x01`);
        text = esc(linked).replace(/\x00(.*?)\x01/g, (_m, n) => `<a data-name="${esc(n)}">${n}</a>`);
        tip = ` data-tip="${esc(t("Click a name to look for him among the narrators"))}"`;
      }
      html += `<div class="lc-key">${esc(label)}</div><div class="lc-val"${tip}>${text}</div>`;
    });
    html += `</div>`;
  }
  for (const p of life.paragraphs) html += `<div class="lc-para">${esc(p)}</div>`;
  // the Narrators page opens no book from here: only the pages on the web (life_card without open_book)
  const reads = (life.read || []).filter((r) => r.text && r.url);
  if (reads.length) {
    html += `<div class="lc-more">${esc(t("Read more"))}</div>`;
    for (const r of reads) {
      html += `<div class="lc-read">${r.book ? "📖 " : "↗ "}<a data-book="${esc(r.book || "")}" data-url="${esc(r.url || "")}"${r.url ? ` data-tip="${esc(r.url)}"` : ""}>${esc(r.text)}</a></div>`;
    }
  }
  life.entries.forEach((e, i) => {
    html += `<div class="lc-entry" data-entry="${i}"><div class="lc-entry-head"><b>${esc(t("In the source"))}:</b> ${esc(e.title)}, ${esc(e.where)}</div><div class="lc-entry-body"></div></div>`;
  });
  if (life.sources.length) html += `<div class="caption"><b>${esc(t("Sources"))}:</b> ${life.sources.map(esc).join(" · ")}</div>`;
  return `<div class="ncard lifecard">${html}</div>`;
}

async function fillEntries(life, root) {
  for (const [i, e] of life.entries.entries()) {
    const box = document.querySelector(`${root || "#n-view"} .lc-entry[data-entry="${i}"] .lc-entry-body`);
    if (!box) continue;
    const found = await invoke("source_entry", { work: e.work, volume: e.volume, number: e.number });
    if (!found.heading) {
      if (found.available) { box.innerHTML = `<div class="caption">${esc(t("This entry is not in the downloaded copy."))}</div>`; continue; }
      box.innerHTML = `<div class="caption">${esc(t("Read his entry in the author's own words (Arabic): the work is downloaded once from isnady-data (about {mb} MB) and kept on this computer — or import it in Data sources, and read the whole work in Books.", { mb: found.size_mb }))}</div>
        <button class="nlink">${esc(t("Download {work}", { work: e.title }))}</button>`;
      box.querySelector("button").onclick = later(["&File", "Data Sources…"]);
      continue;
    }
    const whole = found.text;
    const short = whole.length <= 900 ? whole : whole.slice(0, 900).replace(/ [^ ]*$/, "") + " …";
    const words = whole.split(/\s+/).filter(Boolean).length;
    box.innerHTML = `<div class="lc-src-head">${arabicSpan(found.heading, 0.8, "bold")}</div>
      <div class="lc-src-text">${arabicSpan(short, 0.7)}</div>
      ${short !== whole ? `<button class="nlink">${esc(t("Show the whole entry ({n:,} words)", { n: words }))}</button>` : ""}`;
    const more = box.querySelector("button");
    if (more) {
      let full = false;
      more.onclick = () => {
        full = !full;
        box.querySelector(".lc-src-text").innerHTML = arabicSpan(full ? whole : short, 0.7);
        more.textContent = full ? t("Show less") : t("Show the whole entry ({n:,} words)", { n: words });
      };
    }
  }
}

function critics(d) {
  const P = WIN.people;
  const shia = d.tradition === "shia";
  const terms = (WIN.narrator_terms || {})[APP.ui_language] || (WIN.narrator_terms || {}).en || {};
  let html = `<div class="nc-tags"><span class="${shia ? "tag-gold" : "tag-muted"}" data-tip="${esc(t(shia
    ? "A narrator of the Shia rijal works, judged in the Imami terms of his own critics and identified in the chains of the Shia books"
    : "A narrator of the Sunni rijal works, ranked on Ibn Hajar's scale and identified in the chains of the Sunni books"))}">${esc(t(shia ? "Shia narrator" : "Sunni narrator"))}</span>
    <span class="caption">${esc(t("from {source}", { source: d.source || "" }))}</span></div>`;
  if (d.name_ar.trim() !== d.display_name.trim() || shia) {
    html += `<div><button class="nlink" id="n-entry-toggle">${esc(t("Show the entry as the critic wrote it"))}</button></div>
      <div id="n-entry" class="n-entry" hidden>${arabicSpan(d.name_ar, 0.62, "muted")}</div>`;
  }
  const ar = scriptOf("arabic");
  for (const v of d.verdicts) {
    let rank;
    if (v.shia) {
      rank = v.rank ? `&nbsp;&nbsp;<span style="color:${rankColor(SHIA_DOT[v.rank])}">●</span> ${esc(v.label)}`
        : "&nbsp;&nbsp;<i>" + esc(t("no judgment in these words")) + "</i>";
      if (v.madhhab) rank += "&nbsp;·&nbsp;" + esc(t("creed: {creed}", { creed: v.madhhab }));
    } else {
      rank = v.rank ? `&nbsp;&nbsp;<span style="color:${rankColor(v.rank)}">●</span> <span class="dotted">${esc(t("rank {rank} of 12", { rank: v.rank }))}</span>: ${esc(v.label)}` : "";
    }
    let tip = "";
    if (v.shia) {
      const term = { 1: "thiqa", 2: "mamduh", 3: "muwaththaq", 4: "daif_narrator" }[v.rank];
      if (term) tip = (terms[term] || "") + (v.madhhab ? "\n\n" + (terms.imami || "") : "");
    } else if (v.rank) tip = terms.rank || "";
    html += `<div class="n-verdict"${tip ? ` data-tip="${esc(tip)}"` : ""}><b>${esc(v.critic)}</b>, <i>${esc(v.work)}</i>:&nbsp; <span style="font-family:${esc(ar.family)};font-size:${(ar.size * 0.75).toFixed(1)}pt">${esc(v.phrase)}</span>${rank}</div>`;
  }
  if (!d.verdicts.length) html += `<div class="caption">${esc(t("No verdict recorded."))}</div>`;
  const facts = [];
  if (d.tabaqa) facts.push("<b>" + esc(t("Tabaqa {tabaqa}", { tabaqa: d.tabaqa })) + `</b>: ${esc(d.tabaqa_label)}`);
  if (d.death) facts.push(t("<b>Died</b> {year} AH", { year: d.death }));
  else if (d.death_note) facts.push(t("<b>Death</b>: {note}", { note: esc(d.death_note) }));
  if (facts.length) html += `<div class="lead">${facts.join(" &nbsp;·&nbsp; ")}</div>`;
  html += `<div class="caption">${shia
    ? t("<b>How the Imami critics judge</b> — on two axes: <b>reliability</b> (thiqa; praised — jalil, wajh, 'ayn, la ba's bihi; weak) and <b>creed</b> (Imami, or Waqifi, Fathi, Zaydi, 'ammi …). Together they give the classical four: an Imami <b>thiqa</b>, a <b>praised</b> narrator (mamduh), a thiqa of another school (<b>muwaththaq</b>), a <b>weak</b> one. Read from the critic's own words at the head of the entry; a creed he does not state is left unknown. Not Ibn Hajar's scale: each tradition is shown by its own measure.")
    : t("<b>Rank</b> — Ibn Hajar sorts narrators into twelve degrees, from the Companions (1) down to the accused liar (12); 2–3 are trustworthy, 4 truthful, 5–6 acceptable, 7–12 weak to rejected. <b>Tabaqa</b> — the generation: 1 Companions, 2–5 Successors, 6–9 their followers, 10–12 the compilers' teachers.")}</div>`;
  html += `<div><button class="quiet" id="n-in-book">${esc(t("Open his entry in the book"))}</button></div>`;
  if (d.marks.length) {
    html += `<div class="caption">${esc(t("Where his hadith appear (Ibn Hajar's marks)"))}</div><div class="n-marks">` +
      d.marks.map((m) => `<span dir="auto" class="n-mark${m.book ? " go" : ""}"${m.book ? ` data-key="${esc(m.book)}"` : ""} data-tip="${esc(t("Ibn Hajar's mark {mark}: {meaning}", { mark: m.mark, meaning: m.meaning }) + (m.book ? "\n\n" + t("Click to open this book.") : ""))}">${esc(m.mark)}  ${esc(m.meaning)}</span>`).join("") + `</div>`;
  }
  return nCard(t("What the critics say"), html);
}

function relationsCard(d) {
  const shia = d.tradition === "shia";
  let html = `<div class="caption">${esc(t(shia
    ? "Counted from the chains of the Shia books imported here (al-Kafi …), where both narrators are identified."
    : "Counted from the chains imported here, where both narrators are identified. The full lists of al-Mizzi's Tahdhib al-Kamal are a later step."))}</div><div class="n-cols">`;
  for (const [heading, items] of [[t("Narrates from"), d.teachers], [t("Narrated to"), d.students]]) {
    html += `<div class="n-col"><div class="n-flabel">${esc(heading)}</div>` +
      (items.length ? items.map((r) => `<button class="nlink wide" data-person="${r.id}" data-tip="${esc(r.name + "\n" + t("Open this narrator"))}">${esc(r.short)}  (${r.count})</button>`).join("")
        : `<div class="caption">—</div>`) + `</div>`;
  }
  html += `</div>`;
  if (d.compilers.length) {
    html += `<div class="lead">${t("<b>Compilers who narrate from him directly:</b> {parts}", { parts: d.compilers.map((c) => `${esc(c.book)} (${c.count.toLocaleString("en-US")})`).join(" · ") })}</div>`;
  }
  return nCard(t("Teachers and students in the imported chains"), html);
}

function hadithCard(d) {
  let html = "";
  if (d.hadith.length) {
    html += `<div class="n-grid">`;
    let last = null;
    for (const h of d.hadith) {
      if (h.book !== last) {
        html += `<div class="n-flabel n-gbook">${esc(h.book)}</div>`;
        last = h.book;
      }
      html += `<div><button class="nlink left" data-book="${esc(h.key)}" data-number="${esc(h.number)}" data-tip="${esc(`${h.book} ${h.number}\n` + t("Open this chain"))}">${esc(t("hadith {number}", { number: h.number }))}</button></div>`;
    }
    html += `</div>`;
    if (d.hadith_total > d.hadith.length) html += `<div class="caption">${esc(t("… and {value:,} more.", { value: d.hadith_total - d.hadith.length }))}</div>`;
  } else {
    html += `<div class="caption">${esc(t("Not identified in any imported chain yet."))}</div>`;
  }
  return nCard(t("In the chains of {total:,} hadith", { total: d.hadith_total }), html);
}

async function openNarrator(id, quiet) {
  if (!NARR.loaded) await loadNarrators();
  const d = await invoke("narrator_page", { id });
  if (d.error) return;
  NARR.id = id;
  if (!quiet) record(["narrator", id]);
  let html = nameCard(d.view, d.ref);
  const tree = d.life_id && (WIN.life_nodes || {})[d.life_id];
  html += `<div class="n-trees"><span class="caption">🌳 ${esc(t("Genealogies:"))}</span>
    <button class="nlink" data-tree="family" data-tip="${esc(t("His forefathers and descendants among the narrators, read from their names"))}">${esc(t("his family"))}</button>
    <button class="nlink" data-tree="learning" data-tip="${esc(t("His teachers above him and his students below him, from the imported chains"))}">${esc(t("his line of learning"))}</button>
    ${tree ? `<button class="nlink" data-tree="${esc(tree)}" data-tip="${esc(t("The family of Quraysh as the genealogists give it, with him in it"))}">${esc(t("his place in the tree of Quraysh"))}</button>` : ""}</div>`;
  if (d.life && (d.written || d.life.paragraphs.length)) html += lifeCard(d.life, d.written);
  html += critics(d) + relationsCard(d) + hadithCard(d);
  const view = $("n-view");
  view.innerHTML = html;
  view.scrollTop = 0;
  requestAnimationFrame(() => squareBoxes(view, ".tag-gold, .tag-muted, .n-mark"));
  if (d.life) fillEntries(d.life);
  const star = $("n-star");
  if (star && d.ref) {
    const tipOf = (on) => on ? t("In your bookmarks — click to take it out") + `\n${t("Narrator")}: ${d.ref.title}`
      : t("Add this narrator to your bookmarks") + `\n${t("Narrator")}: ${d.ref.title}\n` + t("Bookmarks in the sidebar keeps them, in folders of your own.");
    star.dataset.tip = tipOf(d.ref.marked);
    star.onclick = async () => {
      const on = await invoke("toggle_bookmark", { kind: "narrator", reference: d.ref.key, title: d.ref.title, subtitle: d.ref.arabic });
      star.textContent = on ? "★" : "☆";
      star.classList.toggle("on", on);
      star.dataset.tip = tipOf(on);
    };
  }
  const toggle = $("n-entry-toggle");
  if (toggle) {
    toggle.onclick = () => {
      const e = $("n-entry");
      e.hidden = !e.hidden;
      toggle.textContent = t(e.hidden ? "Show the entry as the critic wrote it" : "Hide the entry");
    };
  }
  $("n-in-book").onclick = () => openInBookOf(id);
}

// a narrator shown from another page (the chain page, Search): in his own tradition, marked in the list
async function selectNarrator(id, road) {
  if (!NARR.loaded) await loadNarrators();
  if (!road) road = ((await invoke("narrator_names", { id })) || {}).tradition;
  if (road && NARR.road && road !== NARR.road) {
    NARR.road = road;
    for (const b of $("n-roads").children) b.classList.toggle("on", b.dataset.road === road);
    NARR.filters = { tabaqa: "", rank: "", book: "" };
    NARR.famous = false;
    NARR.id = id;
    famousText();
    showFilters();
    $("n-clear").hidden = true;
    overview();
    await listNarrators();
  }
  if (NARR.left.some((r) => r.id === id)) addRows(NARR.token, 0);
  markRow(id);
  await openNarrator(id, true);
}

function wireNarratorView() {
  $("page-narrators").addEventListener("click", (e) => {
    const view = e.target.closest("#n-view");
    if (!view) return;
    const p = e.target.closest("[data-person]");
    if (p) { openNarratorPlace(Number(p.dataset.person)); return; }
    const h = e.target.closest("button[data-book]");
    if (h) { showChain(h.dataset.book, h.dataset.number); return; }
    const m = e.target.closest(".n-mark.go");
    if (m) { openBookAt(m.dataset.key); return; }
    const tr = e.target.closest("[data-tree]");
    if (tr) { openGenealogy(NARR.id, tr.dataset.tree); return; }
    const name = e.target.closest("a[data-name]");
    if (name) {
      // a teacher or student named in his life: looked for among the narrators
      NARR.famous = false;
      famousText();
      $("nq").value = name.dataset.name;
      listNarrators();
      return;
    }
    const read = e.target.closest("a[data-url], a[data-book]");
    if (read) {
      if (read.dataset.book) openBookAt(read.dataset.book);
      else if (read.dataset.url) invoke("open_url", { url: read.dataset.url });
    }
  });
}
