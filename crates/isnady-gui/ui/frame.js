// isnady-rs — the frame of the window, as the Python window has it (gui/main_window.py _build_menus, the sidebar):
// the menu bar (File, Edit, View, Tools, Help — the same items, order, separators and shortcuts), Back and Forward,
// the sections in the order chosen with their signs, the two language buttons and the footer. What the sections,
// themes, languages, shortcuts and search tips are comes from the Python isnady (data/window.json, gen_tables.py).

"use strict";

let WIN = { sections: [], sidebar: [], signs: {}, nav_signs: {}, themes: [], languages: [], names: {}, shortcuts: [],
  search_tips: [], text_scale: [0.8, 1.6, 0.1] };
let ORDER = [];                     // the sections in the order chosen (config.sidebar_order)
let SCALE = 1.0;                    // view.text_scale
let THEME_MODE = "system";          // view.theme
let CONTENT = [];                   // view.content_languages ([] = all)
let FOOTER = "";

const READY = new Set(["search", "books", "narrators", "chains", "scholars", "companions", "sira", "sahifa", "genealogy", "learn", "bookmarks", "statistics"]);
const HISTORY = { list: [], at: -1, replaying: false, opening: false };
const PLACE_SECTION = { chain: "chains", narrator: "narrators", book: "books", chapter: "books", in_book: "books", person_in_book: "books", find: "search", tree: "genealogy", scholar: "scholars", group: "companions", sira: "sira", sahifa: "sahifa", verse: "books", genealogy: "genealogy", term: "learn" };

function sectionLabel(key) {
  const s = WIN.sections.find((x) => x[0] === key);
  return s ? s[1] : key;
}

// ------------------------------------------------------------------ the sidebar

function navSigns() {
  const key = resolvedTheme(THEME_MODE);
  return WIN.nav_signs[key] !== false;
}

function buildNav() {
  const nav = $("nav");
  nav.innerHTML = "";
  const signs = navSigns();
  for (const key of ORDER) {
    const b = document.createElement("button");
    const sign = signs ? WIN.signs[key] || "" : "";
    b.innerHTML = (sign ? `<span class="sign">${sign}</span>` : "") + `<span>${esc(t(sectionLabel(key)))}</span>`;
    b.classList.toggle("signed", !!sign);
    b.dataset.key = key;
    if (!READY.has(key)) {
      b.classList.add("later");
      b.title = t("LATER_TIP", { v: APP.version });
    }
    b.onclick = () => show(key);
    nav.appendChild(b);
  }
  for (const b of nav.children) b.classList.toggle("on", b.dataset.key === CURRENT);
}

function setFooter(message) {
  FOOTER = message;
  $("footer").textContent = (message ? message + "\n" : "") + t("version {version}", { version: APP.version });
}

function updateLanguageButtons() {
  const ui = (WIN.languages.find((l) => l[0] === APP.ui_language) || ["en", "English"])[1];
  $("ui-lang").textContent = t("Interface:") + "  " + ui;
  $("ui-lang").title = t("The interface language: the menus, pages and explanations, how every Arabic name is read (Musaddad b. Musarhad / Müsedded b. Müserhed) and the language of the glossary.");
  const content = CONTENT.length ? CONTENT.map((c) => t(c)).join(", ") : t("all languages");
  $("content-lang").textContent = t("Content:") + "  " + content;
  $("content-lang").title = t("The hadith texts are shown in: {languages}.\nA text that matches a search is shown whatever its language; each book in Books can still choose its own.", { languages: content });
}

// ------------------------------------------------------------------ Back and Forward (NAV2)

function record(entry) {
  if (HISTORY.replaying) return;
  const same = HISTORY.at >= 0 && JSON.stringify(HISTORY.list[HISTORY.at]) === JSON.stringify(entry);
  if (same) return;
  HISTORY.list.splice(HISTORY.at + 1);
  HISTORY.list.push(entry);
  if (HISTORY.list.length > 100) HISTORY.list.shift();
  HISTORY.at = HISTORY.list.length - 1;
  updateHistoryButtons();
}

