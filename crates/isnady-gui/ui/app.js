// isnady-rs — the window. Every text is English in the code and passed through t(): the Python isnady's catalogs
// (Turkish, Arabic) translate it, as they translate the Python window. Data comes from isnady-core through invoke().

"use strict";

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args || {});
const $ = (id) => document.getElementById(id);

let APP = { catalog: {}, books: [], ui_language: "en" };

// ------------------------------------------------------------------ the interface language

function fill(text, values) {
  return text.replace(/\{(\w+)(:,)?\}/g, (whole, key, comma) => {
    if (!(key in values)) return whole;
    const v = values[key];
    return comma && typeof v === "number" ? v.toLocaleString("en-US") : String(v);
  });
}

// texts of this window the Python isnady does not have (yet): translated here, in the same languages
const LOCAL = {
  tr: {
    "Number": "Numara",
    "LATER": "Bu sayfa isnady-rs'e sonraki sürümlerde, birer birer gelecek. O zamana kadar Python isnady (isnady ya da komut satırında iy) aynı verilerle gösteriyor.",
    "NOT_READ": "Son içe aktarılan kitapların senedleri henüz okunmadı. Python isnady her kitap için bir kez okur: onun İsnad Zincirleri sayfasını açın ya da şunu çalıştırın: {cmd}. Sonra isnady-rs gösterir.",
    "LATER_TIP": "isnady-rs {v}: henüz burada değil — Python isnady aynı verilerle gösteriyor",
    "NOT_LAID_OUT": "Son içe aktarılan kitaplar henüz bölümlere ayrılmadı. Python isnady bunu bir kez yapar: Kitaplar sayfasını açın ya da iy book çalıştırın.",
    "Sunni": "Sünnî", "Shia": "Şiî", "All": "Hepsi", "Other": "Diğer",
    "Teachers": "Hocaları", "Students": "Talebeleri", "No narrators found": "Râvi bulunamadı",
    "LATER_MENU": "isnady-rs {v}: bu henüz burada değil. Python isnady aynı verilerle yapıyor: penceresinde {menu}.",
  },
  ar: {
    "Number": "الرقم",
    "LATER": "تأتي هذه الصفحة إلى isnady-rs في الإصدارات القادمة، صفحةً بعد صفحة. وإلى ذلك الحين يعرضها isnady بلغة Python (isnady، أو iy في سطر الأوامر) على البيانات نفسها.",
    "NOT_READ": "لم تُقرأ أسانيد الكتب المستوردة أخيرًا بعد. يقرؤها isnady بلغة Python مرة واحدة لكل كتاب: افتح صفحة سلاسل الإسناد فيه أو شغّل {cmd}. ثم يعرضها isnady-rs.",
    "LATER_TIP": "isnady-rs {v}: ليست هنا بعد — يعرضها isnady بلغة Python على البيانات نفسها",
    "NOT_LAID_OUT": "لم تُقسَّم الكتب المستوردة أخيرًا إلى أبواب بعد. يفعل isnady بلغة Python ذلك مرة واحدة: افتح صفحة الكتب فيه أو شغّل iy book.",
    "Sunni": "سني", "Shia": "شيعي", "All": "الكل", "Other": "أخرى",
    "Teachers": "شيوخه", "Students": "تلاميذه", "No narrators found": "لم يوجد راوٍ",
    "LATER_MENU": "isnady-rs {v}: ليس هذا هنا بعد. يفعله isnady بلغة Python على البيانات نفسها: {menu} في نافذته.",
  },
};
const ENGLISH = {
  "LATER": "This page comes to isnady-rs in the next versions, one page at a time. Until then the Python isnady (isnady, or iy on the command line) shows it, on the same data.",
  "NOT_READ": "The chains of the books imported last are not read yet. The Python isnady reads them once for every book: open its Isnad Chains page, or run {cmd}. Then isnady-rs shows them.",
  "LATER_TIP": "isnady-rs {v}: not here yet — the Python isnady shows it, on the same data",
  "LATER_MENU": "isnady-rs {v}: not here yet. The Python isnady does it, on the same data: {menu} in its window.",
  "NOT_LAID_OUT": "The books imported last are not laid out in chapters yet. The Python isnady does it once: open its Books page, or run iy book.",
};

function t(text, values) {
  const translated = APP.catalog[text] || (LOCAL[APP.ui_language] || {})[text] || ENGLISH[text];
  return fill(translated || text, values || {});
}

function n_(one, many, n, values) {
  return t(n === 1 ? one : many, Object.assign({ n }, values || {}));
}

function esc(s) {
  return String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
}

// ------------------------------------------------------------------ the look (gui/theme.py)
// Every token of every theme comes from the Python isnady (data/window.json): its colours, its typefaces, how round
// its corners are (roundness) and how much room its buttons have (room) — applied as theme.stylesheet applies them.

