"use strict";
// Books (gui/books_page.py, BR1): open any imported work and read it in its own order — a hadith collection chapter
// by chapter, the Taqrib letter by letter, the sira and the tabaqat. Each hadith keeps its card (search.js card);
// each narrator's entry opens the narrator.

const BK = {
  built: false, refreshing: null, shelf: "sunni", works: [], quran: null, work: null, chapter: null, order: [],
  items: [], shown: 0, token: 0, focus: null, nav: null, languages: [], readingTimer: null, findTimer: null,
};
const BK_BATCH = 12, BK_FIRST = 4, BK_PAGE = 60;
const LIBRARY = ["sira", "tabaqat"];

function booksBuild() {
  if (BK.built) return;
  BK.built = true;
  const page = $("page-books");
  page.className = "page books-page";
  page.innerHTML = `<div class="bp">
    <div class="bp-left">
      <div class="np-title">${esc(t("Books"))}</div>
      <div class="caption">${esc(t("Open a book and read it in its own order, chapter by chapter."))}</div>
      <div class="np-segs" id="b-roads"></div>
      <div class="bp-works" id="b-works"></div>
      <div class="flabel">${esc(t("Contents"))}</div>
      <div class="bp-tree" id="b-tree"></div>
    </div>
    <div class="bp-right" id="b-right">
      <div class="bp-where-row">
        <div class="lead bp-where" id="b-where"></div>
        <span id="b-star"></span>
        <button class="quiet tool" id="b-marks" data-tip="${esc(t("Put a marker where you are reading, go to one of your markers, or to where you left each book"))}">🔖 ${esc(t("Markers"))}<i class="tri"></i></button>
        <button class="quiet tool" id="b-search" data-tip="${esc(t("Search only in the chapter you are reading and the chapters under it"))}">🔎 ${esc(t("Search in this chapter"))}</button>
        <button class="quiet tool" id="b-pdf" data-tip="${esc(t("This book, or the chapter you are reading, as a PDF in the languages you choose (or Word, EPUB, HTML …)"))}">⤓ ${esc(t("PDF"))}</button>
      </div>
      <div class="hnav bp-chapters">
        <span class="flabel">${esc(t("Chapter"))}</span>
        <input type="number" class="numfield bp-spin" id="b-spin" min="1" data-tip="${esc(t("Type a chapter's place in the book and press Enter"))}">
        <span class="flabel" id="b-count"></span>
        <button class="quiet" id="b-first" data-tip="${esc(t("The first chapter"))}">⏮</button>
        <button class="quiet" id="b-prev">‹  ${esc(t("Previous chapter"))}</button>
        <button class="quiet" id="b-next">${esc(t("Next chapter"))}  ›</button>
        <button class="quiet" id="b-last" data-tip="${esc(t("The last chapter"))}">⏭</button>
      </div>
      <div id="b-nav"></div>
      <div class="bp-bar"><div class="bp-langs" id="b-langs"></div><span class="grow"></span>
        <div class="bp-find"><input type="search" id="b-find" placeholder="${esc(t("Find in this chapter"))}" autocomplete="off" spellcheck="false"></div></div>
      <div class="bp-body" id="b-body"></div>
    </div>
    <div class="bp-right" id="b-quran" hidden></div>
  </div>`;
  const roads = [["", "All"], ["sunni", "Sunni"], ["shia", "Shia"]];
  $("b-roads").innerHTML = roads.map(([k, l]) => `<button class="seg" data-road="${k}">${esc(t(l))}</button>`).join("");
  $("b-roads").onclick = (e) => {
    const b = e.target.closest("button[data-road]");
    if (b) setShelf(b.dataset.road || null);
  };
  BK.nav = hadithNavigator($("b-nav"), t("Show"), hadithTyped);
  $("b-works").addEventListener("click", (e) => {
    const row = e.target.closest(".bw[data-key]");
    if (row) pickWork(row.dataset.key);
  });
  $("b-works").addEventListener("contextmenu", (e) => {
    e.preventDefault();
    const row = e.target.closest(".bw[data-key]");
    if (row) workMenu(e, row.dataset.key);
  });
  $("b-tree").addEventListener("click", (e) => {
    const arrow = e.target.closest(".pd-arrow");
    if (arrow) { arrow.closest(".pd-node").classList.toggle("open"); return; }
    const item = e.target.closest(".pd-item");
    if (item && item.dataset.sura) quranGo(Number(item.dataset.sura));
    else if (item && item.dataset.id) pickChapter(Number(item.dataset.id));
  });
  $("b-tree").addEventListener("contextmenu", (e) => {
    e.preventDefault();
    const item = e.target.closest(".pd-item");
    if (item) chapterMenu(e, item);
  });
  $("b-first").onclick = () => BK.order.length && openChapter(BK.order[0]);
  $("b-last").onclick = () => BK.order.length && openChapter(BK.order[BK.order.length - 1]);
  $("b-prev").onclick = () => stepChapter(-1);
  $("b-next").onclick = () => stepChapter(1);
  $("b-spin").addEventListener("change", () => {
    const place = Number($("b-spin").value);
    if (place > 0 && place <= BK.order.length && BK.order[place - 1] !== BK.chapter) openChapter(BK.order[place - 1]);
  });
  $("b-find").addEventListener("input", () => {
    clearTimeout(BK.findTimer);
    BK.findTimer = setTimeout(() => BK.chapter && openChapter(BK.chapter, { keepFind: true }), 300);
  });
  $("b-marks").onclick = marksMenu;
  $("b-search").onclick = searchHere;
  $("b-pdf").onclick = later(["&File", "Export…"]);
  $("b-body").addEventListener("scroll", () => {
    clearTimeout(BK.readingTimer);
    BK.readingTimer = setTimeout(rememberReading, 700);
  });
  $("b-body").addEventListener("click", (e) => {
    const go = e.target.closest("[data-narrator]");
    if (go) openNarratorFromBook(Number(go.dataset.narrator));
    const more = e.target.closest("#b-more");
    if (more) moreItems();
    const earlier = e.target.closest("#b-earlier");
    if (earlier) openChapter(BK.chapter, { keepFind: true });
  });
}