// something opened inside a page: the section switch it makes is not a place of its own
async function place(entry, open) {
  const before = HISTORY.opening;
  HISTORY.opening = true;
  try {
    await open();
  } finally {
    HISTORY.opening = before;
  }
  record(entry);
}

function where(entry) {
  const key = entry[0] === "section" ? entry[1] : PLACE_SECTION[entry[0]] || "search";
  return t(sectionLabel(key));
}

function updateHistoryButtons() {
  const back = HISTORY.at > 0;
  const forward = HISTORY.at + 1 < HISTORY.list.length;
  $("back").disabled = !back;
  $("forward").disabled = !forward;
  $("back").title = back ? t("Back to {place} (Alt+Left)", { place: where(HISTORY.list[HISTORY.at - 1]) }) : t("Back (Alt+Left)");
  $("forward").title = forward ? t("Forward to {place} (Alt+Right)", { place: where(HISTORY.list[HISTORY.at + 1]) }) : t("Forward (Alt+Right)");
}

async function goHistory(step) {
  const target = HISTORY.at + step;
  if (target < 0 || target >= HISTORY.list.length) return;
  HISTORY.at = target;
  const entry = HISTORY.list[target];
  HISTORY.replaying = true;
  try {
    if (entry[0] === "section") show(entry[1]);
    else if (entry[0] === "chain") await openChain(entry[1]);
    else if (entry[0] === "in_book") await openInBook(entry[1]);
    else if (entry[0] === "find") { show("search"); searchFor(entry[1], "narrators"); }
    else if (entry[0] === "tree") await openTreeAt(entry[1], entry[2]);
    else if (entry[0] === "genealogy") await openGenealogyAt(entry[1], entry[2]);
    else if (entry[0] === "term") await openTermAt(entry[1]);
    else if (entry[0] === "narrator") await openNarratorPlace(entry[1]);
    else if (entry[0] === "book") await openBookAt(entry[1]);
    else if (entry[0] === "chapter") await openChapterAt(entry[1]);
    else if (entry[0] === "person_in_book") await openPersonInBook(entry[1]);
    else if (entry[0] === "scholar") await openScholarAt(entry[1]);
    else if (entry[0] === "group") await openGroupAt(entry[1]);
    else if (entry[0] === "sira") await openEventAt(entry[1]);
    else if (entry[0] === "sahifa") await openSahifaAt(entry[1]);
    else if (entry[0] === "verse") await openVerseAt(entry[1], entry[2]);
  } catch (_) {
    // a place that is gone (a book removed): stay where we are
  } finally {
    HISTORY.replaying = false;
  }
  updateHistoryButtons();
}

async function openNarratorPlace(id) {
  NARR.id = id;
  show("narrators");
  await selectNarrator(id);
}

// ------------------------------------------------------------------ menus (QMenuBar, QMenu)

const MENU = { stack: [], bar: null, popupFrom: null };
const SEP = "sep";

function plain(label) {
  return label.replace(/&(.)/g, "$1");
}

function mnemonicHtml(label) {
  return esc(label).replace(/&amp;(.)/, (_, c) => `<u>${c}</u>`);
}

function mnemonic(label) {
  const m = label.match(/&(.)/);
  return m ? m[1].toLocaleLowerCase() : "";
}

function act(label, run, shortcut, extra) {
  return Object.assign({ label, run, shortcut: shortcut || "" }, extra || {});
}

// what the Python window does and isnady-rs not yet: said, with where it is in the Python window
function later(path) {
  const names = path.map((p) => plain(t(p)));
  return () => infoDialog(names[names.length - 1].replace(/…$/, ""),
    `<p>${esc(t("LATER_MENU", { v: APP.version, menu: names.join(" → ") }))}</p>`);
}

