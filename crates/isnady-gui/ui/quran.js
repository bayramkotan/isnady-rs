"use strict";
// The Qur'an in Books (gui/quran_view.py, Q1, Q2): a sura with its verses in Arabic and the translations chosen, a
// search through the whole text, and under each verse the hadith that name it — the Sunni books and the Shia books
// each on a line of its own (SH1). The contents of Books list the suras.
//
// 2. Al-Baqara — The Cow                                     البقرة
// 286 verses · Medinan · 120 hadith name its verses
// [‹ Previous sura] [Next sura ›]  [search the Qur'an …]  [➕ Translations…]
// ☑ English — Saheeh International  ☐ Turkish — Diyanet …
// the verses, forty at a time

const QR = { sura: 1, focus: null, rows: [], shown: 0, data: null, seq: 0, built: false };
const QR_PAGE = 40;

function quranBuild() {
  if (QR.built) return;
  QR.built = true;
  const box = $("b-quran");
  box.innerHTML = `<div class="qr-head"><div class="qr-titles"><div class="qr-title" id="qr-title"></div>
      <div class="caption" id="qr-facts"></div></div><div class="qr-ar" id="qr-ar"></div></div>
    <div class="qr-nav" id="qr-nav"><button class="quiet" id="qr-prev">‹  ${esc(t("Previous sura"))}</button>
      <button class="quiet" id="qr-next">${esc(t("Next sura"))}  ›</button>
      <input type="search" id="qr-find" placeholder="${esc(t("Search the Qur'an — Arabic or a translation's words"))}" autocomplete="off" spellcheck="false">
      <button class="quiet" id="qr-add">➕ ${esc(t("Translations…"))}</button></div>
    <div class="qr-langs" id="qr-langs"></div>
    <div class="ncard qr-offer" id="qr-offer" hidden></div>
    <div class="qr-body" id="qr-body"></div>`;
  $("qr-prev").onclick = () => quranGo(QR.sura - 1);
  $("qr-next").onclick = () => quranGo(QR.sura + 1);
  $("qr-add").onclick = later(["Translations of the Qur'an"]);
  $("qr-find").addEventListener("keydown", (e) => { if (e.key === "Enter") quranSearch(); });
  $("qr-find").addEventListener("search", () => { if (!$("qr-find").value.trim()) showSura(QR.sura); });
  $("qr-langs").addEventListener("change", async (e) => {
    const box = e.target.closest("label[data-key]");
    if (!box) return;
    await invoke("quran_toggle", { key: box.dataset.key, on: e.target.checked });
    showSura(QR.sura, null, true);
  });
  $("qr-offer").addEventListener("click", async (e) => {
    const b = e.target.closest("button[data-act]");
    if (!b) return;
    if (b.dataset.act === "skip") {
      await invoke("quran_skip", { languages: QR.data.missing });
      $("qr-offer").hidden = true;
    } else {
      later(["Translations of the Qur'an"])();
    }
  });
  $("qr-body").addEventListener("click", (e) => {
    const a = e.target.closest("a[data-hadith], [data-verse], [data-act]");
    if (!a) return;
    if (a.dataset.hadith) openInBook(Number(a.dataset.hadith));
    else if (a.dataset.verse) {
      const [s, v] = a.dataset.verse.split(":").map(Number);
      $("qr-find").value = "";
      showSura(s, v);
    } else if (a.dataset.act === "more") quranMore();
    else if (a.dataset.act === "download") later(["Download the Qur'an"])();
  });
}

// the Qur'an chosen on the shelf (BooksPage.open_quran): the suras as the contents, the sura read last (or the one
// asked for)
async function openQuran(sura, ayah) {
  BK.quranMode = true;
  BK.work = null;
  markWork();
  $("b-right").hidden = true;
  $("b-quran").hidden = false;
  quranBuild();
  const s = await invoke("quran_state");
  $("b-tree").innerHTML = s.suras.map((x) =>
    `<div class="pd-node"><div class="pd-item" data-sura="${x.n}" style="padding-inline-start:25px"><span>${esc(x.label)}</span></div></div>`).join("");
  await showSura(sura || s.last || 1, ayah);
}