// the shelf drawn (BooksPage.refresh), once, whoever asks first; then the book read last (or the one asked for)
function booksRefresh(keepOpen) {
  booksBuild();
  if (!BK.refreshing || keepOpen) {
    BK.refreshing = (async () => {
      const d = await invoke("books_shelf");
      BK.works = d.works || [];
      BK.quran = d.quran;
      BK.lastWork = d.last_work;
      drawShelf();
      if (keepOpen) return;
      const keys = shelfWorks().map((w) => w.key).concat(["quran"]);
      if (!shelfWorks().length) {
        $("b-where").textContent = "";
        $("b-body").innerHTML = `<div class="lead">${esc(t("No books yet: import a collection (File → Data Sources)."))}</div>`;
        return;
      }
      const wanted = (BK.work && BK.work.key) || BK.lastWork || keys[0];
      if (!BK.opening) await selectWork(keys.includes(wanted) ? wanted : keys[0]);
    })();
  }
  return BK.refreshing;
}

function shelfWorks() {
  return BK.works.filter((w) => LIBRARY.includes(w.kind) || !BK.shelf || w.tradition === BK.shelf);
}

function shelfGroup(w) {
  if (LIBRARY.includes(w.kind)) return "library";
  return w.kind === "sahifa" ? `${w.tradition}-sahifa` : w.tradition;
}

