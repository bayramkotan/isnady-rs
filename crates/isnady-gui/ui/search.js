// isnady-rs — the Search page, as the Python window has it (gui/search_page.py, gui/widgets.py BookPicker,
// GradePicker, LanguagePicker, gui/chain_widgets.py ChainStrip): what is searched, the words, the filters, the
// welcome with the examples and the recent searches, and each hadith as a card with its chain, grades and texts.

"use strict";

const PAGE_SIZE = 25;
const MODE_LABELS = [["all", "All words"], ["any", "Any word"], ["phrase", "Exact phrase"], ["meaning", "By meaning (AI)"]];
const EXAMPLES = ["النيات", "الصلاة", "niyet", "komşu"];
const KINDS = [["hadith", "Hadith"], ["narrators", "Narrators"], ["scholars", "Scholars"]];
const KIND_TEXT = {   // placeholder, start-page title, start-page lead, examples
  hadith: ["Search the hadith: a word or phrase in Arabic, Turkish or English (? an unknown letter, * any letters)", "Search the hadith", "", EXAMPLES],
  narrators: ["Search the narrators by name: Arabic, English or Turkish spelling", "Search the narrators",
    "Every narrator of both traditions — Ibn Hajar's Taqrib for the Sunni books, al-Najashi's Rijal for the Shia books — by any part of his name. Names are matched by their letters, without vowels or diacritics, so “Abu Hurayra”, “Ebû Hüreyre” and أبو هريرة all find him. Try one of these:",
    ["Abu Hurayra", "الزهري", "Ibn Umar", "Âişe", "Zurara"]],
  scholars: ["Search the scholars of hadith: a name or a book", "Search the scholars",
    "The compilers of the books, the scholars who graded their hadith and the critics of narrators — by name in any spelling, or by the title of a work. Try one of these:",
    ["Buhârî", "Albani", "ابن حجر", "Riyad al-Salihin"]],
};
const TRADITIONS = [["", "Both traditions"], ["sunni", "Sunni"], ["shia", "Shia"]];
const SUNNI_FILTERS = [["sahih", "sahih"], ["hasan", "hasan"], ["daif", "da'if"], ["very-weak", "very weak"], ["fabricated", "fabricated"], ["ungraded", "not graded"]];
const SHIA_FILTERS = [["imami-sahih", "sahih"], ["imami-hasan", "hasan"], ["muwaththaq", "muwaththaq"], ["imami-daif", "da'if"], ["imami-ungraded", "not graded"]];
const DIRECT = "no chain: the compiler gives the words without the narrators between";
const BORROWED_PROBLEM = "the chain of the hadith before it (\"from him\" / \"with this chain\")";
const TO_US = new Set(["haddathana", "haddathani", "akhbarana", "akhbarani", "anba'ana", "anba'ani", "sami'tu", "qara'tu 'ala", "bare"]);
const SHIA_COLOR = { 1: 2, 2: 4, 3: 5, 4: 9 };

const SEARCH = {
  kind: "hadith", mode: "all", whole: false, tradition: "",
  books: null,            // null: every book; else the keys chosen
  grades: [], every: false, sahihayn: true,
  languages: [],          // []: every language
  chapters: [],           // [{id, book, title}] (SRCH-SEC1)
  groups: [], allLanguages: [], moreLanguages: [], hadith: 0, persons: true,
  page: null, results: [], searched: false, atStart: true, token: 0,
};

// ------------------------------------------------------------------ the frame of the page

function buildSearchPage() {
  // a label is as tall as its typeface's own line (QLabel): the chips of the chains take Amiri's
  document.documentElement.style.setProperty("--arabic-line", naturalLine(scriptOf("arabic").family, "قال").toFixed(3));
  const page = $("page-search");
  page.innerHTML = `
    <div class="s-top">
      <button id="s-home" class="quiet home" data-tip="${esc(t("Back to the start: clear the search, the filters and the results (Alt+Home)"))}">⌂</button>
      <button id="s-kind" class="combo kind" data-tip="${esc(t("What to search: the text of the hadith, the narrators of both traditions by name, or the scholars of hadith"))}"></button>
      <div class="s-field"><input id="q" type="text" autocomplete="off" spellcheck="false"><button id="q-clear" class="clear" hidden>✕</button></div>
      <button id="go" class="primary">${esc(t("Search"))}</button>
    </div>
    <div class="s-filters">
      <div id="s-hadith-filters" class="row">
        <span class="gap"></span><span class="flabel">${esc(t("Match"))}</span>
        <button id="s-mode" class="combo"></button>
        <label class="check" data-tip="${esc(t("Off: النيات also finds بالنيات, because Arabic words carry attached prefixes.\nOn: only the word standing on its own."))}"><input id="whole" type="checkbox"><span class="box"></span>${esc(t("Whole words only"))}</label>
        <span class="gap"></span><span class="flabel">${esc(t("Books"))}</span>
        <button id="s-books" class="picker" data-tip="${esc(t("Search all books, or tick the ones to search"))}"></button>
        <button id="s-chapter" class="picker active" hidden></button>
        <span class="gap"></span><span class="flabel">${esc(t("Grade"))}</span>
        <button id="s-grades" class="picker"></button>
        <span class="gap"></span><span class="flabel">${esc(t("Language"))}</span>
        <button id="s-languages" class="picker" data-tip="${esc(t("Search every language, or tick the ones to search"))}"></button>
      </div>
      <div id="s-people-filters" class="row" hidden>
        <span class="gap"></span><span class="flabel">${esc(t("Tradition"))}</span>
        <button id="s-tradition" class="combo" data-tip="${esc(t("Sunni: the narrators of Ibn Hajar's Taqrib. Shia: those of al-Najashi's Rijal.\nThe two are never matched to each other: each keeps its own terms."))}"></button>
      </div>
      <span class="grow"></span>
      <span id="s-summary"></span>
      <button id="s-export" class="qlink" hidden data-tip="${esc(t("All the hadith found (up to 1,000), with the books, languages, grades and chapter chosen here, to a file: Word, PDF, Excel, CSV, JSON …"))}">⤓ ${esc(t("Export these results"))}</button>
    </div>
    <div id="s-body" class="s-body"></div>`;
  $("s-home").onclick = goHome;
  $("s-kind").onclick = () => comboPopup($("s-kind"), KINDS, SEARCH.kind, setKind);
  $("s-mode").onclick = () => comboPopup($("s-mode"), MODE_LABELS, SEARCH.mode, (m) => {
    SEARCH.mode = m;
    labelCombos();
    $("whole").disabled = m === "meaning";
    rerunIfSearched();
  });
  $("s-tradition").onclick = () => comboPopup($("s-tradition"), TRADITIONS, SEARCH.tradition, (v) => {
    SEARCH.tradition = v;
    labelCombos();
    rerunIfSearched();
  });
  $("whole").onchange = () => { SEARCH.whole = $("whole").checked; rerunIfSearched(); };
  $("s-books").onclick = openBookPicker;
  $("s-grades").onclick = openGradePicker;
  $("s-languages").onclick = openLanguagePicker;
  $("s-chapter").onclick = clearChapterScope;
  $("s-export").onclick = later(["&File", "Export…"]);
  $("go").onclick = runSearch;
  $("q").addEventListener("keydown", (e) => {
    if (e.key === "Enter") runSearch();
    if (e.key === "Escape" && $("q").value) { e.stopPropagation(); $("q").value = ""; $("q-clear").hidden = true; }
  });
  $("q").addEventListener("input", () => { $("q-clear").hidden = !$("q").value; });
  $("q-clear").onclick = () => { $("q").value = ""; $("q-clear").hidden = true; $("q").focus(); };
  document.addEventListener("keydown", (e) => {
    if (e.altKey && e.key === "Home" && CURRENT === "search") { e.preventDefault(); goHome(); }
  });
  labelCombos();
}

