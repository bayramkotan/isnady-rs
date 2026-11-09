"use strict";
// Edit → Preferences (gui/preferences.py): reading text per script, theme colours, the interface font, the order of
// the sidebar, the books and the languages of the texts, the data sources, and back to zero. Every change is saved at
// once (config.set, shared with `iy config`) and the window redraws; nothing needs an Apply button.

const PF = { tab: 0, state: null, colorMode: "light", booksChanged: false };
const PF_TABS = ["Themes", "Reading text", "Colours", "Interface", "Sidebar", "Books and languages", "Data sources", "Reset"];
const PF_FONTS = ["Amiri", "Scheherazade New", "Noto Naskh Arabic", "Noto Sans Arabic", "Noto Serif", "Noto Sans",
  "DejaVu Sans", "DejaVu Serif", "Liberation Serif", "Georgia", "Times New Roman", "Arial", "Segoe UI", "Inter"];

async function openPreferences(start) {
  PF.state = await invoke("prefs_state");
  PF.booksChanged = false;
  PF.tab = (start || "").startsWith("#") ? Number(start.slice(1)) : { sources: 6, books: 5, reset: 7, themes: 0, sidebar: 4 }[start] ?? 0;
  PF.colorMode = DARK ? "dark" : "light";
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.id = "pf-back";
  back.innerHTML = `<div class="dialog pf" role="dialog">
    <div class="pf-tabs" id="pf-tabs">${PF_TABS.map((x, i) => `<button data-tab="${i}">${esc(t(x))}</button>`).join("")}</div>
    <div class="pf-page" id="pf-page"></div>
    <div class="pf-bottom"><div class="caption pf-where">${esc(t("Saved in {settings_path}  ·  also:  iy config list", { settings_path: PF.state.settings_path }))}</div>
      <button class="quiet" id="pf-reset-all">${esc(t("Reset appearance"))}</button>
      <button class="primary" id="pf-close">${esc(t("Close"))}</button></div></div>`;
  document.body.appendChild(back);
  $("pf-tabs").onclick = (e) => {
    const b = e.target.closest("button[data-tab]");
    if (b) { PF.tab = Number(b.dataset.tab); pfDraw(); }
  };
  $("pf-close").onclick = pfClose;
  $("pf-reset-all").onclick = async () => { pfLook(await invoke("prefs_reset", { prefix: "" })); await pfReload(); };
  back.addEventListener("keydown", (e) => { if (e.key === "Escape") pfClose(); });
  pfDraw();
  $("pf-close").focus();
}

async function pfClose() {
  const back = $("pf-back");
  if (back) back.remove();
  if (PF.booksChanged) {
    const s = await invoke("start");
    APP.books = s.books;
    APP.hadith = s.hadith;
    await fillSearchFilters();
    if (CURRENT) show(CURRENT);
  }
}

async function pfReload() {
  PF.state = await invoke("prefs_state");
  pfDraw();
}

// the window takes the new look at once (theme.apply, the pages redrawn)
function pfLook(look) {
  APP.settings = look;
  THEME_MODE = (PF.state && PF.state.values["view.theme"]) || THEME_MODE;
  applyTheme(THEME_MODE);
  buildNav();
  redrawPage();
}

async function pfSet(key, value) {
  try {
    const look = await invoke("prefs_set", { key, value });
    PF.state.values[key] = value;
    if (key === "view.theme") THEME_MODE = value;
    pfLook(look);
    if (key === "view.sidebar") { ORDER = (await invoke("prefs_state")).sidebar; buildNav(); buildMenuBar(); }
    await pfReload();
  } catch (e) {
    setFooter(String(e));
  }
}

function pfDraw() {
  for (const b of $("pf-tabs").children) b.classList.toggle("on", Number(b.dataset.tab) === PF.tab);
  const page = $("pf-page");
  page.innerHTML = [pfThemes, pfReading, pfColours, pfInterface, pfSidebar, pfBooks, pfSources, pfResetTab][PF.tab]();
  page.onclick = null;
  page.onchange = null;
  [pfThemesWire, pfReadingWire, pfColoursWire, pfInterfaceWire, pfSidebarWire, pfBooksWire, pfSourcesWire, pfResetWire][PF.tab]();
}