function drawShelf() {
  for (const b of $("b-roads").children) b.classList.toggle("on", (b.dataset.road || null) === BK.shelf);
  const order = ["library", "sunni", "sunni-sahifa", "shia", "shia-sahifa"];
  const shelf = shelfWorks().slice().sort((a, b) => {
    const ga = order.indexOf(shelfGroup(a)), gb = order.indexOf(shelfGroup(b));
    return (ga < 0 ? order.length : ga) - (gb < 0 ? order.length : gb);
  });
  const heads = { sunni: "Sunni books · {count}", shia: "Shia books · {count}", library: "Sira and tabaqat · {count}",
    "sunni-sahifa": "Sunni written sources · {count}", "shia-sahifa": "Shia written sources · {count}" };
  let html = `<div class="bw-head">${esc(t("The Qur'an"))}</div>`;
  const q = BK.quran || {};
  html += `<div class="bw" data-key="quran"><div class="bw-title">${esc(t("The Holy Qur'an"))}   القرآن الكريم</div>
    <div class="caption">${esc(q.available ? t("114 suras · 6,236 verses · {n} translations", { n: q.translations }) : t("Not downloaded yet: click to download it (about 3 MB)"))}</div></div>`;
  let heading = null;
  for (const w of shelf) {
    const g = shelfGroup(w);
    if (g !== heading) {
      heading = g;
      const count = shelf.filter((x) => shelfGroup(x) === g).length;
      html += `<div class="bw-head${g.startsWith("shia") ? " gold" : ""}">${esc(t(heads[g] || "Other books · {count}", { count }))}</div>`;
    }
    const leaves = ["hadith", "sahifa"].includes(w.kind) ? t("{n:,} hadith", { n: w.leaves })
      : w.kind === "sira" ? t("{n:,} sections", { n: w.leaves }) : t("{n:,} entries", { n: w.leaves });
    const facts = [esc(t(w.kind_label)), esc(leaves), esc(t("{n} chapters", { n: w.chapters }))];
    if (w.author) facts.unshift(esc(w.author));
    if (w.tradition === "shia" && !LIBRARY.includes(w.kind)) facts.unshift(`<span class="gold"><b>${esc(t("Shia"))}</b></span>`);
    let left = "";
    if (w.last && (w.last.number || w.last.place)) {
      const where = w.last.number ? t("hadith {number}", { number: w.last.number }) : t("chapter {position}", { position: w.last.place });
      left = `<div class="caption">🔖 ${esc(t("You left off at {where}", { where }))}</div>`;
    }
    html += `<div class="bw" data-key="${esc(w.key)}"><div class="bw-title">${esc(w.title)}</div><div class="caption">${facts.join(" · ")}</div>${left}</div>`;
  }
  $("b-works").innerHTML = html;
  markWork();
}

// the row of the book being read (setCurrentRow): marked, and scrolled to
function markWork() {
  const key = BK.quranMode ? "quran" : BK.work && BK.work.key;
  for (const r of $("b-works").querySelectorAll(".bw")) {
    const on = r.dataset.key === key;
    r.classList.toggle("on", on);
    if (on) r.scrollIntoView({ block: "nearest" });
  }
}

function setShelf(tradition) {
  BK.shelf = tradition;
  drawShelf();
  const keys = shelfWorks().map((w) => w.key);
  if (!BK.work || !keys.includes(BK.work.key)) selectWork(keys[0] || "quran");
}

async function selectWork(key, chapter, focus) {
  if (key === "quran") {
    openQuran();
    return;
  }
  const w = BK.works.find((x) => x.key === key);
  if (w && BK.shelf && w.tradition !== BK.shelf && !LIBRARY.includes(w.kind)) {
    BK.shelf = w.tradition;                 // a book of the other tradition was asked for: its shelf
    drawShelf();
  }
  await openWork(key, chapter, focus);
}

// a book picked in the list: opened, and a place in Back and Forward (visited_book)
function pickWork(key) {
  if (key === "quran") { openQuran(); record(["book", "quran"]); return; }
  openWork(key);
  record(["book", key]);
}