async function fillSearchFilters() {
  try {
    const d = await invoke("search_start");
    SEARCH.groups = d.groups;
    SEARCH.allLanguages = d.languages;
    SEARCH.moreLanguages = d.more;
    SEARCH.hadith = d.hadith;
    SEARCH.persons = d.persons;
    SEARCH.recent = d.recent;
  } catch (e) {
    SEARCH.error = String(e);
  }
  const keys = new Set(SEARCH.groups.flatMap((g) => g.books.map((b) => b.key)));
  if (SEARCH.books) {
    SEARCH.books = SEARCH.books.filter((k) => keys.has(k));
    if (!SEARCH.books.length || SEARCH.books.length === keys.size) SEARCH.books = null;
  }
  SEARCH.languages = SEARCH.languages.filter((l) => SEARCH.allLanguages.includes(l));
  if (SEARCH.languages.length === SEARCH.allLanguages.length) SEARCH.languages = [];
  labelPickers();
}

// a QComboBox is as wide as its longest choice: every choice is laid in the same cell, only the current one seen
function comboHtml(list, v) {
  const current = (list.find((x) => x[0] === v) || list[0])[0];
  return `<span class="sizer">${list.map(([k, l]) => `<span${k === current ? "" : ' class="off"'}>${esc(t(l))}</span>`).join("")}</span><i class="chev"></i>`;
}

function labelCombos() {
  $("s-kind").innerHTML = comboHtml(KINDS, SEARCH.kind);
  $("s-mode").innerHTML = comboHtml(MODE_LABELS, SEARCH.mode);
  $("s-tradition").innerHTML = comboHtml(TRADITIONS, SEARCH.tradition);
  $("q").placeholder = t(KIND_TEXT[SEARCH.kind][0]);
  $("s-hadith-filters").hidden = SEARCH.kind !== "hadith";
  $("s-people-filters").hidden = SEARCH.kind !== "narrators";
  // the scholars have no filters: the row is only as tall as the count beside it
  document.querySelector("#page-search .s-filters").classList.toggle("bare", SEARCH.kind === "scholars");
}

function labelPickers() {
  // the books (BookPicker._label)
  let text;
  if (!SEARCH.books) text = t("All books");
  else {
    const chosen = new Set(SEARCH.books);
    const whole = SEARCH.groups.find((g) => g.books.length === chosen.size && g.books.every((b) => chosen.has(b.key)));
    if (whole) text = `${t(whole.label)} (${chosen.size})`;
    else if (chosen.size === 1) text = (SEARCH.groups.flatMap((g) => g.books).find((b) => b.key === SEARCH.books[0]) || {}).name;
    else text = t("{n} books", { n: chosen.size });
  }
  setPicker($("s-books"), text, !!SEARCH.books);
  // the grades (GradePicker._label)
  if (!SEARCH.grades.length) {
    setPicker($("s-grades"), t("All grades"), false);
    $("s-grades").dataset.tip = t("Search hadith of every grade, or tick the grades to search (sahih, hasan …)");
  } else {
    const names = [];
    for (const [filters, road] of [[SUNNI_FILTERS, t("Sunni")], [SHIA_FILTERS, t("Shia")]]) {
      const picked = filters.filter(([c]) => SEARCH.grades.includes(c)).map(([, l]) => t(l));
      if (picked.length) names.push(picked.join(", ") + (road === t("Sunni") ? "" : ` (${road})`));
    }
    let g = names.join(" · ");
    if (Array.from(g).length > 40) g = t("{n} grades", { n: SEARCH.grades.length });
    setPicker($("s-grades"), g, true);
    $("s-grades").dataset.tip = gradesWords(SEARCH.grades, SEARCH.every);
  }
  // the languages (LanguagePicker._label)
  if (!SEARCH.languages.length) setPicker($("s-languages"), t("All languages"), false);
  else {
    const names = SEARCH.languages.map((l) => t(l));
    setPicker($("s-languages"), names.length <= 2 ? names.join(", ") : t("{n} languages", { n: names.length }), true);
  }
}

function setPicker(button, text, active) {
  button.textContent = text + "   ▾";
  button.classList.toggle("active", active);
}

// "sahih, hasan (Sunni) · sahih (Shia)" (grades.describe_choice)
function gradesWords(wanted, every) {
  const parts = [];
  for (const [filters, road] of [[SUNNI_FILTERS, "Sunni"], [SHIA_FILTERS, "Shia"]]) {
    const names = filters.filter(([c]) => wanted.includes(c)).map(([, l]) => t(l));
    if (names.length) parts.push(`${names.join(", ")} (${t(road)})`);
  }
  const text = parts.join(" · ");
  return text + (every && parts.length ? " — " + t("every grader agrees") : "");
}

// a QComboBox's list: the choices under the box, the current one marked
function comboPopup(button, items, current, pick) {
  closeMenus();
  const r = button.getBoundingClientRect();
  const el = document.createElement("div");
  el.className = "menu combo-list";
  el.style.minWidth = r.width + "px";
  el.innerHTML = items.map(([v, label]) => `<div class="mi${v === current ? " on" : ""}" data-v="${esc(v)}"><span class="lb">${esc(t(label))}</span></div>`).join("");
  el.onmousedown = (e) => e.preventDefault();
  el.onclick = (e) => {
    const row = e.target.closest(".mi");
    if (!row) return;
    closeMenus();
    if (row.dataset.v !== current) pick(row.dataset.v);
  };
  el.onmouseover = (e) => {
    const row = e.target.closest(".mi");
    if (!row) return;
    for (const x of el.querySelectorAll(".mi.on")) x.classList.remove("on");
    row.classList.add("on");
  };
  document.body.appendChild(el);
  // a QComboBox shows ten of its items at a time, the others under a scroll bar (maxVisibleItems)
  const rows = el.querySelectorAll(".mi");
  if (rows.length > 10) {
    const cs = getComputedStyle(el);
    const pad = parseFloat(cs.paddingTop) + parseFloat(cs.paddingBottom) + parseFloat(cs.borderTopWidth) + parseFloat(cs.borderBottomWidth);
    el.style.maxHeight = (rows[10].offsetTop - rows[0].offsetTop + pad) + "px";
    el.style.overflowY = "auto";
  }
  placeMenu(el, { x: isRtl() ? r.right : r.left, y: r.bottom + 2, top: r.top, flipUp: true, alignEnd: isRtl() });
  MENU.stack.push({ el, items: [], index: -1, item: null });
}

// a picker's list (QFrame Popup, objectName Card): closed by a click outside it
function pickerPopup(button, html, wire) {
  closeMenus();
  const r = button.getBoundingClientRect();
  const el = document.createElement("div");
  el.className = "menu picker-pop";
  el.innerHTML = html;
  document.body.appendChild(el);
  placeMenu(el, { x: isRtl() ? r.right : r.left, y: r.bottom, top: r.top, flipUp: true, alignEnd: isRtl() });
  MENU.stack.push({ el, items: [], index: -1, item: null });
  wire(el, () => closeMenus());
}

function checkRow(label, checked, attrs, bold) {
  return `<label class="check${bold ? " head" : ""}" ${attrs || ""}><input type="checkbox" ${checked ? "checked" : ""}><span class="box"></span>${esc(label)}</label>`;
}

// a heading's box: ticked, unticked or partly (Qt's ItemIsAutoTristate)
function wireTree(el) {
  const sync = () => {
    for (const head of el.querySelectorAll(".pgroup")) {
      const kids = [...head.querySelectorAll(".pkids input")];
      const on = kids.filter((k) => k.checked).length;
      const box = head.querySelector(".head input");
      box.checked = on === kids.length && on > 0;
      box.indeterminate = on > 0 && on < kids.length;
    }
  };
  for (const head of el.querySelectorAll(".pgroup")) {
    head.querySelector(".head input").onchange = (e) => {
      for (const k of head.querySelectorAll(".pkids input")) k.checked = e.target.checked;
      sync();
    };
    for (const k of head.querySelectorAll(".pkids input")) k.onchange = sync;
  }
  sync();
  return sync;
}