const pfValue = (key) => PF.state.values[key];
const pfDefaultButton = (attrs) => `<button class="quiet" ${attrs}>${esc(t("Default"))}</button>`;

// ---- Themes: each theme drawn as a small isnady window
function pfMini(tk) {
  return `<div class="pf-mini" style="background:${tk.window}">
    <div class="pf-mside" style="background:${tk.sidebar};color:${tk.sidebar_ink}"><b>isnady</b>
      <i style="background:${tk.sidebar_selected};border-left:2px solid ${tk.gold}"></i><i style="background:${tk.sidebar_muted}"></i><i style="background:${tk.sidebar_muted}"></i><i style="background:${tk.sidebar_muted}"></i></div>
    <div class="pf-mmain"><div class="pf-mbar"><span style="background:${tk.surface};border-color:${tk.border}"></span><span style="background:${tk.lapis}"></span></div>
      <div class="pf-mcard" style="background:${tk.surface};border-color:${tk.border}"><i style="background:${tk.ink};width:40%"></i><i style="background:${tk.pill};width:30%;height:7px"></i>
        <i style="background:${tk.muted};width:60%;margin-left:20%"></i><i style="background:${tk.muted};width:80%"></i><i style="background:${tk.muted};width:80%"></i>
        <i style="background:${tk.gilt};width:22%;border-left:2px solid ${tk.gold}"></i></div></div></div>`;
}

function pfThemes() {
  const tiles = [["system", t("Follow system"), t("Sky by day, Starry Night by night — as the system is set")]];
  const texts = WIN.theme_texts || {};
  for (const [k, name] of WIN.themes) tiles.push([k, t(name), t(texts[k] || "")]);
  const mode = pfValue("view.theme");
  const html = tiles.map(([k, name, text]) => {
    const look = k === "system" ? `<div class="pf-halves">${pfMini(WIN.tokens.sky)}${pfMini(WIN.tokens.starry)}</div>` : pfMini(WIN.tokens[k]);
    return `<button class="pf-tile${mode === k ? " on" : ""}" data-theme="${k}" data-tip="${esc(text)}">${look}
      <div class="pf-tname">${mode === k ? "✓  " : ""}${esc(name)}</div><div class="caption">${esc(text)}</div></button>`;
  }).join("");
  return `<div class="lead">${esc(t("Choose how isnady looks. The change shows at once; View → Theme offers the same list."))}</div>
    <div class="pf-tiles">${html}</div><div class="caption">${esc(t("Colours of your own (the Colours tab) apply to Lapis and Lapis Night."))}</div>`;
}

function pfThemesWire() {
  $("pf-page").onclick = (e) => {
    const b = e.target.closest("[data-theme]");
    if (b) pfSet("view.theme", b.dataset.theme);
  };
}

// ---- Reading text: each script its font, size, line spacing and colour
function pfFontField(attrs, value) {
  return `<input type="text" class="pf-font" list="pf-fonts" value="${esc(value)}" ${attrs}>`;
}

