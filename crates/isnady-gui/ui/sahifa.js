"use strict";
// Written sources (gui/sahifa_page.py, SHF1): Hammam's sahifa as it stands in the Musnad with its parallels, the
// writings that live in later books, and writing in the chains — the Sunni road and the Shia road each on its own (SH1).
//
// [Written sources]        | the source: its title, its road and layer, what it is, how isnady finds it, as a book
// [Sunni] [Shia]           | in the books imported here: how many, book by book (Hammam's: the parallels)
// the sources of the road  | its hadith as Search shows them, twenty at a time (Hammam's: each with its parallels)

const SH = { built: false, road: "sunni", current: null, ids: [], shown: 0, data: null, seq: 0, counting: null, stale: false };
const SH_PAGE = 20;
const SH_LAYERS = { 1: "Came down whole", 2: "Lives in later books", 3: "Writing in the chains" };

function sahifaBuild() {
  if (SH.built) return;
  SH.built = true;
  const page = $("page-sahifa");
  page.className = "page narrators-page sahifa-page";
  page.innerHTML = `<div class="np">
    <div class="np-left sh-left">
      <div class="np-title">${esc(t("Written sources"))}</div>
      <div class="np-segs" id="sh-segs">${[["sunni", "Sunni"], ["shia", "Shia"]].map(([k, l]) =>
        `<button class="seg" data-road="${k}">${esc(t(l))}</button>`).join("")}</div>
      <div class="caption">${esc(t("Most hadith came down by word of mouth; some were written in the first generations. What was written, and where it lives in the books imported here."))}</div>
      <div class="np-list" id="sh-list"></div>
    </div>
    <div class="np-right" id="sh-view"></div>
  </div>`;
  $("sh-segs").onclick = async (e) => {
    const b = e.target.closest("button[data-road]");
    if (!b) return;
    SH.road = b.dataset.road;
    if (SH.current && SH.data && SH.data.road !== SH.road) SH.current = null;
    await sahifaRefresh();
  };
  $("sh-list").onclick = (e) => {
    const row = e.target.closest("[data-source]");
    if (row) place(["sahifa", row.dataset.source], () => showSource(row.dataset.source));
  };
  $("sh-view").addEventListener("click", (e) => {
    const a = e.target.closest("[data-hadith], [data-act]");
    if (!a) return;
    if (a.dataset.hadith) openInBook(Number(a.dataset.hadith));
    else if (a.dataset.act === "book") openBookAt(SH.data.book);
    else if (a.dataset.act === "sources") openPreferences("sources");
    else if (a.dataset.act === "more") drawMoreSahifa();
  });
}

// count in the background now if the count is not ready (the window calls it a little after it opens), so the page is
// ready before it is opened; the page fills in by itself when it is done
function sahifaWarm() {
  if (SH.counting) return SH.counting;
  SH.counting = invoke("sahifa_count").finally(() => {
    SH.counting = null;
    if (CURRENT === "sahifa") sahifaRefresh(); else SH.stale = true;
  });
  return SH.counting;
}

function sahifaRow(s) {
  const n = s.count == null ? "…" : t("{n:,} hadith", { n: s.count });
  return `<div class="sh-row" data-source="${esc(s.id)}"><div class="sh-name">${esc(s.title)}</div>
    <div class="sh-sub">${esc(t(SH_LAYERS[s.layer]))} · ${esc(n)}</div></div>`;
}

async function sahifaRefresh() {
  sahifaBuild();
  SH.stale = false;
  for (const b of $("sh-segs").children) b.classList.toggle("on", b.dataset.road === SH.road);
  const list = await invoke("sahifa_list", { road: SH.road });
  $("sh-list").innerHTML = list.map(sahifaRow).join("");
  if (list.some((s) => s.count == null)) {
    sahifaWaiting();
    sahifaWarm();
    return;
  }
  const id = list.some((s) => s.id === SH.current) ? SH.current : (list[0] || {}).id;
  if (id) await showSource(id);
}

function sahifaWaiting() {
  ++SH.seq;
  $("sh-view").innerHTML = `<div class="ncard">
    <div class="ncard-title">${esc(t("Counting the written sources in the books imported here…"))}</div>
    <div class="caption">${esc(t("Once, after the books change; the window can be used meanwhile, and this page fills in by itself."))}</div>
    <div class="sh-busy"><div></div></div></div>`;
}