function openBookPicker() {
  const everything = SEARCH.groups.flatMap((g) => g.books.map((b) => b.key));
  const chosen = new Set(SEARCH.books || everything);
  const tree = SEARCH.groups.map((g) => `<div class="pgroup">
      ${checkRow(t("{group} collections", { group: t(g.label) }), false, "", true)}
      <div class="pkids">${g.books.map((b) => checkRow(`${b.name}  (${b.count.toLocaleString("en-US")})`, chosen.has(b.key), `data-key="${esc(b.key)}"`)).join("")}</div></div>`).join("");
  const quick = [[t("All"), everything], ...SEARCH.groups.map((g) => [t(g.label), g.books.map((b) => b.key)]), [t("None"), []]];
  pickerPopup($("s-books"), `<div class="ptree">${tree}</div>
    <div class="row">${quick.map(([label], i) => `<button class="quiet" data-quick="${i}">${esc(label)}</button>`).join("")}
      <span class="grow"></span><button class="primary small" data-done>${esc(t("Search these"))}</button></div>`, (el, close) => {
    const sync = wireTree(el);
    for (const b of el.querySelectorAll("[data-quick]")) {
      b.onclick = () => {
        const keys = new Set(quick[Number(b.dataset.quick)][1]);
        for (const c of el.querySelectorAll("[data-key]")) c.querySelector("input").checked = keys.has(c.dataset.key);
        sync();
      };
    }
    el.querySelector("[data-done]").onclick = () => {
      const picked = [...el.querySelectorAll("[data-key]")].filter((c) => c.querySelector("input").checked).map((c) => c.dataset.key);
      const next = !picked.length || picked.length === everything.length ? null : picked.sort();
      close();
      if (JSON.stringify(next) !== JSON.stringify(SEARCH.books)) {
        SEARCH.books = next;
        labelPickers();
        rerunIfSearched();
      }
    };
  });
}