let DARK = false;
let TOKENS = {};

// "Follow system": Sky by day, Starry Night by night (theme.apply, UI-FRESH)
function resolvedTheme(name) {
  const key = (name || "system").toLowerCase();
  if (WIN.tokens && key in WIN.tokens) return key;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "starry" : "sky";
}

// Python's round(): halves to the even number
function pyRound(x) {
  const f = Math.floor(x);
  if (Math.abs(x - f - 0.5) < 1e-9) return f % 2 === 0 ? f : f + 1;
  return Math.round(x);
}

// the system's interface font, as Qt takes it (Fusion): Segoe UI 9 pt on Windows, the desktop's on Linux
function systemFont() {
  const ua = navigator.userAgent;
  if (ua.includes("Windows")) return ['"Segoe UI"', 9];
  if (ua.includes("Mac")) return ["-apple-system", 13];
  return ['"Noto Sans", "Cantarell", "DejaVu Sans", sans-serif', 10];
}

function applyTheme(name) {
  const key = resolvedTheme(name);
  const tk = Object.assign({ ui_family: "", heading_family: "", roundness: 1, room: 1, nav_signs: false, latin_family: "" },
    (WIN.tokens || {})[key] || {});
  TOKENS = tk;
  DARK = !!tk.dark;
  const root = document.documentElement.style;
  for (const k of ["window", "surface", "border", "ink", "muted", "lapis", "lapis_soft", "gold", "gilt", "sidebar",
    "sidebar_ink", "sidebar_muted", "sidebar_selected", "pill"]) {
    if (tk[k]) root.setProperty("--" + k.replace(/_/g, "-"), tk[k]);
  }
  root.setProperty("--mark", tk.gilt);
  root.setProperty("--mark-ink", tk.ink);
  root.setProperty("--on-lapis", tk.dark ? tk.window : tk.surface);
  // corners × roundness (a radius over 12 px stays), paddings × room (theme.stylesheet)
  for (const r of [4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 17]) {
    root.setProperty(`--r${r}`, (r > 12 ? r : Math.max(0, Math.min(pyRound(r * tk.roundness), 12))) + "px");
  }
  for (let p = 0; p <= 30; p++) root.setProperty(`--p${p}`, pyRound(p * tk.room) + "px");
  // the interface's typeface: the user's own, else the theme's, else the system's (+0.5 pt with a theme's face)
  const [sysFamily, sysSize] = systemFont();
  const family = (APP.settings && APP.settings["ui.family"]) || tk.ui_family;
  const size = (APP.settings && Number(APP.settings["ui.size"])) || (family ? sysSize + 0.5 : sysSize);
  root.setProperty("--ui", family ? `"${family}", ${sysFamily}, "Amiri"` : `${sysFamily}, "Amiri"`);
  root.setProperty("--ui-size", size + "pt");
  root.setProperty("--heading", `"${tk.heading_family || "Amiri"}", "Amiri"`);
  root.setProperty("--arabic-family", scriptOf("arabic").family);
  root.setProperty("--arabic-size", scriptOf("arabic").size + "pt");
}

// the reader's typeface of one script (theme.script_font): family, size in points, colour and line height
function scriptOf(script) {
  const scripts = WIN.scripts || {};
  const key = script in scripts ? script : "latin";
  const base = scripts[key] || { family: "Amiri", size: 12.5, line_height: 115 };
  const set = (APP.settings && APP.settings.text && APP.settings.text[key]) || {};
  let family = set.family !== undefined ? set.family : base.family;
  if (key === "latin" && TOKENS.latin_family && family === base.family) family = TOKENS.latin_family;
  return {
    family: family ? `"${family}", "Amiri", var(--ui)` : "var(--ui)",
    size: Number(set.size) || base.size,
    line_height: Number(set.line_height) || base.line_height,
    color: set.color || "var(--ink)",
  };
}

// tooltips as Qt draws them (QToolTip): the theme's surface, ink and border, after a moment on the thing
const TIP = { el: null, timer: null, target: null };