async function openWork(key, chapter, focus) {
  const d = await invoke("book_open", { key });
  if (!d) return;
  BK.quranMode = false;
  $("b-right").hidden = false;
  $("b-quran").hidden = true;
  BK.work = d;
  BK.order = d.order;
  markWork();
  $("b-spin").max = Math.max(1, d.order.length);
  $("b-count").textContent = "/ " + thousands(d.order.length);
  $("b-count").dataset.tip = t("{n} chapters in this book", { n: thousands(d.order.length) });
  const hadith = d.kind === "hadith";
  $("b-nav").hidden = !hadith;
  $("b-pdf").hidden = !hadith;
  $("b-search").hidden = !hadith;
  await BK.nav.setBook(hadith ? d.key : null);
  const arabic = /[؀-ۿ]/;
  const ar = scriptOf("arabic");
  const add = (nodes, depth) => nodes.map((c) => {
    const text = c.label + (c.leaves ? `   (${c.leaves})` : "");
    const style = arabic.test(c.label) ? ` style="font-family:${esc(ar.family)};font-size:calc(${(ar.size * 0.62).toFixed(2)}pt * var(--scale))"` : "";
    const kids = c.children.length ? `<div class="pd-kids">${add(c.children, depth + 1)}</div>` : "";
    return `<div class="pd-node"><div class="pd-item" data-id="${c.leaves ? c.id : ""}" data-node="${c.id}" style="padding-inline-start:${depth * 20 + 25}px">${c.children.length ? '<span class="pd-arrow"></span>' : ""}<span${style}>${esc(text)}</span></div>${kids}</div>`;
  }).join("");
  $("b-tree").innerHTML = add(d.tree, 0);
  BK.languages = d.languages;
  $("b-langs").innerHTML = d.languages.map((l) => checkRow(t(l.name), l.checked, `data-lang="${esc(l.name)}"`)).join("");
  for (const box of $("b-langs").querySelectorAll("input")) {
    box.onchange = () => {
      const chosen = [...$("b-langs").querySelectorAll("label.check")].filter((l) => l.querySelector("input").checked).map((l) => l.dataset.lang);
      invoke("book_languages", { key: d.key, languages: chosen });
      if (BK.chapter) openChapter(BK.chapter, { keepFind: true });
    };
  }
  if (chapter) { await openChapter(chapter, focus || {}); return; }
  if (d.start.chapter) await openChapter(d.start.chapter, { hadith: d.start.hadith });
}

function chosenLanguages() {
  return [...$("b-langs").querySelectorAll("label.check")].filter((l) => l.querySelector("input").checked).map((l) => l.dataset.lang);
}

// a chapter opened (open_chapter): where it stands, its ☆, the number shown, its items 60 at a time
async function openChapter(chapterId, o) {
  o = o || {};
  if (!BK.work) return;
  if (!o.keepFind && $("b-find").value) $("b-find").value = "";
  BK.chapter = chapterId;
  const d = await invoke("book_chapter", { key: BK.work.key, chapter: chapterId, find: $("b-find").value, focusHadith: o.hadith || null });
  if (!d || BK.chapter !== chapterId) return;
  const crumbs = d.crumbs;
  $("b-where").innerHTML = crumbs.map((c, i) => i === crumbs.length - 1 ? `<b>${esc(c)}</b>` : esc(c)).join(" › ")
    + (d.position ? `&nbsp;&nbsp;<span class="muted">${esc(t("chapter {position} of {count}", { position: d.position, count: d.count }))}</span>` : "");
  const pos = d.position || 0;
  $("b-prev").disabled = pos <= 1;
  $("b-first").disabled = pos <= 1;
  $("b-next").disabled = !(pos > 0 && pos < BK.order.length);
  $("b-last").disabled = !(pos > 0 && pos < BK.order.length);
  if (pos) $("b-spin").value = pos;
  const star = $("b-star");
  star.innerHTML = "";
  if (d.star) {
    star.innerHTML = `<button class="star${d.star.marked ? " on" : ""}"></button>`;
    wireStar(star.firstChild, "chapter", d.star.ref, d.star.title, d.star.subtitle);
  }
  selectInTree(chapterId);
  if (BK.work.kind === "hadith") BK.nav.setNumber(d.number || "");
  BK.items = d.items;
  BK.token += 1;
  const body = $("b-body");
  const words = $("b-find").value.trim();
  const note = words ? t("{n:,} of {total:,} match “{words}”", { n: d.items.length, total: d.total, words: $("b-find").value })
    : t("{total:,} in this chapter", { total: d.total });
  body.innerHTML = `<div class="caption">${esc(note)}</div>`;
  BK.shown = 0;
  BK.focus = o.hadith ? ["hadith", o.hadith] : o.person ? ["person", o.person] : null;
  if (BK.focus) {
    // start a little before that item, not at the top
    const [field, value] = BK.focus;
    const at = BK.items.findIndex((it) => it[`${field}_id`] === value);
    if (at > 3) {
      BK.shown = at - 2;
      body.insertAdjacentHTML("beforeend", `<div class="bp-center"><button class="quiet" id="b-earlier">${esc(t("Show the {shown:,} earlier items in this chapter", { shown: BK.shown }))}</button></div>`);
    }
  }
  body.scrollTop = 0;
  await moreItems();
}