function pfReading() {
  const cards = PF.state.scripts.map((s) => {
    const fam = pfValue(`text.${s.key}.family`), size = pfValue(`text.${s.key}.size`), line = pfValue(`text.${s.key}.line_height`);
    const color = pfValue(`text.${s.key}.color`);
    const ink = getComputedStyle(document.documentElement).getPropertyValue("--ink").trim();
    const shown = color || ink;
    const rtl = s.key === "arabic";
    return `<div class="ncard pf-card" data-script="${s.key}"><div class="ncard-title">${esc(t(s.label))}</div>
      <div class="pf-grid4"><span class="flabel">${esc(t("Font"))}</span><span class="flabel">${esc(t("Size"))}</span><span class="flabel">${esc(t("Line spacing"))}</span><span class="flabel">${esc(t("Colour"))}</span><span></span>
        ${pfFontField(`data-key="text.${s.key}.family" placeholder="${esc(systemFont()[0].split(",")[0].replace(/"/g, ""))}"`, fam)}
        <span class="pf-num"><input type="number" min="6" max="48" step="0.5" value="${Number(size).toFixed(2)}" data-key="text.${s.key}.size"> pt</span>
        <span class="pf-num"><input type="number" min="80" max="250" step="5" value="${line}" data-key="text.${s.key}.line_height"> %</span>
        <label class="pf-swatch" style="background:${shown};color:${pfInk(shown)}">${esc(color ? color.toUpperCase() : t("Default") + "  " + ink.toUpperCase())}<input type="color" value="${pfHex(shown)}" data-key="text.${s.key}.color"></label>
        ${pfDefaultButton(`data-reset="text.${s.key}"`)}</div>
      <div class="pf-preview${rtl ? " rtl" : ""}" style="font-family:${esc(scriptOf(s.key).family)};font-size:${size}pt;color:${shown}">${esc(s.sample)}</div></div>`;
  }).join("");
  return `<datalist id="pf-fonts">${PF_FONTS.map((f) => `<option value="${esc(f)}">`).join("")}</datalist>
    <div class="lead">${esc(t("Each script has its own font, size, colour and line spacing. A text uses the settings of the script its language is written in. View → Reading Text Size (Ctrl + / Ctrl −) scales them all together."))}</div>${cards}`;
}

function pfHex(c) {
  return /^#[0-9a-fA-F]{6}$/.test(c) ? c : "#000000";
}

function pfInk(c) {
  const h = pfHex(c);
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);
  const l = (Math.max(r, g, b) + Math.min(r, g, b)) / 2 * 255;   // QColor.lightness
  return l > 140 ? "#000000" : "#FFFFFF";
}

function pfFieldsWire() {
  const page = $("pf-page");
  page.onchange = (e) => {
    const el = e.target.closest("[data-key]");
    if (!el) return;
    const v = el.type === "number" ? Number(el.value) : el.type === "color" ? el.value.toUpperCase() : el.value;
    pfSet(el.dataset.key, v);
  };
  page.onclick = async (e) => {
    const r = e.target.closest("[data-reset]");
    if (r) { pfLook(await invoke("prefs_reset", { prefix: r.dataset.reset })); await pfReload(); }
    const c = e.target.closest("[data-clear]");
    if (c) pfSet(c.dataset.clear, "");
  };
}

const pfReadingWire = pfFieldsWire;

// ---- Colours: the light and the dark theme each keep their own
function pfColours() {
  const base = WIN.tokens[PF.colorMode] || {};
  const rows = PF.state.color_tokens.map(([token, label]) => {
    const own = pfValue(`colors.${PF.colorMode}.${token}`);
    const shown = own || base[token] || "#000000";
    return `<span>${esc(t(label))}</span>
      <label class="pf-swatch" style="background:${shown};color:${pfInk(shown)}">${esc(own ? own.toUpperCase() : t("Default") + "  " + String(base[token] || "").toUpperCase())}<input type="color" value="${pfHex(shown)}" data-key="colors.${PF.colorMode}.${token}"></label>
      ${pfDefaultButton(`data-clear="colors.${PF.colorMode}.${token}"`)}`;
  }).join("");
  return `<div class="lead">${esc(t("The light and the dark theme keep their own colours. Choose which one to edit; the window shows the theme picked in View → Theme."))}</div>
    <div><select id="pf-mode"><option value="light"${PF.colorMode === "light" ? " selected" : ""}>${esc(t("Light theme"))}</option><option value="dark"${PF.colorMode === "dark" ? " selected" : ""}>${esc(t("Dark theme"))}</option></select></div>
    <div class="ncard pf-card"><div class="pf-grid3">${rows}</div></div>`;
}

function pfColoursWire() {
  pfFieldsWire();
  $("pf-mode").addEventListener("change", (e) => { e.stopPropagation(); PF.colorMode = e.target.value; pfDraw(); });
}

// ---- Interface: the font of menus, buttons and labels
function pfInterface() {
  const size = Number(pfValue("ui.size")) || 0;
  const mode = pfValue("view.theme");
  const choices = [["system", "Follow system"], ["light", "Light"], ["dark", "Dark"]];
  return `<div class="lead">${esc(t("The font of menus, buttons and labels. Reading text has its own settings."))}</div>
    <datalist id="pf-fonts">${PF_FONTS.map((f) => `<option value="${esc(f)}">`).join("")}</datalist>
    <div class="ncard pf-card"><div class="pf-grid3">
      <span class="flabel">${esc(t("Font"))}</span>${pfFontField(`data-key="ui.family" placeholder="${esc(systemFont()[0])}"`, pfValue("ui.family"))}${pfDefaultButton('data-reset="ui"')}
      <span class="flabel">${esc(t("Size"))}</span><span class="pf-num"><input type="number" min="0" max="24" step="0.5" value="${size.toFixed(2)}" data-key="ui.size" data-tip="${esc(t("System size"))}"> pt</span><span></span>
      <span class="flabel">${esc(t("Theme"))}</span><select data-key="view.theme">${choices.map(([k, l]) => `<option value="${k}"${mode === k ? " selected" : ""}>${esc(t(l))}</option>`).join("")}</select><span></span>
    </div></div>`;
}

function pfInterfaceWire() {
  pfFieldsWire();
  const page = $("pf-page");
  const base = page.onchange;
  page.onchange = (e) => {
    const el = e.target.closest('[data-key="ui.size"]');
    if (el && !Number(el.value)) { pfSet("ui.size", ""); return; }
    base(e);
  };
}

// ---- Sidebar: the order of the sections
function pfSidebar() {
  const rows = PF.state.sidebar.map((k, i) => `<div class="pf-row" draggable="true" data-section="${k}"${PF.pick === k ? ' data-on="1"' : ""}>${i + 1}.  ${esc(t(sectionLabel(k)))}</div>`).join("");
  return `<div class="lead">${esc(t("The order of the sections in the sidebar. Drag a section to its place, or choose it and move it with the arrows; Ctrl+1 … Ctrl+9 follow the order. The change shows at once."))}</div>
    <div class="ncard pf-card pf-order"><div class="pf-list" id="pf-list">${rows}</div>
      <div class="pf-arrows"><button class="quiet" data-move="-1">▲  ${esc(t("Move up"))}</button><button class="quiet" data-move="1">▼  ${esc(t("Move down"))}</button>${pfDefaultButton('data-clear="view.sidebar"')}</div></div>`;
}

function pfOrderWire(list, key, save) {
  list.onclick = (e) => {
    const row = e.target.closest(`[data-${key}]`);
    if (!row) return;
    PF.pick = row.dataset[key];
    for (const r of list.children) r.toggleAttribute("data-on", r === row);
  };
  let dragged = null;
  list.ondragstart = (e) => { dragged = e.target.closest(`[data-${key}]`); };
  list.ondragover = (e) => e.preventDefault();
  list.ondrop = (e) => {
    e.preventDefault();
    const at = e.target.closest(`[data-${key}]`);
    if (!dragged || !at || at === dragged) return;
    const rows = [...list.children];
    at.parentNode.insertBefore(dragged, rows.indexOf(dragged) < rows.indexOf(at) ? at.nextSibling : at);
    save([...list.children].map((r) => r.dataset[key]));
  };
  $("pf-page").addEventListener("click", (e) => {
    const b = e.target.closest("[data-move]");
    if (!b || !PF.pick) return;
    const keys = [...list.children].map((r) => r.dataset[key]);
    const i = keys.indexOf(PF.pick), j = i + Number(b.dataset.move);
    if (i < 0 || j < 0 || j >= keys.length) return;
    [keys[i], keys[j]] = [keys[j], keys[i]];
    save(keys, true);
  });
}

function pfSidebarWire() {
  pfFieldsWire();
  pfOrderWire($("pf-list"), "section", (keys) => pfSet("view.sidebar", keys.join(",")));
}

// ---- Books and languages
function pfBooks() {
  const st = PF.state;
  let books = "";
  if (!st.books.length) books = `<div class="caption">${esc(t("No books imported yet: Data sources."))}</div>`;
  for (const g of st.books) {
    books += `<div class="pf-ghead"><span class="np-ftitle">${esc(t("{group} collections", { group: t(g.label) }))}</span><span class="grow"></span>
      <button class="quiet" data-group="${g.tradition}" data-on="1">${esc(t("All on"))}</button><button class="quiet" data-group="${g.tradition}" data-on="0">${esc(t("All off"))}</button></div>
      <div class="pf-flow">${g.books.map((b) => checkRow(`${b.name} (${b.count.toLocaleString("en")})`, b.active, `data-book="${esc(b.key)}"`)).join("")}</div>`;
  }
  const picked = st.content ? st.content.split(",") : [];
  const langs = st.languages.map((l, i) => {
    const arabic = l === "Arabic";
    return `<div class="pf-row" ${arabic ? "" : 'draggable="true"'} data-lang="${esc(l)}"${PF.pick === l ? ' data-on="1"' : ""}><label class="check"><input type="checkbox" ${!picked.length || picked.includes(l) ? "checked" : ""} data-pick="${esc(l)}"><span class="box"></span></label>${i + 1}.  ${esc(t(l))}${arabic ? "   — " + esc(t("always first")) : ""}</div>`;
  }).join("");
  return `<div class="ncard pf-card"><div class="ncard-title">${esc(t("Books"))}</div>
      <div class="caption">${esc(t("A book switched off stays in the database but is left out of searches, book lists and statistics until you switch it on again. To choose a few books for one search, use Books on the Search page."))}</div>${books}</div>
    <div class="ncard pf-card"><div class="ncard-title">${esc(t("Languages of the texts"))}</div>
      <div class="caption">${esc(t("The languages each hadith is shown in, in this order: Arabic always first, the others as you put them — drag a language to its place or move it with the arrows. A text that matches your search is always shown. The same as Content in the sidebar."))}</div>
      <div class="pf-order"><div class="pf-list" id="pf-langs">${langs}</div>
        <div class="pf-arrows"><button class="quiet" data-move="-1">▲  ${esc(t("Move up"))}</button><button class="quiet" data-move="1">▼  ${esc(t("Move down"))}</button><button class="quiet" data-clear="view.language_order" data-tip="${esc(t("Arabic first, the others from A to Z"))}">${esc(t("Default"))}</button></div></div>
      <label class="pf-radio"><input type="radio" name="pf-fold" value="fold" ${st.folds ? "checked" : ""}> ${esc(t("Fold the other languages behind an arrow on each hadith (▸ click to open)"))}</label>
      <label class="pf-radio"><input type="radio" name="pf-fold" value="omit" ${st.folds ? "" : "checked"}> ${esc(t("Leave the other languages out"))}</label></div>`;
}

function pfBooksWire() {
  const page = $("pf-page");
  page.onclick = async (e) => {
    const g = e.target.closest("[data-group]");
    if (g) {
      const group = PF.state.books.find((x) => x.tradition === g.dataset.group);
      await invoke("book_active", { keys: group.books.map((b) => b.key), on: g.dataset.on === "1" });
      PF.booksChanged = true;
      await pfReload();
    }
    const c = e.target.closest("[data-clear]");
    if (c) { await pfSet(c.dataset.clear, ""); pfLanguagesChanged(); }
  };
  page.onchange = async (e) => {
    const box = e.target.closest("label[data-book]");
    if (box) {
      await invoke("book_active", { keys: [box.dataset.book], on: e.target.checked });
      PF.booksChanged = true;
      return;
    }
    if (e.target.dataset.pick) {
      const boxes = [...page.querySelectorAll("input[data-pick]")];
      const picked = boxes.filter((b) => b.checked).map((b) => b.dataset.pick);
      const value = !picked.length || picked.length === boxes.length ? "" : picked.join(",");
      await pfSet("view.content_languages", value);
      CONTENT = value ? value.split(",") : [];
      APP.content_languages = value;
      updateLanguageButtons();
      redrawPage();
      return;
    }
    if (e.target.name === "pf-fold") { await pfSet("view.other_languages", e.target.value); redrawPage(); }
  };
  pfOrderWire($("pf-langs"), "lang", async (keys) => {
    const others = keys.filter((l) => l !== "Arabic");
    const sorted = [...others].sort();
    await pfSet("view.language_order", others.join(",") === sorted.join(",") ? "" : others.join(","));
    pfLanguagesChanged();
  });
}

function pfLanguagesChanged() {
  redrawPage();
}

// ---- Data sources: sources.js (gui/sources_page.py)

// ---- Reset: the settings, the books, or everything (core.reset; iy reset)
function pfResetTab() {
  const cards = [
    ["settings", "1. Only the settings", "Delete the settings", "var(--lapis)",
      ["theme, fonts and colours", "interface and content languages", "the order of the sidebar", "where you were in each book", "recent searches"],
      ["the books", "the bookmarks", "the profiles"]],
    ["books", "2. Only the books", "Delete the books", "var(--gold)",
      ["every hadith book", "the rijal works and their narrators", "your own sources", "the search index and everything computed from them"],
      ["the settings", "the bookmarks", "the profiles"]],
    ["everything", "3. Everything", "Delete everything", "#B3261E",
      ["the books", "the settings", "the bookmarks", "the downloaded copies of the data"],
      ["the other profiles"]],
  ];
  return `<div class="lead">${esc(t("Bring isnady back to how it was: its settings, its books, or everything. Each asks once more before it deletes; nothing can be undone."))}</div>`
    + cards.map(([kind, head, title, color, goes, stays]) => `<div class="ncard pf-card"><div class="pf-rhead" style="color:${color}">${esc(t(head))}</div>
      <div class="pf-cols">${[["Deleted", goes, "✕"], ["Kept", stays, "✓"]].map(([cap, items, mark]) =>
        `<div><b>${esc(t(cap))}</b>${items.map((x) => `<div>${mark}&nbsp; ${esc(t(x))}</div>`).join("")}</div>`).join("")}</div>
      <div><button class="pf-danger" style="background:${color}" data-kind="${kind}" data-title="${esc(t(title))}">${esc(t(title))}</button></div></div>`).join("");
}

function pfResetWire() {
  $("pf-page").onclick = (e) => {
    const b = e.target.closest("[data-kind]");
    if (!b) return;
    const what = { settings: "All settings will be deleted; the books and bookmarks stay.",
      books: "All books will be deleted; the settings and bookmarks stay.",
      everything: "The books, the settings and the bookmarks will all be deleted." }[b.dataset.kind];
    confirmDialog(b.dataset.title, t(what) + "\n\n" + t("This cannot be undone. Go on?"), async () => {
      const r = await invoke("reset_data", { kind: b.dataset.kind });
      const done = b.dataset.kind === "settings" ? t("{n} settings deleted; isnady is as at its first start.", { n: r.n })
        : b.dataset.kind === "books" ? t("{n} sources deleted.", { n: r.n })
          : t("Everything deleted ({n} sources, the settings and the bookmarks).", { n: r.n });
      if (b.dataset.kind !== "books") { pfLook(r.look); ORDER = r.sidebar; buildNav(); }
      PF.booksChanged = b.dataset.kind !== "settings";
      await pfReload();
      infoDialog(b.dataset.title, `<p>${esc(done)}</p>`);
    });
  };
}

// a question with Yes and No (QMessageBox.warning)
function confirmDialog(title, text, yes) {
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.style.zIndex = 70;
  back.innerHTML = `<div class="dialog small" role="dialog"><h2>${esc(title)}</h2><div class="dialog-body"><p style="white-space:pre-wrap">⚠  ${esc(text)}</p></div>
    <div class="dialog-buttons"><button class="quiet" data-a="yes">${esc(t("Yes"))}</button> <button class="primary" data-a="no">${esc(t("No"))}</button></div></div>`;
  back.onclick = (e) => {
    const b = e.target.closest("[data-a]");
    if (!b) return;
    back.remove();
    if (b.dataset.a === "yes") yes();
  };
  document.body.appendChild(back);
  back.querySelector('[data-a="no"]').focus();
}