function wireTooltips() {
  TIP.el = document.createElement("div");
  TIP.el.className = "tooltip";
  TIP.el.hidden = true;
  document.body.appendChild(TIP.el);
  const hide = () => { clearTimeout(TIP.timer); TIP.el.hidden = true; TIP.target = null; };
  document.addEventListener("mouseover", (e) => {
    let el = e.target.closest("[data-tip], [title]");
    if (el && el.hasAttribute("title")) {
      el.dataset.tip = el.getAttribute("title");
      el.removeAttribute("title");
    }
    if (el === TIP.target) return;
    hide();
    if (!el || !el.dataset.tip) return;
    TIP.target = el;
    TIP.timer = setTimeout(() => {
      if (TIP.target !== el || !document.contains(el)) return;
      TIP.el.textContent = el.dataset.tip;
      TIP.el.hidden = false;
      const w = TIP.el.offsetWidth, h = TIP.el.offsetHeight;
      let x = TIP.x + 12, y = TIP.y + 20;
      if (x + w > innerWidth - 4) x = Math.max(4, innerWidth - w - 4);
      if (y + h > innerHeight - 4) y = Math.max(4, TIP.y - h - 6);
      TIP.el.style.left = x + "px";
      TIP.el.style.top = y + "px";
    }, 700);
  });
  document.addEventListener("mousemove", (e) => { TIP.x = e.clientX; TIP.y = e.clientY; });
  document.addEventListener("mousedown", hide, true);
  document.addEventListener("wheel", hide, true);
  document.addEventListener("keydown", hide, true);
}

// ------------------------------------------------------------------ the pages (the sidebar and menus: frame.js)

let CURRENT = null;

function show(key) {
  CURRENT = key;
  for (const b of $("nav").children) b.classList.toggle("on", b.dataset.key === key);
  const ready = READY.has(key);
  $("page-search").hidden = key !== "search";
  $("page-chains").hidden = key !== "chains";
  $("page-books").hidden = key !== "books";
  $("page-narrators").hidden = key !== "narrators";
  if (key === "narrators" && !NARR.loaded) loadNarrators();
  if (key === "books" && !BOOKS.loaded) loadShelf();
  $("page-later").hidden = ready;
  if (!ready) {
    $("later-title").textContent = t(sectionLabel(key));
    $("later-text").textContent = t("LATER");
  }
  if (key === "chains" && !CHAIN_SHOWN) showChain();
  if (!HISTORY.opening) record(["section", key]);
}

let CHAIN_SHOWN = true;

// ------------------------------------------------------------------ Search

// ------------------------------------------------------------------ Isnad Chains

function rankColor(rank) {
  return (WIN.rank_colors[DARK ? "dark" : "light"] || {})[rank] || "var(--muted)";
}

function linkCard(l) {
  const term = `<div class="term">${esc(t(l.term_meaning))}<span class="ar" lang="ar">${esc(l.term_ar)}</span></div>`;
  let body = "";
  if (l.person) {
    const p = l.person, v = p.verdict;
    const name = p.reading || p.full_reading;
    body += `<div class="full" lang="ar">${esc(p.arabic)}</div>`;
    body += `<div class="who"><b>${esc(name)}</b>${p.full_reading && p.full_reading !== name ? ` · <span class="muted">${esc(p.full_reading)}</span>` : ""}`;
    if (l.confidence !== null && l.confidence < 0.8) {
      body += ` <span class="how">${esc(l.confidence >= 0.6 ? t("Identified by the narrators around him: this teacher and student are seen with him in the chains elsewhere.") : t("Taken as the famous narrator of this name: nothing else decided between those who bear it."))}</span>`;
    }
    body += "</div>";
    if (v) {
      const rank = v.rank ? (v.shia ? esc(t(v.label)) : esc(t("rank {rank} of 12", { rank: v.rank })) + ": " + esc(t(v.label))) : "";
      const phrase = Array.from(v.phrase).length > 160 ? Array.from(v.phrase).slice(0, 160).join("") + " …" : v.phrase;
      body += `<div class="verdict" title="${esc(v.phrase)}">${esc(v.critic)}: <span class="phrase" lang="ar">${esc(phrase)}</span>` +
        (v.rank ? `<span class="dot" style="background:${v.shia ? "var(--gold)" : rankColor(v.rank)}"></span>${rank}` : "") + "</div>";
    }
    const facts = [];
    if (p.tabaqa) facts.push(t("tabaqa {tabaqa}", { tabaqa: p.tabaqa }) + (p.tabaqa_label ? ": " + t(p.tabaqa_label) : ""));
    if (p.death) facts.push(t("d. {year} AH", { year: p.death }));
    if (facts.length) body += `<div class="facts">${esc(facts.join(" · "))}</div>`;
  } else if (l.kind === "group") {
    body += `<div class="unknown">${esc(l.members.length
      ? t("A group of al-Kulayni's teachers: {members} (as al-Kulayni explained it)", { members: l.members.join("، ") })
      : t("A group of al-Kulayni's teachers"))}</div>`;
  } else if (l.kind === "previous") {
    body += `<div class="unknown">${esc(t("\"From him\": the narrator who heads the previous chain in the book"))}</div>`;
  } else if (l.kind === "vague") {
    body += `<div class="unknown">${esc(t("A narrator not named (mubham): the chain does not say who"))}</div>`;
  } else if (l.candidates !== null) {
    body += `<div class="unknown">${esc(l.candidates ? t("not identified: {n} narrators could have this name", { n: l.candidates }) : t("not identified: no narrator of this name in the rijal work"))}</div>`;
  }
  const reading = l.reading && !l.person ? `<div class="facts">${esc(l.reading)}</div>` : "";
  const meeting = l.meeting ? `<div class="meeting">⤷ tahwil (ح): ${esc(l.meeting.said
    ? "the text says the roads meet here (كلهم عن …); attested in " + l.meeting.attested + " other hadith"
    : "not said in the text: the narrator before is found narrating from this one in " + l.meeting.attested + " other hadith, in chains read whole")}</div>` : "";
  return `${meeting}<div class="link"><div class="card">${term}<div class="raw" lang="ar">${esc(l.raw)}</div>${reading}${body}</div></div>`;
}