function openGradePicker() {
  const heads = [[t("Sunni books"), SUNNI_FILTERS], [t("Shia books (the Imami scale: sahih, hasan, muwaththaq, da'if)"), SHIA_FILTERS]];
  const tree = heads.map(([title, filters]) => `<div class="pgroup">${checkRow(title, false, "", true)}
      <div class="pkids">${filters.map(([code, label]) => checkRow(t(label), SEARCH.grades.includes(code), `data-code="${code}"`)).join("")}</div></div>`).join("");
  pickerPopup($("s-grades"), `<div class="ptree grades">${tree}</div>
    ${checkRow(t("Every grader agrees"), SEARCH.every, `data-every data-tip="${esc(t("Off: a hadith is found when one of its graders gave a grade ticked.\nOn: only when every grade it was given is among those ticked."))}"`)}
    ${checkRow(t("Count al-Bukhari's and Muslim's hadith as sahih"), SEARCH.sahihayn, `data-sahihayn data-tip="${esc(t("No grader grades the hadith of the two Sahihs: they are sahih by their compiler's condition and by the community's acceptance of the two books. This is the rule of hadith science, not a grader's grade; each such hadith says so."))}"`)}
    <div class="caption">${esc(t("The grades are those the graders gave, word for word, grouped; a grade given to the chain alone (\"its isnad is hasan\") is counted with its group and shown as written."))}</div>
    <div class="row"><button class="quiet" data-quick="good">${esc(t("Sahih and hasan"))}</button><button class="quiet" data-quick="none">${esc(t("None"))}</button>
      <span class="grow"></span><button class="primary small" data-done>${esc(t("Search these"))}</button></div>`, (el, close) => {
    const sync = wireTree(el);
    for (const b of el.querySelectorAll("[data-quick]")) {
      b.onclick = () => {
        const codes = b.dataset.quick === "good" ? new Set(["sahih", "hasan", "imami-sahih", "imami-hasan"]) : new Set();
        for (const c of el.querySelectorAll("[data-code]")) c.querySelector("input").checked = codes.has(c.dataset.code);
        sync();
      };
    }
    el.querySelector("[data-done]").onclick = () => {
      const picked = [...el.querySelectorAll("[data-code]")].filter((c) => c.querySelector("input").checked).map((c) => c.dataset.code).sort();
      const every = el.querySelector("[data-every] input").checked;
      const sahihayn = el.querySelector("[data-sahihayn] input").checked;
      close();
      if (JSON.stringify([picked, every, sahihayn]) !== JSON.stringify([SEARCH.grades, SEARCH.every, SEARCH.sahihayn])) {
        Object.assign(SEARCH, { grades: picked, every, sahihayn });
        labelPickers();
        rerunIfSearched();
      }
    };
  });
}

function openLanguagePicker() {
  const all = SEARCH.allLanguages;
  const rows = all.map((l) => checkRow(t(l), !SEARCH.languages.length || SEARCH.languages.includes(l), `data-lang="${esc(l)}"`)).join("");
  let more = "";
  if (SEARCH.moreLanguages.length) {
    const names = SEARCH.moreLanguages.slice(0, 5).map((n) => t(n)).join(", ") + (SEARCH.moreLanguages.length > 5 ? " …" : "");
    more = `<button class="qlink" data-more data-tip="${esc(t("These books are in the data repository in more languages: {names}. File → Data Sources imports them (tick the languages beside a book); they then appear in this list.", { names: SEARCH.moreLanguages.map((n) => t(n)).join(", ") }))}">${esc(t("More languages: {names} — import", { names }))}</button>`;
  }
  pickerPopup($("s-languages"), `<div class="langs-list">${checkRow(t("All languages"), !SEARCH.languages.length, "data-all")}${rows}</div>${more}
    <div class="row"><span class="grow"></span><button class="primary small" data-done>${esc(t("Search these"))}</button></div>`, (el, close) => {
    const every = el.querySelector("[data-all] input");
    const boxes = [...el.querySelectorAll("[data-lang] input")];
    every.onchange = () => { for (const b of boxes) b.checked = every.checked; };
    for (const b of boxes) b.onchange = () => { every.checked = boxes.every((x) => x.checked); };
    const m = el.querySelector("[data-more]");
    if (m) m.onclick = () => { close(); later(["&File", "Data Sources…"])(); };
    el.querySelector("[data-done]").onclick = () => {
      const picked = [...el.querySelectorAll("[data-lang]")].filter((c) => c.querySelector("input").checked).map((c) => c.dataset.lang);
      const next = !picked.length || picked.length === all.length ? [] : picked;
      close();
      if (JSON.stringify(next) !== JSON.stringify(SEARCH.languages)) {
        SEARCH.languages = next;
        labelPickers();
        rerunIfSearched();
      }
    };
  });
}

// ------------------------------------------------------------------ what is searched

function setKind(kind) {
  SEARCH.kind = kind;
  labelCombos();
  $("s-summary").textContent = "";
  $("s-export").hidden = true;
  if ($("q").value.trim()) runSearch();
  else { SEARCH.page = null; SEARCH.found = null; SEARCH.searched = false; showStart(); }
  $("q").focus();
}

function searchFor(text, kind) {
  if (kind && kind !== SEARCH.kind) { SEARCH.kind = kind; labelCombos(); }
  $("q").value = text;
  $("q-clear").hidden = !text;
  runSearch();
}

// the page as it opens (NAV1): no words, every filter at its default, the welcome with the recent searches
function goHome() {
  Object.assign(SEARCH, { mode: "all", tradition: "", whole: false, books: null, grades: [], every: false, sahihayn: true, chapters: [] });
  $("whole").checked = false;
  $("whole").disabled = false;
  $("s-chapter").hidden = true;
  SEARCH.kind = "hadith";
  labelCombos();
  labelPickers();
  $("q").value = "";
  $("q-clear").hidden = true;
  SEARCH.page = null;
  SEARCH.found = null;
  SEARCH.searched = false;
  showStart();
  $("s-body").scrollTop = 0;
  $("q").focus();
}

function clearChapterScope() {
  SEARCH.chapters = [];
  $("s-chapter").hidden = true;
  rerunIfSearched();
}

// a chapter of a book to search in, asked from Books (SRCH-SEC1)
function setChapterScope(book, chapterId, title) {
  if (SEARCH.kind !== "hadith") { SEARCH.kind = "hadith"; labelCombos(); }
  SEARCH.books = [book];
  labelPickers();
  SEARCH.chapters = [{ id: chapterId, book, title }];
  const shown = Array.from(title).length <= 34 ? title : Array.from(title).slice(0, 32).join("") + "…";
  $("s-chapter").textContent = "📖 " + shown + "   ✕";
  $("s-chapter").dataset.tip = title + "\n" + t("Searching in this chapter only — click to search the whole book again");
  $("s-chapter").hidden = false;
  show("search");
  $("q").focus();
  $("q").select();
  if ($("q").value.trim()) runSearch();
  else setFooter(t("Type the words to search in {chapter}", { chapter: title }));
}

function rerunIfSearched() {
  if (SEARCH.searched && $("q").value.trim()) runSearch();
}

async function runSearch() {
  const text = $("q").value.trim();
  if (!text) { SEARCH.page = null; SEARCH.found = null; SEARCH.searched = false; showStart(); return; }
  SEARCH.searched = true;
  if (SEARCH.kind !== "hadith") {
    invoke("remember_search", { text, kind: SEARCH.kind });
    await searchPeople(text);
    return;
  }
  SEARCH.found = null;
  if (SEARCH.mode === "meaning") {
    notice(t("Search by meaning"), t("LATER_MENU", { v: APP.version, menu: `${t("Match")} → ${t("By meaning (AI)")}` }) + "\n\n" +
      t("Search by meaning finds hadith that say the same thing in other words or in another language. It learns from the texts imported here; building it takes a few seconds to a minute."));
    $("s-summary").textContent = "";
    $("s-export").hidden = true;
    return;
  }
  document.body.classList.add("busy");
  try {
    const page = await fetchPage(0, true);
    SEARCH.page = page;
    SEARCH.results = page.results.slice();
    render();
  } catch (e) {
    notice(t("Search the hadith"), String(e));
  } finally {
    document.body.classList.remove("busy");
  }
}

function fetchPage(offset, remember) {
  return invoke("find", {
    text: $("q").value.trim(), mode: SEARCH.mode, wholeWords: SEARCH.whole, books: SEARCH.books || [],
    languages: SEARCH.languages, grades: SEARCH.grades, every: SEARCH.every, sahihayn: SEARCH.sahihayn,
    chapters: SEARCH.chapters.map((c) => c.id), offset, limit: PAGE_SIZE, remember: !!remember,
  });
}

async function loadMore() {
  if (SEARCH.found) {
    if (SEARCH.peopleShown >= SEARCH.found.length && SEARCH.peopleTotal > SEARCH.found.length) await readMorePeople();
    showPeople(true);
    return;
  }
  if (!SEARCH.page) return;
  document.body.classList.add("busy");
  try {
    const page = await fetchPage(SEARCH.results.length, false);
    SEARCH.page = page;
    SEARCH.results.push(...page.results);
    $("s-more-row") && $("s-more-row").remove();
    appendCards(page.results);
    updateSummary();
  } finally {
    document.body.classList.remove("busy");
  }
}

// ------------------------------------------------------------------ the welcome (_show_start)

async function showStart() {
  SEARCH.atStart = true;
  $("s-summary").textContent = "";
  $("s-export").hidden = true;
  const kind = SEARCH.kind;
  let body;
  if (SEARCH.error) {
    body = `<div class="lead">${esc(SEARCH.error)}</div>`;
  } else if (SEARCH.hadith === 0 && kind === "hadith") {
    body = `<div class="lead">${t("No books yet. Import them in <b>File → Data Sources</b> (Ctrl+Shift+I) — every book of isnady-data with one click — or from a terminal: <code>iy catalog import --all</code>")}</div>`;
  } else if (kind === "narrators" && !SEARCH.persons) {
    body = `<div class="lead">${esc(t("No narrators yet: import Ibn Hajar's Taqrib or al-Najashi's Rijal (File → Data Sources), or from a terminal: iy catalog import taqrib"))}</div>`;
  } else {
    const lead = KIND_TEXT[kind][2] ? t(KIND_TEXT[kind][2]) : t("Arabic diacritics and letter forms are ignored, so الاعمال finds الأَعْمَالُ. Turkish and other Latin-script text ignores case and accents. Try one of these:");
    body = `<div class="lead narrow">${esc(lead)}</div>
      <div class="examples">${KIND_TEXT[kind][3].map((w) => `<button class="example" data-word="${esc(w)}">${esc(w)}</button>`).join("")}</div>`;
    const rows = await invoke("recent_searches");
    if (rows.length) {
      const names = Object.fromEntries(KINDS);
      body += `<div class="recent-head"><span class="ftitle">${esc(t("Recent searches"))}</span><button class="quiet" id="s-clear-recent">${esc(t("Clear recent searches"))}</button></div>
        <div class="recent">${rows.map((r) => `<button class="example small" data-word="${esc(r.text)}" data-kind="${esc(r.kind)}">${esc(r.kind === "hadith" ? r.text : `${r.text}  · ${t(names[r.kind] || r.kind)}`)}</button>`).join("")}</div>`;
    }
  }
  $("s-body").innerHTML = `<div class="hero">
      <div class="hero-arabic" lang="ar">وَمَا يَنْطِقُ عَنِ الْهَوَى</div>
      <div class="hero-title">${esc(t(KIND_TEXT[kind][1]))}</div>${body}</div>`;
  for (const b of $("s-body").querySelectorAll("[data-word]")) b.onclick = () => searchFor(b.dataset.word, b.dataset.kind);
  const clear = $("s-clear-recent");
  if (clear) clear.onclick = async () => { await invoke("clear_recent_searches"); showStart(); };
}

function notice(title, text) {
  SEARCH.atStart = false;
  $("s-body").innerHTML = `<div class="hero"><div class="hero-title small">${esc(title)}</div><div class="lead">${esc(text)}</div></div>`;
}

// ------------------------------------------------------------------ the results (_render, _update_summary)

function render() {
  SEARCH.atStart = false;
  const page = SEARCH.page;
  const body = $("s-body");
  if (page.total === 0) {
    const typed = $("q").value.trim();
    let hint = page.terms > 1 || SEARCH.whole ? t("Try fewer words, “Any word”, or turn off “Whole words only”.")
      : t("Try another spelling, or search in another language.");
    if (SEARCH.chapters.length && page.before_grades) {
      hint = n_("The words are in {n:,} hadith, none of them in the chapter chosen.", "The words are in {n:,} hadith, none of them in the chapter chosen.", page.before_grades);
    }
    if (SEARCH.grades.length && page.before_grades) {
      hint = n_("The words are in {n:,} hadith, none of them of the grades chosen ({grades}).", "The words are in {n:,} hadith, none of them of the grades chosen ({grades}).",
        page.before_grades, { grades: gradesWords(SEARCH.grades, SEARCH.every) });
    }
    body.innerHTML = `<div class="hero"><div class="hero-title small">${esc(t("Nothing found for “{strip}”", { strip: typed }))}</div><div class="lead">${esc(hint)}</div></div>`;
    updateSummary();
    return;
  }
  body.innerHTML = "";
  appendCards(page.results);
  updateSummary();
  body.scrollTop = 0;
}

// a few cards at a time, so the window never freezes (FIRST_BATCH, NEXT_BATCH)
function appendCards(results) {
  const token = ++SEARCH.token;
  const body = $("s-body");
  let i = 0;
  const step = (count) => {
    if (token !== SEARCH.token) return;
    for (const r of results.slice(i, i + count)) body.appendChild(card(r));
    i += count;
    if (i < results.length) { requestAnimationFrame(() => step(3)); return; }
    const remaining = SEARCH.page.total - SEARCH.results.length;
    if (remaining > 0) {
      const row = document.createElement("div");
      row.id = "s-more-row";
      row.className = "more-row";
      row.innerHTML = `<button class="quiet">${esc(t("Show {page_size} more ({remaining:,} left)", { page_size: Math.min(PAGE_SIZE, remaining), remaining }))}</button>`;
      row.firstChild.onclick = loadMore;
      body.appendChild(row);
    }
  };
  step(4);
}

function updateSummary() {
  const page = SEARCH.page;
  $("s-export").hidden = !(page.total && SEARCH.kind === "hadith");
  if (!page.total) { $("s-summary").textContent = ""; return; }
  let note = page.used_index ? "" : ", " + t("index unavailable");
  if (page.spellings) note += " · " + t("near spellings: {spellings}", { spellings: page.spellings });
  if (SEARCH.chapters.length && !SEARCH.grades.length) {
    note += " · " + t("in the chapter: {total:,} / {all:,}", { total: page.total, all: page.before_grades || page.total });
  }
  if (SEARCH.grades.length) {
    note += " · " + t("of the grades chosen: {total:,} / {all:,}", { total: page.total, all: page.before_grades || page.total });
    $("s-summary").dataset.tip = gradesWords(SEARCH.grades, SEARCH.every);
  } else {
    delete $("s-summary").dataset.tip;
  }
  $("s-summary").textContent = t("{total:,} hadith, {ms} ms", { total: page.total, ms: page.elapsed_ms }) + note;
}

// ------------------------------------------------------------------ narrators and scholars (S3, people_results)

async function searchPeople(text) {
  SEARCH.page = null;
  document.body.classList.add("busy");
  try {
    const d = await invoke("find_people", { kind: SEARCH.kind, text, tradition: SEARCH.tradition });
    SEARCH.found = d.rows;
    SEARCH.peopleTotal = d.total;
    SEARCH.peopleRead = d.read || 0;       // the matches find() read; the next from here (NR-B1)
    SEARCH.peopleMs = d.ms;
  } catch (e) {
    SEARCH.found = null;
    notice(t(KIND_TEXT[SEARCH.kind][1]), String(e));
    return;
  } finally {
    document.body.classList.remove("busy");
  }
  showPeople(false);
}

async function readMorePeople() {
  document.body.classList.add("busy");
  try {
    const have = new Set(SEARCH.found.map((r) => r.id));
    const d = await invoke("more_people", { text: $("q").value.trim(), tradition: SEARCH.tradition, offset: SEARCH.peopleRead, limit: PAGE_SIZE * 4 });
    SEARCH.peopleRead += d.rows.length;
    SEARCH.found.push(...d.rows.filter((r) => !have.has(r.id)));
    if (!d.rows.length) SEARCH.peopleTotal = SEARCH.found.length;   // nothing more to read: the total said more
  } finally {
    document.body.classList.remove("busy");
  }
}

function addTo(body, html) {
  const box = document.createElement("div");
  box.innerHTML = html;
  const nodes = Array.from(box.children);
  for (const n of nodes) body.appendChild(n);
  return nodes;
}

function showPeople(more) {
  SEARCH.atStart = false;
  const body = $("s-body");
  const kind = SEARCH.kind;
  const typed = $("q").value.trim();
  if (!more) {
    ++SEARCH.token;
    body.innerHTML = "";
    SEARCH.peopleShown = 0;
    body.scrollTop = 0;
  }
  $("s-export").hidden = true;                     // narrators and scholars: no export of hadith
  const found = SEARCH.found;
  if (!found.length) {
    const title = kind === "narrators" ? t("No narrators found for “{typed}”", { typed }) : t("No scholars found for “{typed}”", { typed });
    const hint = kind === "narrators"
      ? t("Names are matched by their letters, without vowels or diacritics: “Abu Hurayra”, “Ebû Hüreyre” and أبو هريرة all find him. Try fewer words, or the name he is known by.")
      : t("Try his best-known name (al-Bukhari, Buhârî, البخاري) or the title of one of his works.");
    body.innerHTML = `<div class="hero people-empty"><div class="hero-title small">${esc(title)}</div><div class="lead">${esc(hint)}</div></div>`;
    $("s-summary").textContent = "";
    return;
  }
  $("s-more-row") && $("s-more-row").remove();
  if (!more && found.every((r) => r.weak)) {
    addTo(body, `<div class="lead">${esc(t("No name reads “{typed}”. Arabic is written without its short vowels, so isnady first matches the consonants — these names have the consonants typed, but their readings sound different.", { typed }))}</div>`);
  } else if (!more && found.every((r) => r.close)) {
    addTo(body, `<div class="lead">${esc(t("Nothing is written exactly “{typed}”. These are the closest spellings: a letter typed twice or missing, or written as Turkish and English differ (h and kh, z and dh).", { typed }))}</div>`);
  }
  const batch = found.slice(SEARCH.peopleShown, SEARCH.peopleShown + PAGE_SIZE);
  batch.forEach((row, k) => {
    const index = SEARCH.peopleShown + k;
    if (row.weak && index > 0 && !found[index - 1].weak) {
      // the weaker matches, after the others, under their own heading (S4)
      addTo(body, `<div class="weaker-space"></div><div class="weaker-title">${esc(t("Weaker matches"))}</div>
        <div class="caption wrap">${esc(t("The same consonants as “{typed}”, but the names read differently — Arabic is written without its short vowels, so the consonants are matched first.", { typed }))}</div>`);
    }
    body.appendChild(kind === "narrators" ? personCard(row) : scholarCard(row));
  });
  SEARCH.peopleShown += batch.length;
  requestAnimationFrame(() => squareTags(body));
  // every match can be reached: after the ones read, the next are read when asked for (NR-B1)
  const remaining = Math.max(found.length, SEARCH.peopleTotal) - SEARCH.peopleShown;
  if (remaining > 0) {
    const [row] = addTo(body, `<div id="s-more-row" class="more-row"><button class="quiet">${esc(t("Show {page_size} more ({remaining:,} left)", { page_size: Math.min(PAGE_SIZE, remaining), remaining }))}</button></div>`);
    row.firstChild.onclick = loadMore;
  }
  const total = SEARCH.peopleTotal;
  const close = found.filter((r) => r.close).length;
  const weak = found.filter((r) => r.weak).length;
  let note = !close ? "" : (close === found.length ? " — " + t("by close spelling") : ", " + t("{n:,} by close spelling", { n: close }));
  if (weak) note += weak === found.length ? " — " + t("all weaker matches") : ", " + t("{n:,} weaker", { n: weak });
  const count = kind === "narrators" ? n_("{n:,} narrator", "{n:,} narrators", total) : n_("{n:,} scholar", "{n:,} scholars", total);
  delete $("s-summary").dataset.tip;
  $("s-summary").textContent = count + note + `, ${SEARCH.peopleMs} ms`;
}

// Qt draws a box square when its radius is more than half its height (the tags of a roomy, round theme)
function squareTags(root) {
  for (const tag of root.querySelectorAll(".p-tag:not(.checked)")) {
    tag.classList.add("checked");
    const r = parseFloat(getComputedStyle(tag).borderTopLeftRadius) || 0;
    if (r > tag.offsetHeight / 2) tag.classList.add("square");
  }
}

// [reading                Arabic]  then the full name, muted, when it says more (people_results._head)
function personHead(reading, arabic, sub) {
  const s = scriptOf("arabic");
  const scale = Number(getComputedStyle(document.documentElement).getPropertyValue("--scale")) || 1;
  const size = (s.size * 1.05 * scale).toFixed(1);
  const ar = `<span class="p-arabic${reading ? "" : " alone"}" lang="ar" dir="rtl" style="font-family:${esc(s.family)};font-size:${size}pt;color:${esc(s.color)}">${esc(arabic)}</span>`;
  return `<div class="p-top">${reading ? `<span class="p-title">${esc(reading)}</span>` : ""}${ar}</div>` +
    (sub ? `<div class="p-sub">${esc(sub)}</div>` : "");
}

// one line: tags (tradition, roles), then facts, each with its explanation on hover; the page it opens
function personFacts(tags, parts, action) {
  return `<div class="p-facts">${tags.map(([text, tip]) => `<span class="p-tag" data-tip="${esc(tip)}">${esc(text)}</span>`).join("")}` +
    parts.map(([html, tip]) => `<span class="p-fact"${tip ? ` data-tip="${esc(tip)}"` : ""}><span>${html}</span></span>`).join("") +
    `<span class="grow"></span><span class="p-go">${esc(action)} →</span></div>`;
}

function clickCard(html, tip, open, menu) {
  const el = document.createElement("div");
  el.className = "pcard";
  el.dataset.tip = tip;
  el.innerHTML = html;
  el.addEventListener("click", (e) => { if (e.button === 0) open(); });
  el.addEventListener("contextmenu", async (e) => { e.preventDefault(); popupAt(e, await menu()); });
  return el;
}

function firstPart(text) {
  return text.split(" (")[0];
}

// a narrator found by Search, from either tradition (people_results.person_card)
function personCard(row) {
  const P = WIN.people;
  const shia = row.tradition === "shia";
  const view = row.view;
  const open = () => openPerson(row.id, row.tradition);
  const menu = () => {
    const reading = view.reading || view.full_reading;
    return [act("Open in Narrators", open), SEP,
      act("Copy", null, "", { submenu: () => [
        act("His name in Arabic", () => copyText(view.arabic || row.name)),
        ...(reading ? [act("His name as read", () => copyText(reading))] : []),
        act("The whole entry heading", () => copyText(row.name))] })];
  };
  const known = view.reading || view.full_reading;
  const full = view.full_reading && view.full_reading !== known ? view.full_reading : "";
  let html = personHead(known, view.arabic || row.name, full);
  const [tag, why0] = P.tradition[row.tradition];
  const why = t(why0);
  const tags = [[t(tag), row.source ? why + "\n" + t("Source: {source}", { source: row.source }) : why]];
  const parts = [];
  const rank = row.rank;
  const dot = (color) => `<span style="color:${esc(color)}">●</span> `;
  if (shia) {
    if (rank) {
      const [ar, en0, meaning] = P.shia_ranks[rank];
      const en = t(en0);
      parts.push([dot(rankColor(P.shia_color[rank])) + esc(firstPart(en)), `${ar} — ${en}\n${t(meaning)}`]);
    } else {
      parts.push([esc(t("no judgment")), t("al-Najashi names him without judging him.")]);
    }
    if (row.madhhab) parts.push([esc(t(P.madhhabs[row.madhhab] || row.madhhab)), t("His school, as the critic gives it.")]);
  } else if (rank) {
    const [ar, en0] = P.ranks[rank];
    const en = t(en0);
    parts.push([dot(rankColor(rank)) + esc(firstPart(en)),
      t("Ibn Hajar's rank {rank} of 12: {arabic} — {meaning}", { rank, arabic: ar, meaning: en }) +
      (row.verdict ? "\n" + t("His words: {words}", { words: row.verdict }) : "")]);
  }
  if (row.tabaqa) {
    parts.push([esc(t("tabaqa {tabaqa}", { tabaqa: row.tabaqa })),
      t("Generation (tabaqa) {tabaqa} of 12 in Ibn Hajar's count: {meaning}", { tabaqa: row.tabaqa, meaning: t(P.tabaqat[row.tabaqa] || "") })]);
  }
  if (row.death) parts.push([esc(t("d. {year} AH", { year: row.death })), t("Year of death in the Hijri calendar, as the source gives it.")]);
  if (!shia) {
    parts.push([esc(row.in_chains ? t("{n:,} in chains", { n: row.in_chains }) : t("not in the imported chains")),
      t("How often he is identified in the chains of the imported books.")]);
  }
  const matched = (row.matched || []).join(" ");
  const shown = matched ? ` <span class="p-matched" style="font-family:${esc(scriptOf("arabic").family)}">${esc(matched)}</span>` : "";
  if (row.weak) parts.push([`<span class="muted">≈ ${esc(t("same consonants:"))}${shown}</span>`, t(P.tips.WEAK_TIP)]);
  else if (row.close) parts.push([`<span class="gold">≈ ${esc(t("close spelling:"))}${shown}</span>`, t(P.tips.CLOSE_TIP)]);
  else if (row.unread) parts.push([`<span class="muted">≈ ${esc(t("same consonants:"))}${shown}</span>`, t(P.tips.UNREAD_TIP)]);
  html += personFacts(tags, parts, t("Open in Narrators"));
  return clickCard(html, t("Open {name} on the Narrators page", { name: row.name }), open, menu);
}

// a scholar found by Search: who he is and what of his work is in this database (people_results.scholar_card)
function scholarCard(s) {
  const P = WIN.people;
  const open = () => show("scholars");        // the Hadith Scholars page comes with RS5
  const menu = async () => {
    const marked = await invoke("is_bookmarked", { kind: "scholar", reference: s.id });
    return [act("Open in Hadith Scholars", open), SEP,
      act(marked ? "★  " + t("Remove from bookmarks") : "☆  " + t("Add to bookmarks"),
        () => invoke("toggle_bookmark", { kind: "scholar", reference: s.id, title: s.short, subtitle: s.arabic || "" }), "", { raw: true }),
      act("Copy", null, "", { submenu: () => [
        act("His name as read", () => copyText(s.short)),
        ...(s.arabic ? [act("His name in Arabic", () => copyText(s.arabic))] : [])] })];
  };
  const sub = [s.fuller !== s.short ? s.fuller : "", s.dates].filter((x) => x).join(" · ");
  let html = personHead(s.short, s.head, sub);
  html += `<div class="caption wrap p-works">${esc(t("Works: ") + s.works.join(", "))}</div>`;
  const tags = [], parts = [];
  for (const role of s.roles) {
    const title = t(P.role_help[role][0]), meaning = t(P.role_help[role][1]);
    tags.push([title.split(" (")[0].split(" · ")[0], `${title}\n${meaning}`]);
  }
  if (!tags.length) parts.push([esc(t("nothing of his is imported yet")), t("Import his book or his grades (File → Data Sources) to see his work measured.")]);
  if (s.close) parts.push([`<span class="gold">≈ ${esc(t("close spelling"))}</span>`, t(P.tips.CLOSE_TIP)]);
  html += personFacts(tags, parts, t("Open in Hadith Scholars"));
  return clickCard(html, t("Open {name} on the Hadith Scholars page", { name: s.short }), open, menu);
}

// a narrator opened on the Narrators page, in his own tradition (open_person)
async function openPerson(id, road) {
  NARR.id = id;
  show("narrators");
  await selectNarrator(id, road === "shia" ? "shia" : "sunni");
  record(["narrator", id]);
}

// ------------------------------------------------------------------ a hadith (ResultCard)

function highlight(text, spans) {
  const chars = Array.from(text);
  let out = "", last = 0;
  for (const [s, e] of spans) {
    out += esc(chars.slice(last, s).join("")) + `<span class="gilt">${esc(chars.slice(s, e).join(""))}</span>`;
    last = e;
  }
  return out + esc(chars.slice(last).join(""));
}

function textHtml(x) {
  const body = highlight(x.text, x.spans || []);
  const caption = `<div class="caption${x.direction === "rtl" ? " right" : ""}">${esc(t(x.language))}, ${esc(x.edition_key)}</div>`;
  const style = scriptStyle(x.script);
  if (x.direction === "rtl") return `<div class="tb rtl" dir="rtl" style="${esc(style)}">${body}</div>${caption}`;
  return `<div class="translation"><div class="tb" style="${esc(style)}">${body}</div></div>${caption}`;
}

function card(r) {
  const el = document.createElement("div");
  el.className = "hcard";
  const reference = `${r.key}:${r.number}`;
  let html = `<div class="hcard-head">
      <span class="hcard-title">${esc(r.name)}</span><span class="hcard-number">${esc(t("hadith {number}", { number: r.number }))}</span>
      <span class="grow"></span>
      ${starHtml(r.bookmarked, "hadith", `${r.name} ${r.number}`)}
      <button class="qlink" data-act="export" data-tip="${esc(t("Write this hadith to a file: PDF, Word, Markdown, JSON, Excel …"))}">${esc(t("Export"))}</button>
      <button class="qlink" data-act="book" data-tip="${esc(t("Read this hadith where it stands in its book, among the hadith of its chapter"))}">${esc(t("In its book"))}</button>
      <button class="qlink" data-act="chain" data-tip="${esc(t("Open this hadith's chain of transmission"))}">${esc(t("View chain"))}</button>
    </div>`;
  if (r.related.length) {
    // other narrations of the same hadith (YZ2, takhrij); each number opens that hadith's chain
    const groups = new Map();
    for (const x of r.related) {
      if (!groups.has(x.book_name)) groups.set(x.book_name, []);
      groups.get(x.book_name).push(x);
    }
    const parts = [...groups].map(([book, items]) => esc(book) + " " + items.map((x) =>
      `<a class="num${x.kind === "same" ? "" : " probably"}" data-book="${esc(x.book)}" data-number="${esc(x.number)}">${esc(x.number)}</a>`).join(" · "));
    html += `<div class="caption wrap" data-tip="${esc(t("Other narrations of this hadith found by comparing the texts (takhrij).\nUpright numbers: the same text. Italic: probably the same report (the texts overlap and the same Companion narrates both).\nClick a number to open it."))}">${esc(t("Also narrated in:"))} ${parts.join("&nbsp;&nbsp;|&nbsp;&nbsp;")}</div>`;
  }
  if (r.verses.length) {
    html += `<div class="caption wrap" data-tip="${esc(t("The verses this hadith's translation marks as the ones it quotes or explains — the translator's mark. Click one to read it in the Qur'an, with the other hadith that name it."))}">📖 ${esc(t("Verses it names:"))} ${r.verses.map(([s, a, , label]) => `<a data-verse="${s}:${a}">${esc(label)}</a>`).join(" · ")}</div>`;
  }
  if (r.chains.length) {
    if (r.chains.length > 1) html += `<div class="chain-count">${esc(n_("{n} chain", "{n} chains of this hadith", r.chains.length))}</div>`;
    r.chains.forEach((c, i) => {
      if (r.chains.length > 1) html += `<div class="caption">${esc(t("Chain {i}", { i: i + 1 }))}</div>`;
      html += stripHtml(c, r);
    });
    html += '<div class="space6"></div>';
  }
  if (r.grades.length) {
    html += `<div class="pills">${r.grades.map(([g, v, group]) => {
      const meaning = group === null ? "" : ((WIN.grade_terms[APP.ui_language] || WIN.grade_terms.en)[group] || "");
      return `<span class="pill" data-tip="${esc(t("{grader} graded this hadith: {grade}", { grader: g, grade: v }) + (meaning ? "\n\n" + meaning : ""))}">${esc(g)}: <b>${esc(v)}</b></span>`;
    }).join("")}</div><div class="space4"></div>`;
  } else if (r.key === "bukhari" || r.key === "muslim") {
    html += `<div class="pills"><span class="pill" data-tip="${esc(t("No grader grades the hadith of the two Sahihs: they are sahih by their compiler's condition and by the community's acceptance of the two books. This is the rule of hadith science, not a grader's grade; each such hadith says so."))}">${t("In the Sahihayn: <b>sahih</b> by its compiler's condition")}</span></div><div class="space4"></div>`;
  }
  html += r.texts.map(textHtml).join("");
  const settings = `<button class="qlink" data-act="languages" data-tip="${esc(t("Choose the languages the hadith are shown in (Settings → Books and languages)"))}">${esc(t("Language settings"))}</button>`;
  if (r.folded.length) {
    const names = [...new Set(r.folded.map((x) => x.language))].join(", ");
    html += `<div class="row"><button class="qlink" data-act="fold" data-tip="${esc(t("Languages you chose not to show (Settings → Books and languages). Click to show them here."))}">${esc(t("▸  Other languages: {names}", { names }))}</button><span class="gap18"></span>${settings}</div>
      <div class="folded" hidden></div>`;
  } else {
    html += `<div class="row">${settings}</div>`;
  }
  el.innerHTML = html;
  el.querySelector('[data-act="chain"]').onclick = () => showChain(r.key, r.number);
  el.querySelector('[data-act="book"]').onclick = () => openInBook(r.hadith_id);
  el.querySelector('[data-act="export"]').onclick = later(["&File", "Export…"]);
  el.querySelector('[data-act="languages"]').onclick = later(["&Edit", "Preferences…"]);
  const fold = el.querySelector('[data-act="fold"]');
  if (fold) {
    fold.onclick = () => {
      const holder = el.querySelector(".folded");
      if (!holder.innerHTML) holder.innerHTML = r.folded.map(textHtml).join("");
      holder.hidden = !holder.hidden;
      fold.textContent = (holder.hidden ? "▸  " : "▾  ") + Array.from(fold.textContent).slice(3).join("");
    };
  }
  wireStar(el.querySelector(".star"), "hadith", reference, `${r.name} ${r.number}`, r.subtitle);
  for (const a of el.querySelectorAll("a[data-book]")) a.onclick = () => showChain(a.dataset.book, a.dataset.number);
  for (const a of el.querySelectorAll("a[data-verse]")) a.onclick = later(["&View", "Books"]);
  wireStrip(el);
  el.addEventListener("contextmenu", (e) => cardMenu(e, r, el));
  return el;
}