function sectionShortcut(i) {
  return i < 9 ? `Ctrl+${i + 1}` : "";
}

function menus() {
  return [
    ["&File", () => [
      act("Data Sources…", () => openPreferences("sources"), "Ctrl+Shift+I"),
      act("Open Data Folder", () => invoke("open_data_folder")),
      SEP,
      act("Export…", () => exportDialog("hadith"), "Ctrl+E"),
      act("Import…", () => importDialog("book"), "Ctrl+Shift+E"),
      SEP,
      act("Profiles", null, "", { submenu: profileMenu }),
      SEP,
      act("Quit", () => invoke("quit"), "Ctrl+Q"),
    ]],
    ["&Edit", () => [
      act("Find", find, "Ctrl+F"),
      act("Copy", copySelection, "Ctrl+C"),
      SEP,
      act("Back", () => goHistory(-1), "Alt+Left"),
      act("Forward", () => goHistory(+1), "Alt+Right"),
      SEP,
      act("Preferences…", () => openPreferences(), "Ctrl+,"),
    ]],
    ["&View", () => [
      ...ORDER.map((key, i) => act(sectionLabel(key), () => show(key), sectionShortcut(i))),
      SEP,
      act("Interface Language", null, "", { submenu: uiLanguageMenu }),
      act("Content Languages", null, "", { submenu: contentLanguageMenu, onclose: applyContentLanguages }),
      act("Theme", null, "", { submenu: themeMenu }),
      act("Reading Text Size", null, "", { submenu: () => [
        act("Larger", () => changeTextScale(+1), "Ctrl++"),
        act("Smaller", () => changeTextScale(-1), "Ctrl+-"),
        act("Default Size", () => changeTextScale(0), "Ctrl+0"),
      ] }),
      SEP,
      act("Full Screen", () => invoke("full_screen"), "F11"),
    ]],
    ["&Tools", () => [
      act("Clear the Cache", clearCache),
      act("Rebuild Search Index", () => catchUp("index")),
      act("Read Chains Again", () => catchUp("chains")),
      act("Identify Narrators Again", () => catchUp("narrators")),
      SEP,
      act("Create Desktop Shortcut…", createShortcut),
      SEP,
      act("Build AI Indexes (Meaning, Takhrij)", later(["&Tools", "Build AI Indexes (Meaning, Takhrij)"])),
    ]],
    ["&Help", () => [
      act("Search Tips", searchTips),
      act("Keyboard Shortcuts", keyboardShortcuts),
      SEP,
      act("Check for Updates…", later(["&Help", "Check for Updates…"])),
      act("Check Installation…", later(["&Help", "Check Installation…"])),
      act("Log…", openLog, "Ctrl+Shift+L"),
      act("Licences", licences),
      act("isnady on GitHub", () => invoke("open_url", { url: WIN.repo_url })),
      act("Report an Issue", () => invoke("open_url", { url: WIN.issues_url })),
      SEP,
      act("About isnady", about),
    ]],
  ];
}

async function profileMenu() {
  const list = await invoke("profiles");
  return [
    ...list.map((p) => act(p.name || "Default", () => useProfile(p.id), "", { radio: true, checked: p.active, raw: !!p.name })),
    SEP,
    act("New Profile…", later(["&File", "Profiles", "New Profile…"]), "",
      { tip: "Another person's own settings, place in the books and bookmarks; the books are shared" }),
    act("Rename This Profile…", later(["&File", "Profiles", "Rename This Profile…"])),
    act("Delete a Profile…", later(["&File", "Profiles", "Delete a Profile…"])),
    SEP,
    act("Export This Profile…", later(["&File", "Profiles", "Export This Profile…"]), "",
      { tip: "One file with this profile's settings, place and bookmarks — for another computer" }),
    act("Import a Profile…", later(["&File", "Profiles", "Import a Profile…"])),
  ];
}