// a source opened from elsewhere (Back, the start): its road, it chosen
async function openSahifaAt(id) {
  show("sahifa");
  sahifaBuild();
  const d = await invoke("sahifa_source", { id });
  if (d.error) return;
  SH.current = id;
  SH.road = d.road;
  await sahifaRefresh();
}

function openSahifa(id) {
  return place(["sahifa", id], () => openSahifaAt(id));
}

async function showSource(id) {
  const seq = ++SH.seq;
  const d = await invoke("sahifa_source", { id });
  if (seq !== SH.seq || d.error) return;
  SH.current = id;
  for (const r of $("sh-list").children) r.classList.toggle("on", r.dataset.source === id);
  if (d.waiting) {
    sahifaWaiting();
    sahifaWarm();
    return;
  }
  SH.data = d;
  const shia = d.road === "shia";
  let html = `<div class="ncard sh-card"><div class="sh-title">${esc(d.title)}</div>
    <div class="nc-tags"><span class="${shia ? "tag-gold" : "tag-muted"}">${esc(t(shia ? "Shia road" : "Sunni road"))}</span><span class="tag-lapis">${esc(t(SH_LAYERS[d.layer]))}</span></div>
    <div class="lead">${esc(d.lead)}</div>
    <div class="caption">🔎 ${esc(d.how)}</div>
    <div><button class="quiet" data-act="${d.as_book ? "book" : "sources"}">📚 ${esc(t(d.as_book ? "Read it as a book in Books" : "Import it as a book (Data sources)"))}</button></div></div>`;
  let c = `<div class="sh-ctitle">${esc(t("In the books imported here: {n:,} hadith", { n: d.ids.length }))}</div>`;
  if (d.books.length) {
    c += `<div class="lead">${d.books.map(([name, n]) => `<b>${esc(name)}</b> ${n.toLocaleString("en")}`).join(" · ")}</div>`;
  } else if (id === "hammam") {
    c += `<div class="caption">${esc(t("Not found: import Musnad Ahmad ibn Hanbal (File → Data Sources) — the sahifa is read from it."))}</div>`;
  } else {
    c += `<div class="caption">${esc(t("None in the books imported here."))}</div>`;
  }
  if (id === "hammam" && d.ids.length) {
    const h = d.hammam;
    c += `<div class="caption">${esc(t("{with_} of the {n} have a parallel in the other books through Hammam's chain — {count:,} hadith:", { with_: h.with, n: d.ids.length, count: h.count }))} ${h.books.map(([name, n]) => `${esc(name)} ${n.toLocaleString("en")}`).join(" · ")}</div>`;
  }
  html += `<div class="ncard sh-card">${c}</div>`;
  const view = $("sh-view");
  view.innerHTML = html;
  view.scrollTop = 0;
  SH.ids = d.ids;
  SH.shown = 0;
  await drawMoreSahifa();
}

async function drawMoreSahifa() {
  const seq = SH.seq;
  const view = $("sh-view");
  const more = view.querySelector(".sh-more");
  if (more) more.remove();
  const part = SH.ids.slice(SH.shown, SH.shown + SH_PAGE);
  const cards = await invoke("hadith_cards", { ids: part });
  if (seq !== SH.seq) return;
  const hammam = SH.current === "hammam";
  const byId = new Map(cards.map((r) => [r.hadith_id, r]));
  part.forEach((hid, k) => {
    if (hammam) {
      const found = SH.data.hammam.parallels[String(hid)] || [];
      let line = t("Hadith {i} of the sahifa", { i: SH.shown + k + 1 });
      line = esc(line) + " · " + (found.length
        ? esc(t("its parallels:")) + " " + found.map(([h, name, n]) => `<a data-hadith="${h}">${esc(name)} ${esc(String(n))}</a>`).join(" · ")
        : esc(t("no parallel through Hammam found in the other books imported here")));
      const note = document.createElement("div");
      note.className = "caption sh-note";
      note.innerHTML = "📜 " + line;
      view.appendChild(note);
    }
    const r = byId.get(hid);
    if (r) view.appendChild(card(r));
  });
  SH.shown = Math.min(SH.ids.length, SH.shown + SH_PAGE);
  const left = SH.ids.length - SH.shown;
  if (left > 0) {
    const b = document.createElement("div");
    b.className = "sh-more";
    b.innerHTML = `<button class="quiet" data-act="more">${esc(t("Show {page} more ({left:,} left)", { page: Math.min(SH_PAGE, left), left }))}</button>`;
    view.appendChild(b);
  }
}
