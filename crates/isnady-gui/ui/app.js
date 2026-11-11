// isnady-rs — the window. Every text is English in the code and passed through t(): the Python isnady's catalogs
// (Turkish, Arabic) translate it, as they translate the Python window. Data comes from isnady-core through invoke().

"use strict";

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args || {});
const $ = (id) => document.getElementById(id);

let APP = { catalog: {}, books: [], ui_language: "en" };

// ------------------------------------------------------------------ the interface language

// {n}, {n:,} (thousands), {x:.1f}, {s:+.2f} — the format specs the Python texts use
function fill(text, values) {
  return text.replace(/\{(\w+)(?::([+]?)(,|,?\.(\d)f))?\}/g, (whole, key, sign, spec, digits) => {
    if (!(key in values)) return whole;
    const v = values[key];
    if (typeof v !== "number" || !spec) return String(v);
    if (spec === ",") return v.toLocaleString("en-US");
    if (spec.startsWith(",")) return v.toLocaleString("en-US", { minimumFractionDigits: Number(digits), maximumFractionDigits: Number(digits) });
    const s = v.toFixed(Number(digits));
    return sign && v >= 0 ? "+" + s : s;
  });
}

// texts of this window the Python isnady does not have (yet): translated here, in the same languages
const LOCAL = {
  tr: {
    "Number": "Numara",
    "LATER": "Bu sayfa isnady-rs'e sonraki sürümlerde, birer birer gelecek. O zamana kadar Python isnady (isnady ya da komut satırında iy) aynı verilerle gösteriyor.",
    "LATER_TIP": "isnady-rs {v}: henüz burada değil — Python isnady aynı verilerle gösteriyor",
    "Sunni": "Sünnî", "Shia": "Şiî", "All": "Hepsi", "Other": "Diğer",
    "Teachers": "Hocaları", "Students": "Talebeleri", "No narrators found": "Râvi bulunamadı",
    "LATER_MENU": "isnady-rs {v}: bu henüz burada değil. Python isnady aynı verilerle yapıyor: penceresinde {menu}.",
    "the file to import, with its folder; several: with ; between them": "içe aktarılacak dosya, klasörüyle; birkaç dosya: aralarına ; koyun",
    "Asking crates.io for the latest isnady…": "crates.io'ya isnady'nin en yeni sürümü soruluyor…",
    "crates.io could not be reached (no internet?). Installed: isnady {v}.": "crates.io'ya ulaşılamadı (internet yok mu?). Kurulu: isnady {v}.",
    "A newer isnady is out ({where}).": "Daha yeni bir isnady çıktı ({where}).",
    "The same in a terminal: iy-rs update --check, iy-rs update.": "Terminalde aynısı: iy-rs update --check, iy-rs update.",
    "The same in a terminal: iy-rs doctor, iy-rs doctor --fix.": "Terminalde aynısı: iy-rs doctor, iy-rs doctor --fix.",
  },
  ar: {
    "Number": "الرقم",
    "LATER": "تأتي هذه الصفحة إلى isnady-rs في الإصدارات القادمة، صفحةً بعد صفحة. وإلى ذلك الحين يعرضها isnady بلغة Python (isnady، أو iy في سطر الأوامر) على البيانات نفسها.",
    "LATER_TIP": "isnady-rs {v}: ليست هنا بعد — يعرضها isnady بلغة Python على البيانات نفسها",
    "Sunni": "سني", "Shia": "شيعي", "All": "الكل", "Other": "أخرى",
    "Teachers": "شيوخه", "Students": "تلاميذه", "No narrators found": "لم يوجد راوٍ",
    "LATER_MENU": "isnady-rs {v}: ليس هذا هنا بعد. يفعله isnady بلغة Python على البيانات نفسها: {menu} في نافذته.",
    "the file to import, with its folder; several: with ; between them": "الملف المراد استيراده مع مجلده؛ عدة ملفات: افصل بينها بـ ;",
    "Asking crates.io for the latest isnady…": "يُسأل crates.io عن أحدث إصدار من isnady…",
    "crates.io could not be reached (no internet?). Installed: isnady {v}.": "تعذّر الوصول إلى crates.io (لا اتصال؟). المثبّت: isnady {v}.",
    "A newer isnady is out ({where}).": "صدر إصدار أحدث من isnady ({where}).",
    "The same in a terminal: iy-rs update --check, iy-rs update.": "الأمر نفسه في الطرفية: iy-rs update --check، iy-rs update.",
    "The same in a terminal: iy-rs doctor, iy-rs doctor --fix.": "الأمر نفسه في الطرفية: iy-rs doctor، iy-rs doctor --fix.",
  },
};
const ENGLISH = {
  "LATER": "This page comes to isnady-rs in the next versions, one page at a time. Until then the Python isnady (isnady, or iy on the command line) shows it, on the same data.",
  "LATER_TIP": "isnady-rs {v}: not here yet — the Python isnady shows it, on the same data",
  "LATER_MENU": "isnady-rs {v}: not here yet. The Python isnady does it, on the same data: {menu} in its window.",
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
  // the user's own colours belong to the Lapis pair (theme.apply)
  if (key === "light" || key === "dark") Object.assign(tk, ((APP.settings || {}).colors || {})[key] || {});
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
  $("page-scholars").hidden = key !== "scholars";
  $("page-companions").hidden = key !== "companions";
  $("page-sira").hidden = key !== "sira";
  $("page-sahifa").hidden = key !== "sahifa";
  $("page-statistics").hidden = key !== "statistics";
  $("page-genealogy").hidden = key !== "genealogy";
  $("page-learn").hidden = key !== "learn";
  $("page-bookmarks").hidden = key !== "bookmarks";
  if (key === "narrators" && !NARR.loaded) loadNarrators();
  if (key === "books") booksRefresh();
  if (key === "scholars" && !HISTORY.opening) scholarsRefresh();
  if (key === "companions" && !HISTORY.opening) companionsRefresh();
  if (key === "sira" && !HISTORY.opening) siraRefresh();
  if (key === "sahifa" && (!HISTORY.opening || SH.stale)) sahifaRefresh();
  if (key === "statistics" && !ST.loaded) statisticsRefresh();
  if (key === "genealogy" && !HISTORY.opening) genealogyRefresh();
  if (key === "learn" && !HISTORY.opening) learnRefresh();
  if (key === "bookmarks") bookmarksRefresh();
  $("page-later").hidden = ready;
  if (!ready) {
    $("later-title").textContent = t(sectionLabel(key));
    $("later-text").textContent = t("LATER");
  }
  if (key === "chains") chainsRefresh();
  if (!HISTORY.opening) record(["section", key]);
}