async function useProfile(id) {
  const now = (await invoke("profiles")).find((p) => p.active);
  if (now && now.id === id) return;
  await invoke("use_profile", { id });
  location.reload();           // the window again, with this profile's settings (main.py builds it again)
}

// English and the translated languages first; the others in a submenu of their own (UI-B1)
function uiLanguageMenu() {
  const names = WIN.names[APP.ui_language] || {};
  const item = ([code, own, english, , share]) => {
    const other = names[code] || english;
    const label = (other === own ? own : `${own}  ·  ${other}`) + (share >= 0.995 ? "" : `  (${Math.round(share * 100)}%)`);
    return act(label, () => setUiLanguage(code), "", { radio: true, checked: code === APP.ui_language, raw: true });
  };
  const first = [], rest = [];
  for (const l of WIN.languages) (l[0] === "en" || l[4] > 0 ? first : rest).push(l);
  const items = first.map(item);
  if (rest.length) items.push(SEP, act("Languages not translated yet", null, "", { submenu: () => rest.map(item) }));
  items.push(SEP, act("A text not yet translated is shown in English", null, "", { disabled: true }));
  return items;
}

async function setUiLanguage(code) {
  if (code === APP.ui_language) return;
  await invoke("set_setting", { key: "view.ui_language", value: code });
  location.reload();           // the window again in the new language
}

// the languages the texts are shown in, ticked in place: the menu stays open, the choice is applied when it closes
let CONTENT_PICK = null;

async function contentLanguageMenu() {
  const available = await invoke("content_languages");
  if (!available.length) {
    CONTENT_PICK = null;
    return [act("No texts imported yet", null, "", { disabled: true })];
  }
  const shown = new Set(CONTENT.length ? CONTENT : available);
  CONTENT_PICK = { available, boxes: new Map(available.map((l) => [l, shown.has(l)])) };
  const every = () => [...CONTENT_PICK.boxes.values()].every(Boolean);
  const boxes = available.map((lang) => act(lang, null, "", {
    box: true, isOn: () => CONTENT_PICK.boxes.get(lang),
    toggle: () => CONTENT_PICK.boxes.set(lang, !CONTENT_PICK.boxes.get(lang)),
  }));
  return [
    act("All languages", null, "", { box: true, isOn: every,
      toggle: () => { const on = !every(); for (const l of available) CONTENT_PICK.boxes.set(l, on); } }),
    SEP,
    ...boxes,
  ];
}

async function applyContentLanguages() {
  if (!CONTENT_PICK) return;
  const picked = CONTENT_PICK.available.filter((l) => CONTENT_PICK.boxes.get(l));
  const wanted = !picked.length || picked.length === CONTENT_PICK.available.length ? [] : picked;
  CONTENT_PICK = null;
  if (wanted.join(",") === CONTENT.join(",")) return;
  CONTENT = wanted;
  await invoke("set_setting", { key: "view.content_languages", value: wanted.join(",") });
  APP.content_languages = wanted.join(",");
  updateLanguageButtons();
  redrawPage();
}

function themeMenu() {
  const choices = [["system", "Follow system (Sky / Starry Night)"], ...WIN.themes];
  const items = [];
  choices.forEach(([mode, label], n) => {
    items.push(act(label, () => setTheme(mode), "", { radio: true, checked: THEME_MODE === mode }));
    if (n === 0) items.push(SEP);
  });
  items.push(SEP, act("Choose with previews…", () => openPreferences("themes")));
  return items;
}

async function setTheme(mode) {
  THEME_MODE = mode;
  await invoke("set_setting", { key: "view.theme", value: mode });
  applyTheme(mode);
  buildNav();
}

async function changeTextScale(direction) {
  const [min, max, step] = WIN.text_scale;
  const value = direction === 0 ? 1.0 : Math.round(Math.min(Math.max(SCALE + direction * step, min), max) * 100) / 100;
  SCALE = value;
  document.documentElement.style.setProperty("--scale", String(value));
  await invoke("set_setting", { key: "view.text_scale", value });
}