function chainHtml(ch, i, n, data) {
  let out = n > 1 ? `<div class="chain-label">${esc(t("Chain {i}", { i: i + 1 }))}${ch.borrowed ? " — " + esc(t("from the hadith before (بهذا الإسناد), not counted in the statistics")) : ""}</div>` : "";
  out += '<div class="timeline">';
  out += `<div class="link"><div class="card"><b style="font-family:Georgia,serif;font-size:18px">${esc(data.name)}</b>
          <div class="facts">${esc(t("The compiler's book, where the chain begins"))}</div></div></div>`;
  out += ch.links.map(linkCard).join("");
  if (ch.links.length) {
    if (ch.end && !ch.raised) {
      out += `<div class="link end"><div class="card">${esc(t("The chain reaches {whom}", { whom: t(ch.end) }))}</div></div>`;
    } else if (ch.raised) {
      out += `<div class="link end"><div class="card"><span class="ar-inline" lang="ar">رفعه</span> ${esc(t("Raised (marfu'): the rest of the chain is left out — whom it reaches is not named"))}</div></div>`;
    } else if (ch.problem && !ch.borrowed) {
      out += `<div class="link"><div class="card muted">… ${esc(t("The chain is read as far as here; the rest is in the text. ({reason})", { reason: t(ch.problem) }))}</div></div>`;
    }
  }
  out += "</div>";
  if (!ch.links.length) {
    out += `<div class="notice">${esc(ch.problem ? t(ch.problem) : t("No Arabic text for this hadith, so no chain was read."))}</div>`;
    if (ch.raw) out += `<div class="rawtext" lang="ar">${esc(ch.raw)}</div>`;
  }
  for (const w of ch.words) out += `<div class="words"><span class="code">${esc(w.code)}</span>${esc(w.text)}</div>`;
  return out;
}

// what the page says first about the chain (gui/isnad_page.py _verdict)
function verdictText(ch, tradition) {
  if (!ch) return t("No Arabic text for this hadith, so no chain was read.");
  if (ch.borrowed) {
    return t("This hadith names no chain of its own: the book says it is the chain of the hadith before it (\"وعنه\", \"وبهذا الإسناد\"). The chain drawn is taken from that hadith and is not counted in the statistics.");
  }
  if (ch.problem) return t("This chain was kept whole and not split: {reason}. The full wording is shown below.", { reason: t(ch.problem) });
  const n = ch.links.length;
  const known = ch.links.filter((l) => l.person).length;
  const looked = ch.links.some((l) => l.candidates !== null);
  let text;
  if (tradition === "shia" && n) {
    if (ch.end && !ch.raised) {
      text = n_("Read as a Shia chain: {n} link between the compiler and {end}.", "Read as a Shia chain: {n} links between the compiler and {end}.", n, { end: t(ch.end) });
    } else if (ch.raised) {
      text = n_("Read as a Shia chain: {n} link, then raised (رفعه) without the rest of its links — a gap (marfu' in the Imami usage): whom it reaches is not named.",
        "Read as a Shia chain: {n} links, then raised (رفعه) without the rest of its links — a gap (marfu' in the Imami usage): whom it reaches is not named.", n);
    } else {
      text = n_("Read as a Shia chain: {n} link; it stops before a Ma'sum is named (maqtu', or the last link could not be read with confidence).",
        "Read as a Shia chain: {n} links; it stops before a Ma'sum is named (maqtu', or the last link could not be read with confidence).", n);
    }
    if (looked) text += " " + t("{known} of them identified in the Shia rijal works (al-Najashi …).", { known });
    if (ch.jamian) text += " " + t("A second road joins it (\"jami'an\"); the first road is drawn.");
    return text;
  }
  if (ch.reaches_prophet) {
    text = n_("{n} narrator between the compiler and the Prophet, as read from the text.", "{n} narrators between the compiler and the Prophet, as read from the text.", n);
    if (looked) text += " " + t("{known} of them identified in Ibn Hajar's Taqrib al-Tahdhib.", { known });
    return text;
  }
  return t("The chain as read stops before the Prophet: the report may be from a Companion or a later narrator, or the last link could not be read with confidence.");
}