async function moreItems() {
  BK.token += 1;
  const token = BK.token;
  const more = $("b-more-row");
  if (more) more.remove();
  const end = Math.min(BK.items.length, BK.shown + BK_PAGE);
  let batch = BK_FIRST;
  while (BK.shown < end) {
    const stop = Math.min(end, BK.shown + batch);
    const slice = BK.items.slice(BK.shown, stop);
    const ids = slice.filter((it) => it.kind === "hadith").map((it) => it.hadith_id);
    const cards = ids.length ? await invoke("book_cards", { ids, languages: chosenLanguages() }) : [];
    if (token !== BK.token) return;               // another chapter replaced this one
    const body = $("b-body");
    let c = 0;
    for (const it of slice) body.appendChild(it.kind === "hadith" ? bookCard(cards[c++]) : itemCard(it));
    BK.shown = stop;
    batch = BK_BATCH;
  }
  if (BK.focus) {
    // the cards above take their final height a while after they are drawn (the chains' rows, the fonts): the item
    // is kept at the top for three seconds, or until the reader scrolls (_scroll_to_focus, twice in the Python page)
    const focus = BK.focus;
    const body = $("b-body");
    const find = () => body.querySelector(focus[0] === "hadith" ? `[data-hadith="${focus[1]}"]` : `[data-person="${focus[1]}"]`);
    const el = find();
    if (el) {
      el.classList.add("focus");
      setTimeout(() => el.classList.remove("focus"), 2500);
      let wanted = Math.max(0, el.offsetTop - 12);
      body.scrollTop = wanted;
      const started = Date.now();
      const keep = () => {
        if (BK.focus !== focus || Date.now() - started > 3000) { if (BK.focus === focus) BK.focus = null; return; }
        if (Math.abs(body.scrollTop - wanted) > 2) { BK.focus = null; return; }      // the reader scrolled
        wanted = Math.max(0, el.offsetTop - 12);
        body.scrollTop = wanted;
        setTimeout(keep, 80);
      };
      setTimeout(keep, 80);
    } else BK.focus = null;
  }
  const left = BK.items.length - BK.shown;
  if (left > 0) {
    $("b-body").insertAdjacentHTML("beforeend", `<div class="bp-center" id="b-more-row"><button class="quiet" id="b-more">${esc(t("Show {page} more ({left:,} left in this chapter)", { page: Math.min(BK_PAGE, left), left }))}</button></div>`);
  }
}

function bookCard(r) {
  const el = card(r);
  el.dataset.hadith = r.hadith_id;
  return el;
}

// a section of the sira, a life of the tabaqat, a rijal entry, a cross-reference (_item_widget)
function itemCard(it) {
  const el = document.createElement("div");
  el.className = "c-card bp-item";
  if (it.person_id) el.dataset.person = it.person_id;
  const tb = (text, factor, bold) => {
    const s = scriptOf("arabic");
    const lh = naturalLine(s.family, "قال").toFixed(3);
    return `<div class="tb rtl" dir="rtl" style="${esc(`font-family:${s.family};font-size:calc(${(s.size * factor).toFixed(2)}pt * var(--scale));line-height:${lh};color:${s.color}`)}">${bold ? `<b>${esc(text)}</b>` : esc(text)}</div>`;
  };
  if (it.kind === "reference") {
    el.innerHTML = `<div class="caption">${esc(t("Cross-reference"))}</div>${tb(it.text || "", 0.7)}`;
    return el;
  }
  if (it.kind === "paragraph" || (it.kind === "entry" && !it.person_id)) {
    let html = "";
    if (it.title || it.label) {
      html += `<div class="bp-top">${it.label ? `<span class="c-number">#${esc(it.label)}</span>` : ""}${it.title ? `<div class="grow">${tb(it.title, 0.95, true)}</div>` : '<span class="grow"></span>'}</div>`;
    }
    el.innerHTML = html + tb(it.text || "", 0.85);
    return el;
  }
  let head = `<div class="bp-top"><span class="c-number">${it.label ? "#" + esc(it.label) : ""}</span>`;
  if (it.person_id) {
    if (it.reading) head += `<span class="bp-latin">${esc(it.reading)}</span>`;
    head += `<span class="grow"></span><button class="qlink" data-narrator="${it.person_id}">${esc(t("Open narrator"))}</button>`;
  } else head += '<span class="grow"></span>';
  el.innerHTML = head + "</div>" + tb(it.text || it.title || "", 0.85);
  return el;
}