function find() {
  show("search");
  $("q").focus();
  $("q").select();
}

function copySelection() {
  const text = String(window.getSelection() || "");
  if (text && navigator.clipboard) navigator.clipboard.writeText(text).catch(() => document.execCommand("copy"));
  else document.execCommand("copy");
}

// the page in view drawn again after the content languages changed
function redrawPage() {
  if (CURRENT === "search" && SEARCH.results && SEARCH.results.length) runSearch(false);
  else if (CURRENT === "books" && BK.work && BK.chapter) openWork(BK.work.key, BK.chapter);
}

// ---- drawing the menus

function buildMenuBar() {
  const bar = $("menubar");
  bar.innerHTML = "";
  menus().forEach(([label, fill], i) => {
    const b = document.createElement("button");
    const text = t(label);
    b.innerHTML = mnemonicHtml(text);
    b.dataset.index = i;
    b.dataset.key = mnemonic(text);
    b.onmousedown = (e) => {
      e.preventDefault();
      if (MENU.bar === i) closeMenus(); else openBarMenu(i);
    };
    b.onmouseenter = () => { if (MENU.bar !== null && MENU.bar !== i) openBarMenu(i); };
    bar.appendChild(b);
  });
}

function openBarMenu(i) {
  closeMenus();
  const button = $("menubar").children[i];
  MENU.bar = i;
  button.classList.add("open");
  const r = button.getBoundingClientRect();
  openMenu(menus()[i][1], { x: isRtl() ? r.right : r.left, y: r.bottom, alignEnd: isRtl() });
}

function isRtl() {
  return document.documentElement.dir === "rtl";
}

async function openMenu(fill, at, parentItem) {
  const items = await fill();
  const el = document.createElement("div");
  el.className = "menu";
  const level = { el, items, index: -1, item: parentItem, child: null };
  items.forEach((it, n) => {
    if (it === SEP) {
      el.insertAdjacentHTML("beforeend", '<div class="sep"></div>');
      return;
    }
    const row = document.createElement("div");
    row.className = "mi" + (it.disabled ? " disabled" : "") + (it.submenu ? " sub" : "");
    row.dataset.n = n;
    const label = it.raw ? esc(it.label) : esc(plain(t(it.label)));
    const mark = it.box ? (it.isOn() ? "☑" : "☐") : it.checked ? (it.radio ? "●" : "✓") : "";
    row.innerHTML = `<span class="ind${it.box ? " box" : ""}">${mark}</span><span class="lb">${label}</span>` +
      `<span class="sc">${esc(it.shortcut)}</span><span class="ar">${it.submenu ? (isRtl() ? "◂" : "▸") : ""}</span>`;
    if (it.tip) row.title = t(it.tip);
    row.onmouseenter = () => hover(level, n);
    row.onmousedown = (e) => e.preventDefault();
    row.onclick = (e) => { e.stopPropagation(); choose(level, n); };
    el.appendChild(row);
  });
  document.body.appendChild(el);
  placeMenu(el, at);
  MENU.stack.push(level);
  return level;
}

function placeMenu(el, at) {
  const w = el.offsetWidth, h = el.offsetHeight;
  let x = at.alignEnd ? at.x - w : at.x;
  let y = at.y;
  if (at.flipX !== undefined && (x + w > innerWidth || x < 0)) x = at.flipX - (at.alignEnd ? 0 : w);
  if (y + h > innerHeight - 4) y = at.flipUp ? at.top - h : innerHeight - h - 4;   // a submenu moves up to fit
  y = Math.max(4, y);
  x = Math.min(Math.max(4, x), innerWidth - w - 4);
  el.style.left = x + "px";
  el.style.top = y + "px";
}

function rowOf(level, n) {
  return level.el.querySelector(`.mi[data-n="${n}"]`);
}