async function showChain(book, number) {
  if (book) $("chain-book").value = book;
  if (number !== undefined) $("chain-number").value = number;
  CHAIN_SHOWN = true;
  const before = HISTORY.opening;
  HISTORY.opening = true;
  show("chains");
  HISTORY.opening = before;
  const view = $("chain-view");
  view.innerHTML = '<p class="muted">…</p>';
  try {
    const d = await invoke("chain", { book: $("chain-book").value, number: $("chain-number").value });
    $("chain-number").value = d.number || "";
    $("prev").disabled = !d.prev || d.prev === d.number;
    $("next").disabled = !d.next || d.next === d.number;
    $("prev").dataset.number = d.prev || "";
    $("next").dataset.number = d.next || "";
    if (d.error === "no_book") { view.innerHTML = ""; return; }
    if (d.error === "not_read") {
      view.innerHTML = `<div class="notice">${esc(t("NOT_READ", { cmd: `iy chain ${d.book} ${d.number}` }))}</div>`;
      return;
    }
    if (d.error === "no_number") {
      view.innerHTML = `<div class="notice">${esc(t("There is no hadith {number} in this book.{hint}", {
        number: d.number, hint: " " + t("Its numbers run from {first} to {last}.", { first: d.first, last: d.last }) }))}
        ${d.nearest ? `<button class="quiet" id="nearest">${esc(t("Show {number} instead", { number: d.nearest }))}</button>` : ""}</div>`;
      if (d.nearest) $("nearest").onclick = () => showChain(d.book, d.nearest);
      return;
    }
    record(["chain", $("chain-book").value, d.number]);
    const summary = verdictText(d.chains[0], d.tradition);
    view.innerHTML = `<div class="chain-title"><h1>${esc(d.name)}</h1><span class="num">${esc(t("hadith {number}", { number: d.number }))}</span>
        ${d.tradition === "shia" ? `<span class="road">${esc(t("Shia — read the Shia way"))}</span>` : ""}</div>
        <div class="summary">${esc(summary)}</div>` +
      (d.chains.length ? d.chains.map((c, i) => chainHtml(c, i, d.chains.length, d)).join("")
        : `<div class="notice">${esc(t("No Arabic text for this hadith, so no chain was read."))}</div>`);
    $("main").scrollTop = 0;
  } catch (e) {
    view.innerHTML = `<div class="notice">${esc(String(e))}</div>`;
  }
}

// ------------------------------------------------------------------ Books (gui/books_page.py)

let BOOKS = { loaded: false, works: [], road: "sunni", key: null, chapter: 0, offset: 0, limit: 40 };
const ARABIC_RE = /[\u0600-\u06FF]/;

async function loadShelf() {
  BOOKS.loaded = true;
  const s = await invoke("shelf");
  if (s.error === "not_ready") {
    $("shelf").innerHTML = `<div class="notice">${esc(t("NOT_LAID_OUT"))}</div>`;
    return;
  }
  BOOKS.works = s.works;
  const roads = [["sunni", "Sunni"], ["shia", "Shia"], ["all", "All"]];
  $("roads").innerHTML = roads.map(([k, l]) => `<button class="quiet ${BOOKS.road === k ? "on" : ""}" data-road="${k}">${esc(t(l))}</button>`).join("");
  drawShelf();
  if (!BOOKS.key && s.works.length) openBook(s.works[0].key, true);
}

function drawShelf() {
  let html = "", last = "";
  for (const w of BOOKS.works) {
    if (BOOKS.road !== "all" && w.tradition !== BOOKS.road && w.kind !== "sira" && w.kind !== "tabaqat") continue;
    const head = (BOOKS.road === "all" ? t(w.tradition === "shia" ? "Shia" : w.tradition === "sunni" ? "Sunni" : "Other") + " · " : "") + t(w.kind_label);
    if (head !== last) { html += `<div class="kind-head">${esc(head)}</div>`; last = head; }
    const count = w.kind === "hadith" || w.kind === "sahifa" ? t("{n:,} hadith", { n: w.leaves })
      : w.kind === "rijal" ? t("{n:,} entries", { n: w.leaves }) : t("{n:,} sections", { n: w.leaves });
    html += `<button class="work ${BOOKS.key === w.key ? "on" : ""}" data-key="${esc(w.key)}">
      <div class="t">${esc(w.title)}</div><div class="n">${esc(count)} · ${esc(t("{n} chapters", { n: w.chapters }))}</div></button>`;
  }
  $("shelf").innerHTML = html;
}