function selectInTree(chapterId) {
  const tree = $("b-tree");
  for (const x of tree.querySelectorAll(".pd-item.on")) x.classList.remove("on");
  const item = tree.querySelector(`.pd-item[data-node="${chapterId}"]`);
  if (!item) return;
  for (let n = item.parentElement.parentElement.closest(".pd-node"); n; n = n.parentElement.closest(".pd-node")) n.classList.add("open");
  item.classList.add("on");
  item.scrollIntoView({ block: "nearest" });
}

function stepChapter(delta) {
  const i = BK.order.indexOf(BK.chapter) + delta;
  if (BK.order.includes(BK.chapter) && i >= 0 && i < BK.order.length) openChapter(BK.order[i]);
}

function pickChapter(id) {
  openChapter(id);
  record(["chapter", id]);
}

// a hadith number typed, or stepped to: its chapter, scrolled to it (_hadith_typed)
async function hadithTyped() {
  if (!BK.work) return;
  const number = BK.nav.number();
  let d = await invoke("hadith_by_number", { book: BK.work.key, number });
  if (!d.id) {
    if (!d.near) return;
    BK.nav.setNumber(d.near);
    d = await invoke("hadith_by_number", { book: BK.work.key, number: d.near });
  }
  if (d.id) await showHadithInBook(d.id);
}

// the hadith at the top of the reading pane, kept a moment after the scrolling stops (BM3)
function topHadith() {
  const body = $("b-body");
  const top = body.scrollTop;
  for (const el of body.querySelectorAll("[data-hadith]")) {
    if (el.offsetTop + el.offsetHeight > top + 8) return Number(el.dataset.hadith);
  }
  return null;
}

function rememberReading() {
  if (BK.chapter && BK.work && BK.work.kind === "hadith") {
    const hid = topHadith();
    if (hid) invoke("book_remember", { key: BK.work.key, chapter: BK.chapter, hadith: hid });
  }
}

// ------------------------------------------------------------------ opened from other pages

async function booksShow() {
  show("books");
  await booksRefresh();
}

async function openPlaceInBook(where, o) {
  BK.opening = true;
  try {
    show("books");
    await booksRefresh();
  } finally {
    BK.opening = false;
  }
  if (!BK.work || BK.work.key !== where.key) await selectWork(where.key, where.chapter, o);
  else await openChapter(where.chapter, o);
}

// "In its book" (show_hadith): the book and chapter of a hadith, scrolled to it
async function showHadithInBook(hid) {
  const where = await invoke("book_place_of", { hadith: hid, person: null, chapter: null });
  if (where) await openPlaceInBook(where, { hadith: hid });
}

async function openInBook(hid) {
  await place(["in_book", hid], () => showHadithInBook(hid));
}

// a narrator's entry in the rijal work (show_person)
async function openPersonInBook(pid) {
  await place(["person_in_book", pid], async () => {
    const where = await invoke("book_place_of", { hadith: null, person: pid, chapter: null });
    if (where) await openPlaceInBook(where, { person: pid });
  });
}

// a book by its key (_open_book)
async function openBookAt(key) {
  await place(["book", key], async () => {
    BK.opening = true;
    try {
      show("books");
      await booksRefresh();
    } finally {
      BK.opening = false;
    }
    await selectWork(key);
  });
}

// a chapter by its node (show_chapter: a bookmark, Back and Forward)
async function openChapterAt(chapterId) {
  if (chapterId < 0) { await openVerseAt(-chapterId, null); return; }       // a sura of the Qur'an
  const where = await invoke("book_place_of", { hadith: null, person: null, chapter: chapterId });
  if (where) await openPlaceInBook(where, {});
}

async function openNarratorFromBook(id) {
  await place(["narrator", id], async () => {
    NARR.id = id;
    show("narrators");
    await selectNarrator(id);
  });
}

// ------------------------------------------------------------------ the menus (CTX1, BM3)