function quranGo(n) {
  if (n < 1 || n > 114) return;
  place(["chapter", -n], () => showSura(n));
}

// a verse a hadith names, opened in the Qur'an (show_verse, Q2)
async function openVerseAt(sura, ayah) {
  BK.opening = true;
  try {
    show("books");
    await booksRefresh();
  } finally {
    BK.opening = false;
  }
  await openQuran(sura, ayah);
}

function openVerse(sura, ayah) {
  return place(["verse", sura, ayah || null], () => openVerseAt(sura, ayah));
}

function quranText(text, rtl, factor, color) {
  const s = scriptOf(rtl ? "arabic" : "latin");
  return `<div class="qr-text${rtl ? " rtl" : ""}" dir="${rtl ? "rtl" : "ltr"}" style="font-family:${esc(s.family)};font-size:calc(${(s.size * factor).toFixed(2)}pt * var(--scale));line-height:normal;color:${esc(color || s.color)}">${text}</div>`;
}

async function showSura(n, ayah, keep) {
  const seq = ++QR.seq;
  const d = await invoke("quran_sura", { n });
  if (seq !== QR.seq) return;
  const body = $("qr-body");
  if (d.download) { quranDownload(d); return; }
  if (d.error) return;
  for (const id of ["qr-nav", "qr-langs"]) $(id).hidden = false;
  const top = keep ? body.scrollTop : 0;
  QR.sura = n;
  QR.focus = ayah || null;
  QR.data = d;
  $("qr-title").textContent = d.title;
  $("qr-ar").innerHTML = arabicSpan(d.arabic, 0.9);
  $("qr-facts").textContent = t("{verses} verses · {place}", { verses: d.verses_count, place: t(d.meccan ? "Meccan" : "Medinan") })
    + (d.named ? " · " + t("{n} hadith name its verses", { n: d.named }) : "");
  $("qr-prev").disabled = n <= 1;
  $("qr-next").disabled = n >= 114;
  $("qr-langs").innerHTML = d.editions.map((e) =>
    `<span data-tip="${esc(t("Source: {source}", { source: e.source }) + "\n" + t("Read from {origin}", { origin: e.origin }))}">${checkRow(`${t(e.language)} — ${e.author}`, e.on, `data-key="${esc(e.key)}"`)}</span>`).join("");
  const offer = $("qr-offer");
  offer.hidden = !d.missing.length;
  if (d.missing.length) {
    const names = d.missing.map((l) => t(l)).join(", ");
    offer.innerHTML = `<div class="lead">📥 ${esc(t("No translation in {languages} is downloaded yet.", { languages: names }))}</div>`
      + d.missing.map((l) => `<button class="primary" data-act="get" data-tip="${esc(t("The translation readers of {language} know best, from isnady-data; others with ➕ Translations…", { language: t(l) }))}">${esc(t("Download {language}", { language: t(l) }))}</button>`).join("")
      + `<button class="quiet" data-act="other">${esc(t("Another translation…"))}</button><button class="quiet" data-act="skip" data-tip="${esc(t("Do not offer it again"))}">✕</button>`;
  }
  for (const x of $("b-tree").querySelectorAll(".pd-item")) {
    const on = Number(x.dataset.sura) === n;
    x.classList.toggle("on", on);
    if (on) x.scrollIntoView({ block: "nearest" });
  }
  QR.rows = d.verses;
  QR.shown = 0;
  body.innerHTML = "";
  quranMore(ayah);
  if (keep) body.scrollTop = top;
  else if (ayah) {
    for (const delay of [120, 400, 900]) setTimeout(quranFocus, delay);
  } else body.scrollTop = 0;
}