async function openBook(key, quiet) {
  BOOKS.key = key;
  BOOKS.chapter = 0;
  if (!quiet) record(["book", key]);
  drawShelf();
  const c = await invoke("contents", { key });
  if (c.error) { $("reader").innerHTML = ""; return; }
  const last = c.last ? `<div class="notice">${esc(t("You left off at {where}", { where: c.last.where }))}
      <button class="primary" id="go-last" style="margin-inline-start:12px">${esc(t("Go where I left this book"))}</button></div>` : "";
  $("reader").innerHTML = `<h1>${esc(c.title)}</h1><div class="crumbs">${esc(t("{n} chapters in this book", { n: c.chapters.length }))}</div>${last}
    <div class="toc">${c.chapters.map((ch) => `<button data-chapter="${ch.index}"><span class="i">${ch.index}</span>${esc(ch.trail.join(" › "))}<span class="c">(${ch.count.toLocaleString("en-US")})</span></button>`).join("")}</div>`;
  if (c.last) $("go-last").onclick = () => readChapter(key, c.last.index);
  $("main").scrollTop = 0;
}

function itemHtml(it) {
  if (it.kind === "hadith") {
    return resultCard({ key: it.key, name: "", number: it.number, grades: it.grades,
      texts: it.texts.map((x) => Object.assign({ spans: [], matched: true }, x)) });
  }
  const head = [it.label ? "#" + it.label : "", it.title || ""].filter(Boolean).join(" ");
  const paras = (it.text || "").split("\n\n").filter((p) => p.trim());
  const ar = ARABIC_RE.test(it.text || it.title || "");
  return `<div class="card entry ${ar ? "ar" : ""}">${head ? `<div class="card-head"><h3 ${ar ? 'class="ar"' : ""}>${esc(head)}</h3></div>` : ""}
    ${paras.map((p) => `<p>${esc(p.split(/\s+/).join(" "))}</p>`).join("")}</div>`;
}

async function readChapter(key, index, more) {
  if (!more) { BOOKS.offset = 0; record(["chapter", key, index]); }
  BOOKS.chapter = index;
  const d = await invoke("read_chapter", { key, index, offset: BOOKS.offset, limit: BOOKS.limit });
  if (d.error) return;
  const nav = `<div class="bar">
      <button class="quiet" id="toc-back">☰ ${esc(t("Contents"))}</button>
      <span class="grow"></span>
      <span class="muted">${esc(t("chapter {position} of {count}", { position: d.index, count: d.count }))}</span>
      <button class="quiet" id="ch-prev" ${d.index <= 1 ? "disabled" : ""}>‹ ${esc(t("Previous chapter"))}</button>
      <button class="quiet" id="ch-next" ${d.index >= d.count ? "disabled" : ""}>${esc(t("Next chapter"))} ›</button></div>`;
  const body = d.items.map(itemHtml).join("");
  if (!more) {
    $("reader").innerHTML = nav + `<div class="crumbs"><b>${esc(d.trail[0])}</b> › ${esc(d.trail.slice(1).join(" › "))} · ${esc(t("{total:,} in this chapter", { total: d.total }))}</div><div id="items">${body}</div>
      <div class="more-row"><button class="quiet" id="ch-more"></button></div>`;
    $("toc-back").onclick = () => openBook(key);
    $("ch-prev").onclick = () => readChapter(key, d.index - 1);
    $("ch-next").onclick = () => readChapter(key, d.index + 1);
    $("ch-more").onclick = () => readChapter(key, d.index, true);
    $("main").scrollTop = 0;
  } else {
    $("items").insertAdjacentHTML("beforeend", body);
  }
  BOOKS.offset += d.items.length;
  $("ch-more").hidden = BOOKS.offset >= d.total;
  $("ch-more").textContent = `${t("Next")} ${BOOKS.limit} (${BOOKS.offset} / ${d.total.toLocaleString("en-US")})`;
}

// ------------------------------------------------------------------ Narrators (gui/narrators_page.py)

let NARR = { loaded: false, road: "sunni", id: null };

function personButton(p) {
  const name = p.title || p.reading || p.arabic;
  const facts = [];
  if (p.tabaqa) facts.push(t("tabaqa {tabaqa}", { tabaqa: p.tabaqa }));
  if (p.death) facts.push(t("d. {year}", { year: p.death }));
  if (p.rank && !p.shia) facts.push(t("rank {rank}", { rank: p.rank }));
  facts.push(t("{n:,} in chains", { n: p.in_chains }));
  return `<button class="person ${NARR.id === p.id ? "on" : ""}" data-person="${p.id}">
    <div class="t">${esc(name)}</div><div class="a" lang="ar">${esc(p.arabic)}</div><div class="n">${esc(facts.join(" · "))}</div></button>`;
}