async function hover(level, n) {
  const depth = MENU.stack.indexOf(level);
  if (depth < 0) return;
  if (level.index !== n) {
    await closeFrom(depth + 1);
    for (const r of level.el.querySelectorAll(".mi.on")) r.classList.remove("on");
    level.index = n;
    rowOf(level, n).classList.add("on");
    const it = level.items[n];
    if (it.submenu && !it.disabled) await openSubmenu(level, n);
  }
}

async function openSubmenu(level, n) {
  const r = rowOf(level, n).getBoundingClientRect();
  const rtl = isRtl();
  level.child = await openMenu(level.items[n].submenu, { x: rtl ? r.left : r.right, y: r.top - 6, alignEnd: rtl,
    flipX: rtl ? r.right : r.left }, level.items[n]);
}

async function choose(level, n) {
  const it = level.items[n];
  if (!it || it === SEP || it.disabled) return;
  if (it.submenu) { await hover(level, n); return; }
  if (it.box) {                       // a box ticked in place; the menu stays open
    it.toggle();
    for (const [i, x] of level.items.entries()) {
      if (x !== SEP && x.box) rowOf(level, i).querySelector(".ind").textContent = x.isOn() ? "☑" : "☐";
    }
    return;
  }
  await closeMenus();
  if (it.run) it.run();
}

async function closeFrom(depth) {
  while (MENU.stack.length > depth) {
    const level = MENU.stack.pop();
    level.el.remove();
    if (level.item && level.item.onclose) await level.item.onclose();
  }
}

async function closeMenus() {
  await closeFrom(0);
  if (MENU.bar !== null) {
    const b = $("menubar").children[MENU.bar];
    if (b) b.classList.remove("open");
  }
  MENU.bar = null;
  if (MENU.popupFrom) {
    const close = MENU.popupFrom.onclose;
    MENU.popupFrom = null;
    if (close) await close();
  }
}

// a sidebar button's own menu (the two language buttons)
async function popup(button, fill, onclose) {
  await closeMenus();
  const r = button.getBoundingClientRect();
  MENU.popupFrom = { onclose };
  await openMenu(fill, { x: isRtl() ? r.right : r.left, y: r.bottom, top: r.top, flipUp: true, alignEnd: isRtl() });
}

function menuKey(e) {
  const level = MENU.stack[MENU.stack.length - 1];
  if (!level) return false;
  const rows = level.items.map((it, n) => (it !== SEP && !it.disabled ? n : -1)).filter((n) => n >= 0);
  const pos = rows.indexOf(level.index);
  if (e.key === "Escape") {
    if (MENU.stack.length > 1) closeFrom(MENU.stack.length - 1); else closeMenus();
  } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    const down = e.key === "ArrowDown";
    const next = pos < 0 ? rows[down ? 0 : rows.length - 1] : rows[(pos + (down ? 1 : rows.length - 1)) % rows.length];
    if (next !== undefined) {
      for (const r of level.el.querySelectorAll(".mi.on")) r.classList.remove("on");
      level.index = next;
      rowOf(level, next).classList.add("on");
    }
  } else if (e.key === "Enter" || e.key === " ") {
    if (level.index >= 0) {
      const it = level.items[level.index];
      if (it.submenu) openSubmenu(level, level.index); else choose(level, level.index);
    }
  } else if (e.key === (isRtl() ? "ArrowLeft" : "ArrowRight")) {
    const it = level.items[level.index];
    if (it && it.submenu) openSubmenu(level, level.index);
    else if (MENU.bar !== null) openBarMenu((MENU.bar + 1) % 5);
  } else if (e.key === (isRtl() ? "ArrowRight" : "ArrowLeft")) {
    if (MENU.stack.length > 1) closeFrom(MENU.stack.length - 1);
    else if (MENU.bar !== null) openBarMenu((MENU.bar + 4) % 5);
  } else {
    return false;
  }
  return true;
}

// ------------------------------------------------------------------ dialogs (gui/dialogs.py InfoDialog)