// ------------------------------------------------------------------ Search

// ------------------------------------------------------------------ Isnad Chains (chains.js)

function rankColor(rank) {
  return (WIN.rank_colors[DARK ? "dark" : "light"] || {})[rank] || "var(--muted)";
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
  wireNarratorView();
  chainsBuild();
  $("c-body").addEventListener("click", chainsClick);
  setTimeout(sahifaWarm, 4000);          // the written sources counted in the background (main_window: warm)
  if (s.catch_up) catchUp();
  if (s.interrupted) SEARCH.notice = s.interrupted;
  const open = s.open || "";
  if (open.startsWith("chain:")) {
    const [, book, ...num] = open.split(":");
    showChain(book, num.join(":"));
  } else if (open.startsWith("book:")) {
    await openBookAt(open.slice(5));
  } else if (open.startsWith("inbook:")) {
    const [, book, ...num] = open.split(":");
    const d = await invoke("hadith_by_number", { book, number: num.join(":") });
    if (d.id) await openInBook(d.id);
  } else if (open.startsWith("person:")) {
    await openPersonInBook(Number(open.slice(7)));
  } else if (open.startsWith("narrator:")) {
    show("narrators");
    await loadNarrators();
    const q = open.slice(9);
    if (/^\d+$/.test(q)) await openNarrator(Number(q)); else { $("nq").value = q; await listNarrators(); }
  } else if (open.startsWith("term:")) {
    await openTerm(open.slice(5));
  } else if (open === "bookmarks") {
    show("bookmarks");
  } else if (open.startsWith("tree:")) {
    const [, which, ...rest] = open.split(":");
    await openTree(which, rest.join(":"));
  } else if (open.startsWith("genealogy:")) {
    const [, person, which] = open.split(":");
    await openGenealogy(Number(person), which);
  } else if (open.startsWith("sira:")) {
    const id = open.slice(5);
    await place(["sira", id], () => openEventAt(id));
  } else if (open === "log") {
    show("search");
    await openLog();
  } else if (open.startsWith("prefs:")) {
    show("search");
    await openPreferences(open.slice(6));
  } else if (open.startsWith("verse:")) {
    const [, s, a] = open.split(":");
    await openVerse(Number(s), a ? Number(a) : null);
  } else if (open.startsWith("stats:")) {
    const [, sec, scope] = open.split(":");
    if (scope) ST.scope = scope === "-" ? null : scope;
    show("statistics");
    statsSection(Number(sec));
  } else if (open.startsWith("sahifa:")) {
    await openSahifa(open.slice(7));
  } else if (open.startsWith("group:")) {
    await openGroup(open.slice(6));
  } else if (open.startsWith("scholar:")) {
    await openScholar(open.slice(8));
  } else if (open.startsWith("search:")) {
    $("q").value = open.slice(7);
    show("search");
    runSearch(false);
  } else {
    show("search");
  }
}