function searchHere() {
  if (!BK.work || !BK.chapter) return;
  if (BK.work.kind !== "hadith") {
    setFooter(t("Only the books of hadith are searched; this is a book of narrators."));
    return;
  }
  const item = $("b-tree").querySelector(`.pd-item[data-node="${BK.chapter}"]`);
  const trail = [];
  for (let n = item && item.closest(".pd-node"); n; n = n.parentElement.closest(".pd-node")) {
    trail.unshift(n.querySelector(".pd-item").textContent.replace(/\s+\(\d+\)$/, ""));
  }
  // the Search page with the chapter beside the words (SRCH-SEC1)
  show("search");
  setChapterScope(BK.work.key, BK.chapter, trail.join(" › "));
}

async function marksMenu() {
  const d = await invoke("marks_list");
  const entries = [act("Put a marker here…", BK.work && BK.chapter ? addMark : null, "", { disabled: !(BK.work && BK.chapter) })];
  if (d.marks.length) {
    entries.push(SEP);
    for (const m of d.marks) {
      entries.push(act(`🔖 ${m.name}   —   ${m.where}`, null, "", { raw: true, submenu: () => [
        act("Go to this marker", () => goToMark({ id: m.id })),
        act("Rename…", () => renameMark(m)),
        SEP,
        act("✕  " + t("Remove this marker"), async () => {
          await invoke("mark_remove", { id: m.id });
          setFooter(t("Marker removed: {name}", { name: m.name }));
        }, "", { raw: true }),
      ] }));
    }
  }
  if (d.left.length) {
    entries.push(SEP);
    for (const p of d.left) {
      entries.push(act(p.where, null, "", { raw: true, submenu: () => [
        act("Go there", () => goToMark({ book: p.book })),
        SEP,
        act("✕  " + t("Forget where I left this book"), () => forgetPlace(p.book), "", { raw: true }),
      ] }));
    }
  }
  if (d.marks.length || d.left.length) entries.push(SEP);
  if (d.marks.length) entries.push(act("✕  " + t("Remove all markers…"), () => removeAllMarks(d.marks.length), "", { raw: true }));
  if (d.left.length) entries.push(act("✕  " + t("Forget where I left every book…"), () => forgetPlace(null), "", { raw: true }));
  await popup($("b-marks"), async () => entries);
}

async function addMark() {
  if (!BK.work || !BK.chapter) return;
  const hadith = BK.work.kind === "hadith" ? topHadith() : null;
  const suggested = await invoke("mark_suggest", { key: BK.work.key, chapter: BK.chapter, hadith });
  const name = await askText(t("Put a marker here"), t("Name of the marker:"), suggested);
  if (name !== null) await invoke("mark_add", { name, key: BK.work.key, chapter: BK.chapter, hadith });
}

async function renameMark(m) {
  const name = await askText(t("Rename a marker"), t("Name of the marker:"), m.name);
  if (name !== null && name.trim()) await invoke("mark_rename", { id: m.id, name });
}

async function removeAllMarks(n) {
  if (!await askYesNo(t("Remove all markers"), t("Remove all {n} of your markers? Where you left each book is kept.", { n }))) return;
  const removed = await invoke("marks_remove_all");
  setFooter(t("{n} markers removed", { n: removed }));
}

async function forgetPlace(book) {
  if (book === null && !await askYesNo(t("Forget where I left every book"), t("Forget where you left every book? Your named markers are kept."))) return;
  const n = await invoke("place_forget", { book });
  setFooter(book === null ? t("Forgot where you left {n} books", { n }) : t("Forgot where you left this book"));
  await booksRefresh(true);
}

async function goToMark(which) {
  const p = await invoke("mark_resolve", { id: which.id || null, book: which.book || null });
  if (!p) return;
  if (!BK.work || BK.work.key !== p.book) await selectWork(p.book, p.chapter, { hadith: p.hadith });
  else await openChapter(p.chapter, { hadith: p.hadith });
}