function starHtml(marked, kind, what) {
  return `<button class="star${marked ? " on" : ""}">${marked ? "★" : "☆"}</button>`;
}

const KIND_NAMES = { hadith: "Hadith", narrator: "Narrator", scholar: "Scholar", term: "Term", chapter: "Chapter", tree: "Genealogy" };

function starTip(marked, kind, title) {
  const what = `${t(KIND_NAMES[kind] || "")}: ${title}`;
  return marked ? t("In your bookmarks — click to take it out") + `\n${what}`
    : t("Add this hadith to your bookmarks") + `\n${what}\n` + t("Bookmarks in the sidebar keeps them, in folders of your own.");
}

function wireStar(button, kind, reference, title, subtitle) {
  const draw = (on) => {
    button.textContent = on ? "★" : "☆";
    button.classList.toggle("on", on);
    button.dataset.tip = starTip(on, kind, title);
  };
  draw(button.classList.contains("on"));
  button.onclick = async () => draw(await invoke("toggle_bookmark", { kind, reference, title, subtitle }));
}

async function openInBook(hadithId) {
  const p = await invoke("hadith_place", { hadithId });
  if (p.index) await openBookPlace(p.key, p.index);
  else await openBookPlace(p.key);
}

// ---- the chain under the title (ChainStrip): right to left, each name with its words in each language below it

