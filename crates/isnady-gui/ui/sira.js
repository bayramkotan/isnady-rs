"use strict";
// The Sira as a timeline (gui/sira_page.py, SIY1): the life of the Prophet ﷺ from the Year of the Elephant to his
// death, drawn on one line in its three periods; each event with its date (and where the reports differ), its place,
// the verses revealed about it, the chapter of Ibn Hisham's Sira that tells it, and the hadith that name it.
//
// [Sira — the life of the Prophet ﷺ]  ·  the timeline: the periods as bands, every event a dot
// the events by period               |  the event: title, chips, text · the verses · in the sources · ‹ ›

const SR = { built: false, current: null, data: null, seq: 0 };
const SR_FIRST = -53, SR_LAST = 11;   // the Year of the Elephant … his death (years after the Hijra)
const SR_COLORS = { before: "var(--muted)", mecca: "var(--gold)", medina: "var(--lapis)" };

function siraBuild() {
  if (SR.built) return;
  SR.built = true;
  const page = $("page-sira");
  page.className = "page sira-page";
  page.innerHTML = `<div class="sr">
    <div class="ncard sr-head">
      <div class="np-title">${esc(t("Sira — the life of the Prophet ﷺ"))}</div>
      <div class="caption">${esc(t("From the Year of the Elephant to his death, as Ibn Ishaq tells it in Ibn Hisham's edition; where others give another date, it is said beside it. Click an event on the line or in the list."))}</div>
      <div class="sr-line" id="sr-line"></div>
    </div>
    <div class="sr-body">
      <div class="sr-left"><div class="np-list" id="sr-list"></div></div>
      <div class="np-right" id="sr-view"></div>
    </div>
  </div>`;
  $("sr-list").onclick = (e) => {
    const row = e.target.closest("[data-event]");
    if (row) pickEvent(row.dataset.event);
  };
  $("sr-line").onclick = (e) => {
    const dot = e.target.closest("[data-event]");
    if (dot) pickEvent(dot.dataset.event);
  };
  $("sr-view").addEventListener("click", (e) => {
    const b = e.target.closest("[data-step], [data-chapter], [data-phrase], [data-sura]");
    if (!b || b.disabled) return;
    if (b.dataset.step) pickEvent(b.dataset.step);
    else if (b.dataset.chapter) {
      const id = Number(b.dataset.chapter);
      place(["chapter", id], () => openChapterAt(id));
    } else if (b.dataset.phrase) searchPhrase(b.dataset.phrase);
    else openVerse(Number(b.dataset.sura), Number(b.dataset.aya));
  });
  new ResizeObserver(() => drawTimeline()).observe($("sr-line"));
}

// Sira → the hadith that name an event: the Search page with that phrase, the words in that order
function searchPhrase(phrase) {
  SEARCH.mode = "phrase";
  show("search");
  labelCombos();
  searchFor(phrase, "hadith");
}

// the whole life on one line: the three periods as bands, every event a dot (stacked when in the same year)
function drawTimeline() {
  const box = $("sr-line");
  if (!box || !SR.data || !box.clientWidth) return;
  const w = box.clientWidth, lineY = 62;
  const x = (year) => 28 + (year - SR_FIRST) / (SR_LAST - SR_FIRST) * (w - 56);
  const names = Object.fromEntries(SR.data.periods.map((p) => [p.key, p.name]));
  let svg = `<svg width="${w}" height="118" viewBox="0 0 ${w} 118">`;
  for (const [period, a, b] of [["before", SR_FIRST, -13], ["mecca", -13, 0], ["medina", 0, SR_LAST]]) {
    svg += `<rect x="${x(a)}" y="${lineY - 5}" width="${x(b) - x(a)}" height="10" rx="5" fill="${SR_COLORS[period]}" fill-opacity="0.24"/>`;
    svg += `<text x="${(x(a) + x(b)) / 2}" y="${lineY + 42}" class="sr-period">${esc(names[period])}</text>`;
  }
  for (const [year, label] of [[SR_FIRST, "570"], [-13, "610"], [0, t("Hijra") + " 622"], [SR_LAST, "632"]]) {
    svg += `<line x1="${x(year)}" y1="${lineY + 7}" x2="${x(year)}" y2="${lineY + 13}" class="sr-tick"/>`;
    svg += `<text x="${x(year)}" y="${lineY + 26}" class="sr-mark">${esc(label)}</text>`;
  }
  const stack = {};
  let current = null;
  for (const e of SR.data.events) {
    const k = stack[e.year] || 0;
    stack[e.year] = k + 1;
    const cx = x(e.year), cy = lineY - 14 - k * 11, big = e.id === SR.current;
    if (big) current = e;
    svg += `<circle data-event="${esc(e.id)}" cx="${cx}" cy="${cy}" r="${big ? 6.5 : 4.5}" fill="${SR_COLORS[e.period]}" class="sr-dot${big ? " on" : ""}"><title>${esc(e.title + "\n" + e.year_label)}</title></circle>`;
  }
  if (current) {
    const cx = Math.min(Math.max(134, x(current.year)), w - 134);
    svg += `<text x="${cx}" y="12" class="sr-current">${esc(current.title)}</text>`;
  }
  box.innerHTML = svg + "</svg>";
}