function infoDialog(title, body) {
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.innerHTML = `<div class="dialog" role="dialog"><h2>${esc(title)}</h2><div class="dialog-body">${body}</div>
    <div class="dialog-buttons"><button class="quiet">${esc(t("Close"))}</button></div></div>`;
  const close = () => back.remove();
  back.querySelector(".dialog-buttons button").onclick = close;
  back.onmousedown = (e) => { if (e.target === back) close(); };
  back.addEventListener("click", (e) => {
    const a = e.target.closest("a[href]");
    if (a) { e.preventDefault(); invoke("open_url", { url: a.getAttribute("href") }); }
  });
  document.body.appendChild(back);
  back.querySelector(".dialog-buttons button").focus();
}

function searchTips() {
  const tips = WIN.search_tips.slice(0, -1);              // the last text is the dialog's title
  infoDialog(t(WIN.search_tips[WIN.search_tips.length - 1]), tips.map((p) => `<p>${t(p)}</p>`).join(""));
}

function keyboardShortcuts() {
  const rows = WIN.shortcuts.map(([k, v]) => `<tr><td class="k"><b>${esc(k)}</b></td><td>${esc(t(v))}</td></tr>`).join("");
  infoDialog(t("Keyboard shortcuts"), `<table>${rows}</table>`);
}

async function licences() {
  const d = await invoke("licences");
  const tier = { A: t("may be redistributed"), B: t("may be redistributed under its conditions"), C: t("may not be redistributed") };
  const rows = d.sources.map((r) => `<p><b>${esc(r.name)}</b> (${esc(r.origin === "user" ? t("User Resource") : t("Built-in"))})<br>` +
    t("Licence: {licence}; tier {tier}, {meaning}.", { licence: esc(r.license || t("not stated")), tier: r.tier, meaning: tier[r.tier] || "" }) +
    `<br><span class="small">${esc(r.location)}</span></p>`).join("");
  const data = rows || `<p>${esc(t("No data sources imported yet."))}</p>`;
  infoDialog(t("Licences"), `
<p><b>isnady</b> — ${esc(t("MIT licence."))} <a href="${WIN.repo_url}">${WIN.repo_url}</a></p>
<p><b>Amiri</b> — ${esc(t("typeface by Khaled Hosny — SIL Open Font License 1.1."))}
<a href="https://github.com/aliftype/amiri">github.com/aliftype/amiri</a></p>
<p><b>Tauri</b> — MIT / Apache 2.0.</p>
<h3>${esc(t("Data in this database"))}</h3>
<p>${esc(t("Every hadith, grade and text belongs to the source it came from. Tier C sources and User Resources stay on this computer and are never published."))}</p>
${data}`);
}

async function about() {
  const d = await invoke("about");
  const fts = d.fts5 ? t("available") : t("not available (search scans instead)");
  infoDialog(t("About isnady"), `
<p><span class="big">isnady ${esc(APP.version)} (Rust)</span><br>
${esc(t("Hadith search built around the isnad, the chain of transmission."))}</p>
<p>${esc(t("By Bayram Kotan."))} <a href="${WIN.repo_url}">GitHub</a>, <a href="${WIN.pypi_url}">PyPI</a>,
<a href="${WIN.issues_url}">${esc(t("report an issue"))}</a>.</p>
<table>
<tr><td class="k">Rust</td><td>${esc(d.rust)} (${esc(d.os)})</td></tr>
<tr><td class="k">Tauri / WebView</td><td>${esc(d.tauri)} / ${esc(d.webview)}</td></tr>
<tr><td class="k">SQLite</td><td>${esc(d.sqlite)}, FTS5 trigram ${esc(fts)}</td></tr>
<tr><td class="k">${esc(t("Data folder"))}</td><td>${esc(d.data_dir)}</td></tr>
<tr><td class="k">${esc(t("Database"))}</td><td>${esc(d.db_path)}</td></tr>
</table>`);
}

// ------------------------------------------------------------------ keys and the mouse