function critic(v) {
  if (v.scheme === "shia") return t(v.critic_name || "al-Najashi");
  return (v.critic_name || "").includes("حجر") || !v.critic_name ? t("Ibn Hajar") : v.critic_name;
}

function personSummary(p) {
  if (!p) return "";
  const parts = [];
  const v = p.verdict;
  if (v) {
    if (v.scheme === "shia") parts.push(`${critic(v)}: ${Array.from(v.phrase).slice(0, 80).join("")}` + (v.rank ? ` (${v.rank_label})` : ""));
    else parts.push(`${t("Ibn Hajar")}: ${v.phrase}` + (v.rank ? " (" + t("rank {rank} of 12", { rank: v.rank }) + ")" : ""));
  }
  if (p.tabaqa) parts.push(t("tabaqa {tabaqa}", { tabaqa: p.tabaqa }));
  if (p.death) parts.push(t("d. {year} AH", { year: p.death }));
  return parts.join(" · ");
}

function endLabel(c) {
  const key = c.ends_with || (c.reaches_prophet ? "prophet" : "");
  if (!key) return "";
  let label = key === "imam" ? t("an Imam") : t(c.masum || key);
  if (key === "prophet") label = t("the Prophet ﷺ");
  return c.raised ? t("raised (رفعه) to {whom}", { whom: label }) : label;
}