// a new version indexes the texts, reads the chains or identifies the narrators again, once, after the window has
// opened (search_page._catch_up); the Tools menu does one of them again (job "index", "chains", "narrators"): what
// it is doing is said at the foot of the window
async function catchUp(job) {
  let bar = $("catchup");
  if (!bar) {
    bar = document.createElement("div");
    bar.id = "catchup";
    bar.className = "catchup";
    document.body.appendChild(bar);
  }
  const titles = { index: "Rebuild Search Index", chains: "Read Chains Again", narrators: "Identify Narrators Again" };
  bar.textContent = job === "ai" ? t("Building the AI indexes (meaning, takhrij)…")
    : job ? t(titles[job]) + "…" : t("Bringing the chains up to date after the new version (once)…");
  await invoke("catch_up", { job: job || null });
  const poll = async () => {
    const st = await invoke("catch_up_status");
    if (st.running) {
      if (st.text) bar.textContent = st.raw ? st.text : t(st.text, { todo: st.count, missing: st.count });
      setTimeout(poll, 800);
      return;
    }
    if (st.error) {
      bar.textContent = job === "ai" ? st.error : t("The chains could not be brought up to date: {error}", { error: st.error });
      setTimeout(() => bar.remove(), 15000);
      return;
    }
    if (job === "ai") {                        // said for a while; a search by meaning shown again with the index
      bar.textContent = st.done;
      setTimeout(() => bar.remove(), 15000);
      if ($("page-search") && !$("page-search").hidden && $("q") && $("q").value.trim()) runSearch(false);
      return;
    }
    bar.remove();
    if ($("page-chains") && !$("page-chains").hidden) showOverview();   // the other pages read it when opened
    if (job === "index" && $("page-search") && !$("page-search").hidden && $("q") && $("q").value.trim()) runSearch(false);
  };
  setTimeout(poll, 800);
}

window.addEventListener("DOMContentLoaded", () => {
  start().catch((e) => { document.body.insertAdjacentHTML("beforeend", `<div class="notice">${esc(String(e))}</div>`); });
});