function keyName(e) {
  let k = e.key === "=" ? "+" : e.key;
  if (k.length === 1) k = k.toUpperCase();
  const shift = e.shiftKey && k !== "+";            // Ctrl++ is Ctrl+Shift+= on many keyboards
  return (e.ctrlKey || e.metaKey ? "Ctrl+" : "") + (shift ? "Shift+" : "") + (e.altKey ? "Alt+" : "") + k;
}

function shortcutAction(name) {
  const fixed = {
    "Alt+ArrowLeft": () => goHistory(-1), "Alt+ArrowRight": () => goHistory(+1),
    "Ctrl+F": find, "Ctrl+Q": () => invoke("quit"), "F11": () => invoke("full_screen"),
    "Ctrl++": () => changeTextScale(+1),
    "Ctrl+-": () => changeTextScale(-1), "Ctrl+0": () => changeTextScale(0),
    "Ctrl+Shift+I": () => openPreferences("sources"), "Ctrl+E": () => exportDialog("hadith"),
    "Ctrl+Shift+E": () => importDialog("book"), "Ctrl+,": () => openPreferences(),
    "Ctrl+Shift+L": openLog,
  };
  if (fixed[name]) return fixed[name];
  const m = name.match(/^Ctrl\+([1-9])$/);
  if (m && ORDER[Number(m[1]) - 1]) return () => show(ORDER[Number(m[1]) - 1]);
  return null;
}

function onKey(e) {
  if (document.querySelector(".dialog-back")) {
    if (e.key === "Escape") document.querySelector(".dialog-back").remove();
    return;
  }
  if (MENU.stack.length && menuKey(e)) { e.preventDefault(); return; }
  if (e.altKey && !e.ctrlKey && e.key.length === 1) {
    const key = e.key.toLocaleLowerCase();
    const b = [...$("menubar").children].find((x) => x.dataset.key && x.dataset.key === key);
    if (b) { e.preventDefault(); openBarMenu(Number(b.dataset.index)); return; }
  }
  const run = shortcutAction(keyName(e));
  if (run) { e.preventDefault(); run(); }
}

// ------------------------------------------------------------------ the frame, at start

function buildFrame(s) {
  WIN = s.window;
  const saved = (s.sidebar || "").split(",").filter((k) => WIN.sidebar.includes(k));
  ORDER = [...new Set(saved)];
  for (const k of WIN.sidebar) if (!ORDER.includes(k)) ORDER.push(k);
  THEME_MODE = s.theme || "system";
  SCALE = Number(s.text_scale) || 1.0;
  CONTENT = (s.content_languages || "").split(",").filter(Boolean);
  document.documentElement.style.setProperty("--scale", String(SCALE));
  buildMenuBar();
  buildNav();
  $("back").onclick = () => goHistory(-1);
  $("forward").onclick = () => goHistory(+1);
  updateHistoryButtons();
  $("ui-lang").onclick = () => popup($("ui-lang"), uiLanguageMenu);
  $("content-lang").onclick = () => popup($("content-lang"), contentLanguageMenu, applyContentLanguages);
  updateLanguageButtons();
  setFooter(s.error ? t("Database not opened")
    : n_("{hadith:,} hadith in {n} book", "{hadith:,} hadith in {n} books", s.collections, { hadith: s.hadith_all }));
  document.addEventListener("keydown", onKey, true);
  document.addEventListener("mousedown", (e) => {
    if (MENU.stack.length && !e.target.closest(".menu") && !e.target.closest("#menubar")) closeMenus();
  });
  window.addEventListener("blur", () => { if (MENU.stack.length) closeMenus(); });
  window.addEventListener("resize", () => { if (MENU.stack.length) closeMenus(); });
  document.addEventListener("mouseup", (e) => {          // the mouse's back and forward buttons
    if (e.button === 3) { e.preventDefault(); goHistory(-1); }
    if (e.button === 4) { e.preventDefault(); goHistory(+1); }
  });
}