function stripHtml(c, r) {
  const items = [];        // [{chip, words}] — each arrow kept with the chip it points to
  const words = c.words;
  let n = 0;
  const chip = (text, cls, tip, act, data, raw) => ({ text, cls, tip, act, data, raw });
  const add = (piece, own) => {
    const pieces = own !== undefined ? own : words.map((w) => (n < w.length ? w[n] : ""));
    if (own === undefined) n += 1;
    items.push({ piece, pieces });
  };
  let arrow = null;
  const push = (piece, own) => { add(Object.assign(piece, { arrow }), own); arrow = null; };
  const direct = !c.links.length && c.problem === DIRECT;
  const first = c.links[0];
  if (r.compiler && (direct || (first && TO_US.has(first.transmission || "bare")))) {
    const own = r.codes.filter((code) => r.compiler[code]).map((code) => r.compiler[code]).slice(0, words.length);
    push(chip(r.compiler.arabic, "compiler", t("The compiler of the book: the \"us\" of \"narrated to us\" is he")), own);
    arrow = "";
  }
  if (c.problem === BORROWED_PROBLEM) {
    push(chip("بهذا الإسناد", "unknown", t("Taken from the hadith before it: the book says \"وعنه\" / \"وبهذا الإسناد\". Not counted in the statistics.")));
    arrow = "";
  }
  const nameTip = t("Click to look for this name among the narrators");
  c.links.forEach((l, i) => {
    const term = `${l.term_ar}: ${t(l.term_meaning)}`;
    if (i) arrow = term;
    const p = l.person;
    if (!p && l.candidates === null) {
      push(chip(l.short, "", `${l.raw}\n${term}\n\n${nameTip}`, "name", l.short, l.raw));
    } else if (p) {
      push(chip(l.short, "", `${p.display_name}\n${personSummary(p)}\n${term}\n\n${t("Click to open this narrator.")}`, "person", p.id, l.raw));
    } else {
      const why = l.note || (l.candidates ? t("not identified: {n} narrators could have this name", { n: l.candidates })
        : t("not identified: no narrator of this name in the rijal work"));
      push(chip(l.short, "unknown", `${l.raw}\n${why}\n${term}` + (l.note ? "" : `\n\n${nameTip}`), l.note ? null : "name", l.short, l.raw));
    }
  });
  if (direct) push(chip("…", "unknown", t("The compiler gives no narrators between himself and whom the words are from")));
  if (c.problem && c.problem !== BORROWED_PROBLEM && !(c.reaches_prophet || c.ends_with)) {
    arrow = "";
    push(chip("…", "unknown", t("The chain is read as far as here; the rest is in the text. ({reason})", { reason: t(c.problem) })));
  }
  if (c.reaches_prophet || c.ends_with) {
    arrow = "";
    const key = c.ends_with || "prophet";
    const name = key === "prophet" ? "النبي ﷺ" : (WIN.masum_ar[key] || "الإمام عليه السلام");
    const node = key === "prophet" ? "muhammad" : key;
    const clickable = WIN.quraysh.includes(node);
    push(chip(name, "prophet", t("The chain reaches {whom}", { whom: endLabel(c) }) + (clickable ? "\n\n" + (key === "prophet"
      ? t("Click to read his life ﷺ and his place in the family of the Prophet ﷺ")
      : t("Click to read his life and his place in the family of the Prophet ﷺ")) : ""), clickable ? "end" : null, node));
  }
  return `<div class="strip" dir="rtl">${items.map(({ piece, pieces }) => {
    const col = `<span class="col"><span class="schip ${piece.cls}${piece.act ? " click" : ""}" lang="ar" data-tip="${esc(piece.tip)}"${piece.act ? ` data-act="${piece.act}" data-v="${esc(piece.data)}"` : ""}${piece.raw ? ` data-raw="${esc(piece.raw)}"` : ""}>${esc(piece.text)}</span>${pieces.filter(Boolean).map((w) => `<span class="word" dir="auto">${esc(w)}</span>`).join("")}</span>`;
    if (piece.arrow === null || piece.arrow === undefined) return col;
    return `<span class="pair" dir="ltr">${col}<span class="arrow" data-tip="${esc(piece.arrow)}">←</span></span>`;
  }).join("")}</div>`;
}