function quranVerse(row) {
  let html = quranText(`${esc(row.arabic)} <span class="qr-num">﴿${row.ayah}﴾</span>`, true, 1);
  for (const x of row.texts) {
    html += quranText(esc(x.text), x.rtl, 0.9);
    html += `<div class="caption">${esc(t(x.language))}, ${esc(x.author)}</div>`;
  }
  let links = "";
  for (const [road, title] of [["sunni", "Named in the Sunni books:"], ["shia", "Named in the Shia books:"]]) {
    const mine = row[road];
    if (!mine.length) continue;
    const parts = mine.slice(0, 12).map(([h, book, num]) => `<a data-hadith="${h}" class="${road}">${esc(book)} ${esc(num)}</a>`);
    const more = mine.length > 12 ? ` … +${mine.length - 12}` : "";
    links += `<div class="caption qr-named" data-tip="${esc(t("Hadith whose translation marks this verse as the one it quotes or explains, e.g. \"(33:33)\" — the translator's mark, not a reading of isnady's own. Click one to read it in its book."))}">📜 ${esc(t(title))} ${parts.join(" · ")}${more}</div>`;
  }
  html += `<div class="qr-foot"><div class="caption qr-ref">${esc(QR.data.name)} ${QR.sura}:${row.ayah}</div><div class="qr-links">${links}</div></div>`;
  return `<div class="ncard qr-verse" data-ayah="${row.ayah}">${html}</div>`;
}

function quranMore(until) {
  const body = $("qr-body");
  const more = body.querySelector(".qr-more");
  if (more) more.remove();
  const end = Math.max(QR.shown + QR_PAGE, until || 0);
  body.insertAdjacentHTML("beforeend", QR.rows.slice(QR.shown, end).map(quranVerse).join(""));
  QR.shown = Math.min(end, QR.rows.length);
  const left = QR.rows.length - QR.shown;
  if (left > 0) {
    body.insertAdjacentHTML("beforeend", `<div class="qr-more"><button class="quiet" data-act="more">${esc(t("Show {page} more verses ({left} left)", { page: Math.min(QR_PAGE, left), left }))}</button></div>`);
  }
}

function quranFocus() {
  const card = $("qr-body").querySelector(`.qr-verse[data-ayah="${QR.focus}"]`);
  if (!card) return;
  const body = $("qr-body");
  body.scrollTop = Math.max(0, body.scrollTop + card.getBoundingClientRect().top - body.getBoundingClientRect().top - 20);
  card.classList.add("focus");
}

function quranDownload(d) {
  $("qr-title").textContent = t("The Qur'an");
  $("qr-ar").innerHTML = arabicSpan("القرآن الكريم", 0.9);
  $("qr-facts").textContent = "";
  for (const id of ["qr-nav", "qr-langs", "qr-offer"]) $(id).hidden = true;
  $("qr-body").innerHTML = `<div class="ncard qr-download"><div class="lead">${esc(t("The text of the Qur'an and its translations are downloaded once, on request, from isnady-data: the Arabic (Tanzil's text) with a translation in each language you read — {languages} — about 1 MB each; more translations can be added afterwards, of 98 languages (those isnady-data does not keep are read from fawazahmed0/quran-api).", { languages: d.languages.map((l) => t(l)).join(", ") }))}</div>
    <div><button class="primary" data-act="download">${esc(t("Download the Qur'an"))}</button></div></div>`;
}

async function quranSearch() {
  const text = $("qr-find").value.trim();
  if (!text) { showSura(QR.sura); return; }
  ++QR.seq;
  const found = await invoke("quran_search", { text });
  $("qr-title").textContent = t("Search the Qur'an");
  $("qr-ar").innerHTML = "";
  $("qr-facts").textContent = found.length < 200 ? t("{n} verses hold “{words}”", { n: found.length, words: text })
    : t("The first 200 verses that hold “{words}”", { words: text });
  const body = $("qr-body");
  body.innerHTML = found.length ? found.map((r) => `<div class="ncard qr-found"><div><button class="linkbtn" data-verse="${r.sura}:${r.ayah}">${esc(r.label)}</button></div>${quranText(esc(r.text), r.rtl, r.rtl ? 1 : 0.9)}</div>`).join("")
    : `<div class="lead">${esc(t("No verse holds these words in the texts downloaded."))}</div>`;
  body.scrollTop = 0;
}