async function loadNarrators() {
  NARR.loaded = true;
  const roads = [["sunni", "Sunni"], ["shia", "Shia"], ["all", "All"]];
  $("n-roads").innerHTML = roads.map(([k, l]) => `<button class="quiet ${NARR.road === k ? "on" : ""}" data-road="${k}">${esc(t(l))}</button>`).join("");
  $("nq").placeholder = t("Name, kunya or nisba — Arabic or Latin letters");
  await listNarrators();
}

async function listNarrators() {
  const text = $("nq").value.trim();
  if (!text) {
    const f = await invoke("famous", { tradition: NARR.road });
    $("n-list").innerHTML = f.groups.map((g) => `<div class="kind-head">${esc(t(g.title))}</div>` + g.people.map(personButton).join("")).join("");
    if (!NARR.id && f.groups.length && f.groups[0].people.length) openNarrator(f.groups[0].people[0].id, true);
    return;
  }
  const r = await invoke("find_narrators", { text, tradition: NARR.road });
  $("n-list").innerHTML = r.people.length ? r.people.map(personButton).join("") : `<div class="muted">${esc(t("No narrators found"))}</div>`;
  if (r.people.length && !r.people.some((p) => p.id === NARR.id)) openNarrator(r.people[0].id, true);
}

async function openNarrator(id, quiet) {
  NARR.id = id;
  if (!quiet) record(["narrator", id]);
  for (const b of $("n-list").querySelectorAll("button[data-person]")) b.classList.toggle("on", Number(b.dataset.person) === id);
  const d = await invoke("narrator", { id });
  if (d.error) return;
  const head = `<div class="n-head"><h1>${esc(d.title || d.reading || d.arabic)}</h1><div class="a" lang="ar">${esc(d.arabic)}</div>
    ${d.full_reading ? `<div class="muted">${esc(d.full_reading)}</div>` : ""}</div>`;
  const facts = [];
  if (d.tabaqa) facts.push(t("tabaqa {tabaqa}", { tabaqa: d.tabaqa }) + (d.tabaqa_label ? ": " + t(d.tabaqa_label) : ""));
  if (d.death || d.death_note) facts.push(t("d. {year} AH", { year: d.death || "?" }) + (d.death_note ? ` (${d.death_note})` : ""));
  facts.push(t("{n:,} in chains", { n: d.in_chains }));
  if (d.source) facts.push(d.source);
  let html = head + `<div class="facts-row">${esc(facts.join(" · "))}</div>`;
  if (d.other_names.length) html += `<div class="chips">${d.other_names.slice(0, 12).map((n) => `<span class="chip"><span class="ar" lang="ar">${esc(n)}</span></span>`).join("")}</div>`;
  for (const v of d.verdicts) {
    const color = v.shia ? "var(--gold)" : rankColor(v.rank);
    const rank = v.rank ? (v.shia ? t(v.label) + (v.madhhab ? ", " + t(v.madhhab) : "") : t("rank {rank} of 12", { rank: v.rank }) + ": " + t(v.label)) : t("no judgment");
    html += `<div class="verdict-card" style="border-inline-start-color:${color}"><b>${esc(v.critic)}</b>, <i>${esc(v.work)}</i>
      <div class="phrase" lang="ar">${esc(v.phrase)}</div><div class="muted">${esc(rank)}</div></div>`;
  }
  if (d.marks.length) html += `<h3 class="sec">${esc(t("Books"))}</h3><div class="chips">${d.marks.map(([m, meaning]) => `<span class="chip"><span class="ar">${esc(m)}</span> ${esc(t(meaning))}</span>`).join("")}</div>`;
  const people = (list) => list.map((r) => `<span class="chip" data-person="${r.id}"><span class="ar" lang="ar">${esc(r.name)}</span> (${r.count})</span>`).join("");
  if (d.teachers.length) html += `<h3 class="sec">${esc(t("Teachers"))}</h3><div class="chips">${people(d.teachers)}</div>`;
  if (d.students.length) html += `<h3 class="sec">${esc(t("Students"))}</h3><div class="chips">${people(d.students)}</div>`;
  if (d.compilers.length) html += `<div class="muted">${esc(d.compilers.map((c) => `${c.book} (${c.count})`).join(" · "))}</div>`;
  if (d.hadith.length) {
    html += `<h3 class="sec">${esc(t("{n:,} hadith", { n: d.hadith_total }))}</h3><div class="chips">` +
      d.hadith.map((h) => `<span class="chip" data-book="${esc(h.key)}" data-number="${esc(h.number)}">${esc(h.book)} ${esc(h.number)}</span>`).join("") +
      (d.hadith_total > d.hadith.length ? " …" : "") + "</div>";
  }
  $("n-view").innerHTML = html;
  $("main").scrollTop = 0;
}