async function siraRefresh() {
  siraBuild();
  SR.data = await invoke("sira_events");
  let html = "";
  for (const p of SR.data.periods) {
    html += `<div class="sr-period-head" style="color:${SR_COLORS[p.key]}">${esc(p.name)}</div>`;
    for (const e of SR.data.events.filter((x) => x.period === p.key)) {
      html += `<div class="sr-row" data-event="${esc(e.id)}"><div>${esc(e.title)}</div><div>${esc(e.year_label)}</div></div>`;
    }
  }
  $("sr-list").innerHTML = html;
  const ids = SR.data.events.map((e) => e.id);
  await showEvent(ids.includes(SR.current) ? SR.current : ids[0]);
}

// an event picked on the line, in the list or by ‹ ›: shown, and a place for Back and Forward (NAV2b)
function pickEvent(id) {
  return place(["sira", id], () => showEvent(id));
}

// an event opened from elsewhere (Back, the start)
async function openEventAt(id) {
  SR.current = id;
  show("sira");
  if (!SR.data) await siraRefresh();
  else await showEvent(id);
}

async function showEvent(id) {
  const seq = ++SR.seq;
  const e = await invoke("sira_event", { id });
  if (seq !== SR.seq || e.error) return;
  SR.current = id;
  drawTimeline();
  const list = $("sr-list");
  for (const r of list.querySelectorAll(".sr-row")) {
    const on = r.dataset.event === id;
    r.classList.toggle("on", on);
    if (on) {
      // QListWidget.scrollToItem: just enough to show the row
      const top = r.offsetTop - list.offsetTop, bottom = top + r.offsetHeight;
      if (top < list.scrollTop) list.scrollTop = top;
      else if (bottom > list.scrollTop + list.clientHeight) list.scrollTop = bottom - list.clientHeight;
    }
  }
  const chips = [
    `<span class="tag-lapis">${esc(e.period_name)}</span>`,
    `<span class="tag-muted">📅 ${esc(e.year_label)}</span>`,
    `<span class="tag-muted">🕰 ${esc(e.when)}</span>`,
    `<span class="tag-muted">📍 ${esc(e.place)}</span>`,
    `<span class="tag-muted">👤 ${esc(t("his age: {n}", { n: e.age }))}</span>`,
  ];
  let html = `<div class="ncard sr-card">
    <div class="sr-top"><div class="sr-title">${esc(e.title)}</div><div class="sr-ar">${arabicSpan(e.arabic, 0.85)}</div></div>
    <div class="sr-chips">${chips.join("")}</div>
    <div class="sr-stripe" style="background:${SR_COLORS[e.period]}"></div>
    <div class="lead">${esc(e.text)}</div>
    ${e.differ ? `<div class="caption sr-differ">⚖ ${esc(e.differ)}</div>` : ""}
  </div>`;
  if (e.quran.length) {
    html += `<div class="ncard sr-card"><div class="sr-ctitle">📖 ${esc(t("The verses revealed about it"))}</div>
      <div class="sr-chips">${e.quran.map((q) => `<button class="quiet" data-sura="${q.sura}" data-aya="${q.aya}">${esc(q.label)}</button>`).join("")}</div></div>`;
  }
  let src = `<div class="sr-ctitle">📚 ${esc(t("In the sources"))}</div>`;
  if (e.chapter) {
    src += `<div><button class="linkbtn" data-chapter="${e.chapter}">🕌 ${esc(t("Read it in Ibn Hisham's Sira"))} — ${esc(e.sira)}</button></div>`;
  } else if (e.sira) {
    src += `<div class="caption">${esc(t("Ibn Hisham's Sira tells it in “{chapter}” — import the Sira in File → Data Sources to read it here.", { chapter: e.sira }))}</div>`;
  }
  if (e.phrase && e.count !== null) {
    src += `<div><button class="linkbtn" data-phrase="${esc(e.phrase)}"${e.count > 0 ? "" : " disabled"}>📜 ${esc(t("The hadith that say “{phrase}”: about {n:,} in the Sunni books", { phrase: e.phrase, n: e.count }))}</button></div>`;
  }
  html += `<div class="ncard sr-card">${src}</div>`;
  const ids = SR.data.events.map((x) => x.id);
  const k = ids.indexOf(id);
  const prev = ids[k - 1], next = ids[k + 1];
  html += `<div class="sr-nav">
    <button class="quiet"${prev ? ` data-step="${esc(prev)}"` : " disabled"}>‹  ${esc(t("Previous event"))}</button>
    <button class="quiet"${next ? ` data-step="${esc(next)}"` : " disabled"}>${esc(t("Next event"))}  ›</button></div>`;
  const view = $("sr-view");
  view.innerHTML = html;
  view.scrollTop = 0;
}