// right-click on a book (_work_menu): open, where I left it, its markers, from the first chapter, copy its name
async function workMenu(e, key) {
  const w = BK.works.find((x) => x.key === key);
  if (!w) return;
  const marks = (await invoke("marks_list")).marks.filter((m) => m.book === key);
  const left = w.last;
  const entries = [
    act("Open", () => { selectWork(key); }),
    act("Go where I left this book", left ? () => goToMark({ book: key }) : null, "", { disabled: !left }),
    act("Markers in this book", null, "", { submenu: () => marks.map((m) => act(`🔖 ${m.name}`, () => goToMark({ id: m.id }), "", { raw: true })), disabled: !marks.length }),
    act("From the first chapter", w.kind === "hadith" ? async () => {
      if (!BK.work || BK.work.key !== key) await selectWork(key);
      if (BK.order.length) await openChapter(BK.order[0]);
    } : null, "", { disabled: w.kind !== "hadith" }),
    SEP,
    act("Copy the name of the book", () => copyText(w.title)),
  ];
  if (left) entries.push(act("✕  " + t("Forget where I left this book"), () => forgetPlace(key), "", { raw: true }));
  popupAt(e, entries);
}

// right-click on a chapter (_chapter_menu)
async function chapterMenu(e, item) {
  const chapter = item.dataset.id ? Number(item.dataset.id) : null;
  const title = item.textContent.replace(/   \(\d+\)$/, "");
  const hadith = BK.work && BK.work.kind === "hadith";
  const go = async (then) => { await openChapter(chapter); if (then) await then(); };
  const entries = [act("Open this chapter", chapter ? () => go() : null, "", { disabled: !chapter })];
  if (hadith) entries.push(act("Search in this chapter", chapter ? () => go(searchHere) : null, "", { disabled: !chapter }));
  entries.push(act("Put a marker here…", chapter ? () => go(addMark) : null, "", { disabled: !chapter }), SEP);
  if (chapter) {
    const d = await invoke("book_chapter_ref", { chapter });
    if (d) {
      const marked = await invoke("is_bookmarked", { kind: "chapter", reference: d.ref });
      entries.push(act((marked ? "★  " + t("Remove from bookmarks") : "☆  " + t("Add to bookmarks")), async () => {
        await invoke("toggle_bookmark", { kind: "chapter", reference: d.ref, title: d.title, subtitle: d.subtitle });
        if (chapter === BK.chapter) openChapter(BK.chapter, { keepFind: true });
      }, "", { raw: true }));
    }
  }
  entries.push(act("Copy the chapter title", () => copyText(title)));
  popupAt(e, entries);
}

// ------------------------------------------------------------------ small dialogs (QInputDialog, QMessageBox)

// QInputDialog.getText; `lines`: getMultiLineText (Enter makes a new line there)
function askText(title, label, value, lines) {
  return new Promise((resolve) => {
    const back = document.createElement("div");
    back.className = "dialog-back";
    const field = lines === "password" ? `<input type="password" class="ask-input">`
      : lines ? `<textarea class="ask-input" rows="6"></textarea>` : `<input type="text" class="ask-input">`;
    back.innerHTML = `<div class="dialog ask" role="dialog"><div class="pd-title">${esc(title)}</div><div>${esc(label)}</div>
      ${field}<div class="pd-buttons"><button class="primary small ok">${esc(t("OK"))}</button><button class="pd-cancel">${esc(t("Cancel"))}</button></div></div>`;
    document.body.appendChild(back);
    const input = back.querySelector(".ask-input");
    input.value = value || "";
    const done = (v) => { back.remove(); resolve(v); };
    back.querySelector(".ok").onclick = () => done(input.value);
    back.querySelector(".pd-cancel").onclick = () => done(null);
    input.addEventListener("keydown", (e) => {
      if (e.key === "Enter" && !lines) done(input.value);
      if (e.key === "Escape") done(null);
    });
    input.focus();
    input.select();
  });
}

function askYesNo(title, text) {
  return new Promise((resolve) => {
    const back = document.createElement("div");
    back.className = "dialog-back";
    back.innerHTML = `<div class="dialog ask" role="dialog"><div class="pd-title">${esc(title)}</div><div>${esc(text)}</div>
      <div class="pd-buttons"><button class="primary small yes">${esc(t("Yes"))}</button><button class="pd-cancel no">${esc(t("No"))}</button></div></div>`;
    document.body.appendChild(back);
    const done = (v) => { back.remove(); resolve(v); };
    back.querySelector(".yes").onclick = () => done(true);
    back.querySelector(".no").onclick = () => done(false);
    back.querySelector(".yes").focus();
  });
}