// ------------------------------------------------------------------ start

async function start() {
  const s = await invoke("start");
  APP = s;
  document.documentElement.lang = s.ui_language;
  document.documentElement.dir = s.rtl ? "rtl" : "ltr";
  WIN = s.window;
  applyTheme(s.theme);
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => { applyTheme(THEME_MODE); buildNav(); });
  buildFrame(s);
  wireTooltips();
  // the web view's own menu (Back, Reload, Inspect Element) is not the program's: a widget without a menu of its own
  // shows none, as in the Python window; a text field keeps the system's (cut, copy, paste)
  document.addEventListener("contextmenu", (e) => {
    if (!e.target.closest("input, textarea")) e.preventDefault();
  });
  try { await document.fonts.ready; } catch (_) { /* the fonts load later: the lines are measured then */ }
  buildSearchPage();
  await fillSearchFilters();
  showStart();
  for (const b of s.books) {
    const opt = `<option value="${esc(b.key)}">${esc(b.name)} (${b.count.toLocaleString("en-US")})</option>`;
    $("chain-book").insertAdjacentHTML("beforeend", opt);
  }
  $("book-label").textContent = t("Book");
  $("number-label").textContent = t("Number");
  $("show-chain").textContent = t("Show chain");
  $("random").textContent = t("Random hadith");
  $("prev").textContent = "‹ " + t("Previous");
  $("next").textContent = t("Next") + " ›";
  $("n-roads").addEventListener("click", (e) => {
    const b = e.target.closest("button[data-road]");
    if (!b) return;
    NARR.road = b.dataset.road;
    for (const x of $("n-roads").children) x.classList.toggle("on", x === b);
    listNarrators();
  });
  let typing = null;
  $("nq").addEventListener("input", () => { clearTimeout(typing); typing = setTimeout(listNarrators, 250); });
  $("n-list").addEventListener("click", (e) => {
    const b = e.target.closest("[data-person]");
    if (b) openNarrator(Number(b.dataset.person));
  });
  $("n-view").addEventListener("click", (e) => {
    const p = e.target.closest("[data-person]");
    if (p) { openNarrator(Number(p.dataset.person)); return; }
    const h = e.target.closest("[data-book]");
    if (h) showChain(h.dataset.book, h.dataset.number);
  });
  $("roads").addEventListener("click", (e) => {
    const b = e.target.closest("button[data-road]");
    if (!b) return;
    BOOKS.road = b.dataset.road;
    for (const x of $("roads").children) x.classList.toggle("on", x === b);
    drawShelf();
  });
  $("shelf").addEventListener("click", (e) => {
    const b = e.target.closest("button[data-key]");
    if (b) openBook(b.dataset.key);
  });
  $("reader").addEventListener("click", (e) => {
    const ch = e.target.closest("button[data-chapter]");
    if (ch) { readChapter(BOOKS.key, Number(ch.dataset.chapter)); return; }
    const b = e.target.closest("button[data-book]");
    if (b) showChain(b.dataset.book, b.dataset.number);
  });
  $("show-chain").onclick = () => showChain();
  $("chain-number").addEventListener("keydown", (e) => { if (e.key === "Enter") showChain(); });
  $("chain-book").onchange = () => showChain($("chain-book").value, "");
  $("prev").onclick = () => showChain(undefined, $("prev").dataset.number);
  $("next").onclick = () => showChain(undefined, $("next").dataset.number);
  $("random").onclick = async () => {
    const n = await invoke("random_number", { book: $("chain-book").value });
    if (n) showChain(undefined, n);
  };
  CHAIN_SHOWN = false;
  const open = s.open || "";
  if (open.startsWith("chain:")) {
    const [, book, ...num] = open.split(":");
    showChain(book, num.join(":"));
  } else if (open.startsWith("book:")) {
    const [, key, n] = open.split(":");
    show("books");
    await loadShelf();
    await openBook(key);
    if (n) await readChapter(key, Number(n));
  } else if (open.startsWith("narrator:")) {
    show("narrators");
    await loadNarrators();
    const q = open.slice(9);
    if (/^\d+$/.test(q)) await openNarrator(Number(q)); else { $("nq").value = q; await listNarrators(); }
  } else if (open.startsWith("search:")) {
    $("q").value = open.slice(7);
    show("search");
    runSearch(false);
  } else {
    show("search");
  }
}

window.addEventListener("DOMContentLoaded", () => {
  start().catch((e) => { document.body.insertAdjacentHTML("beforeend", `<div class="notice">${esc(String(e))}</div>`); });
});