function wireStrip(el) {
  for (const c of el.querySelectorAll(".schip[data-act]")) {
    c.onclick = () => {
      if (c.dataset.act === "person") openNarratorPlace(Number(c.dataset.v));
      else if (c.dataset.act === "name") findNarrator(c.dataset.v);
      else if (c.dataset.act === "end") later(["&View", "Genealogies"])();
    };
    c.addEventListener("contextmenu", (e) => {
      e.preventDefault();
      e.stopPropagation();
      const raw = c.dataset.raw || c.textContent;
      const entries = [];
      if (c.dataset.act === "person") entries.push(act("Open this narrator", () => openNarratorPlace(Number(c.dataset.v))));
      entries.push(act("Look for this name among the narrators", () => findNarrator(c.dataset.act === "name" ? c.dataset.v : raw)), SEP,
        act("Copy", null, "", { submenu: () => [act("The name as the chain gives it", () => copyText(raw))] }));
      popupAt(e, entries);
    });
  }
}

// a name of a chain not identified, looked for among the narrators (find_narrator)
// a name of a chain isnady could not tell (several narrators bear it): the narrators of that name, on Search
function findNarrator(name) {
  place(["find", name], () => {
    show("search");
    searchFor(name, "narrators");
  });
}

function copyText(text) {
  if (navigator.clipboard) navigator.clipboard.writeText(text || "").catch(() => {});
}

// a menu at the pointer (gui/context_menu.popup)
function popupAt(e, entries) {
  closeMenus();
  if (!entries.some((x) => x !== SEP)) return;
  openMenu(() => entries, { x: e.clientX, y: e.clientY, top: e.clientY, alignEnd: isRtl() });
}

// right-click on a hadith (CTX1): its book, its chain, the other narrations, bookmark, copy, export
async function cardMenu(e, r, el) {
  if (e.target.closest(".schip")) return;
  e.preventDefault();
  const tb = e.target.closest(".tb");
  if (tb) {                 // a text's own menu (TextBlock): copy it
    popupAt(e, [act("Copy text", () => copyText(tb.innerText))]);
    return;
  }
  const reference = `${r.name}, ` + t("hadith {number}", { number: r.number });
  const texts = r.texts.concat(r.folded);
  const arabic = texts.filter((x) => x.direction === "rtl");
  const others = texts.filter((x) => x.direction !== "rtl");
  const whole = () => [reference, ...arabic.map((x) => x.text), ...others.map((x) => `${x.text}\n(${t(x.language)}, ${x.edition_key})`),
    ...(r.grades.length ? [r.grades.map(([g, v]) => `${g}: ${v}`).join("; ")] : [])].join("\n\n");
  const marked = await invoke("is_bookmarked", { kind: "hadith", reference: `${r.key}:${r.number}` });
  const copies = [act(t("Reference ({reference})", { reference }), () => copyText(reference), "", { raw: true }),
    ...arabic.slice(0, 1).map((x) => act("Arabic text", () => copyText(x.text))),
    ...others.map((x) => act(t("Translation: {language}", { language: t(x.language) }), () => copyText(x.text), "", { raw: true })),
    SEP, act("Everything: reference, texts and grades", () => copyText(whole()))];
  const related = r.related.slice(0, 20).map((x) => act(`${x.book_name} ${x.number}` + (x.kind === "same" ? "" : "  ~"), () => showChain(x.book, x.number), "", { raw: true }));
  const verses = r.verses.map(([s, a, b]) => act(`${s}:${a}` + (b !== a ? `-${b}` : ""), later(["&View", "Books"]), "", { raw: true }));
  const entries = [act("In its book", () => openInBook(r.hadith_id)), act("View chain", () => showChain(r.key, r.number))];
  if (related.length) entries.push(act("Also narrated in", null, "", { submenu: () => related }));
  if (verses.length) entries.push(act("Verses it names", null, "", { submenu: () => verses }));
  entries.push(SEP,
    act((marked ? "★  " + t("Remove from bookmarks") : "☆  " + t("Add to bookmarks")), async () => {
      const on = await invoke("toggle_bookmark", { kind: "hadith", reference: `${r.key}:${r.number}`, title: `${r.name} ${r.number}`, subtitle: r.subtitle });
      const star = el.querySelector(".star");
      star.classList.toggle("on", on);
      wireStar(star, "hadith", `${r.key}:${r.number}`, `${r.name} ${r.number}`, r.subtitle);
    }, "", { raw: true }),
    act("Copy", null, "", { submenu: () => copies }),
    act("Export…", later(["&File", "Export…"])));
  popupAt(e, entries);
}

// the reader's typeface of a script (theme.script_font): family and size from the settings, times the text size
// The line height is a share of the typeface's own line spacing, as QTextDocument's "line-height: 125%" is (Amiri's
// own spacing is near twice its size, so 125% of it is far more than CSS's 125% of the size).
function scriptStyle(script) {
  const s = scriptOf(script);
  const lh = (naturalLine(s.family, script === "arabic" ? "قال" : "Hxg") * s.line_height / 100).toFixed(3);
  return `font-family:${s.family};font-size:calc(${s.size}pt * var(--scale));line-height:${lh};color:${s.color}`;
}

const NATURAL = new Map();

function naturalLine(family, sample) {
  const key = family + "|" + sample;
  if (NATURAL.has(key)) return NATURAL.get(key);
  const probe = document.createElement("span");
  probe.style.cssText = `position:absolute;visibility:hidden;font-family:${family};font-size:100px;line-height:normal;white-space:nowrap`;
  probe.textContent = sample;
  document.body.appendChild(probe);
  const ratio = probe.getBoundingClientRect().height / 100 || 1.2;
  probe.remove();
  if (document.fonts && document.fonts.status === "loaded") NATURAL.set(key, ratio);
  return ratio;
}
