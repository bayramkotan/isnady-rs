// File → Export… and File → Import… (gui/transfer_dialog.py, EX1): anything isnady holds, in the format chosen, with
// a preview before it is written. What, how and where are asked here; isnady_core::transfer does the rest.

const XF_KIND_TEXT = {
  hadith: "Hadith (by number)", book: "A book or a chapter", search: "Search results", bookmarks: "Bookmarks",
  narrators: "Narrators", network: "Isnad network (who narrated from whom)", settings: "Settings", profile: "Profile",
  backup: "Everything of mine (backup)", statistics: "Statistics",
};
const XF_KIND_HELP = {
  hadith: "The hadith named, with their chapter, texts, grades and chains (Arabic as written, then in Turkish and English).",
  book: "A whole book or one chapter. \"isnady book\" keeps it so that it can be imported again as a book.",
  search: "The hadith a search finds (up to 1,000), as on the Search page.",
  bookmarks: "Your bookmarks with their folders and notes. JSON is isnady's own and comes back with the folders as they were.",
  narrators: "The narrators of a tradition: their names, tabaqa, death, rank and verdict, and how often they are in the chains.",
  network: "Who narrated from whom in the chains, and how often: for Gephi, yEd or a spreadsheet.",
  settings: "The settings you changed.",
  profile: "A profile with its settings, place and bookmarks.",
  backup: "This profile's settings, place and bookmarks and the list of the books imported: what makes your isnady yours, in one file.",
  statistics: "The results of the Statistics page — the corpus, the model, the laboratory and the graders — as data. Each chart and table on the Statistics page has its own ⤓ too, and its Export button writes a section or all of them as PDF, a web page or Excel.",
};
const XF = { job: null };

// several at once, or all of them: a list to tick, with All, None and — for books — Sunni and Shia
function xfCheckList(id, items, checked, groups) {
  const picks = [["All", "all"], ["None", "none"]].concat(groups ? [["Sunni", "sunni"], ["Shia", "shia"]] : []);
  return `<div class="xf-list" id="${id}"><div class="xf-items">${items.map(([key, text, group]) =>
    `<label class="src-check"><input type="checkbox" data-key="${esc(key)}" data-group="${esc(group || "")}"${checked.includes(key) ? " checked" : ""}> ${esc(text)}</label>`).join("")}</div>
    <div class="xf-picks">${picks.map(([text, pick]) => `<button class="quiet small" data-pick="${pick}">${esc(t(text))}</button>`).join("")}
    <span class="grow"></span><span class="caption xf-count"></span></div></div>`;
}

function xfKeys(el) { return [...el.querySelectorAll("input:checked")].map((i) => i.dataset.key); }
function xfEvery(el) { const all = el.querySelectorAll("input").length; return all > 0 && xfKeys(el).length === all; }

function xfWireList(el, changed) {
  const count = () => {
    el.querySelector(".xf-count").textContent = t("{n} of {total} chosen", { n: xfKeys(el).length, total: el.querySelectorAll("input").length });
    if (changed) changed();
  };
  el.addEventListener("change", count);
  el.addEventListener("click", (e) => {
    const b = e.target.closest("[data-pick]");
    if (!b) return;
    const pick = b.dataset.pick;
    for (const i of el.querySelectorAll("input")) i.checked = pick === "all" ? true : pick === "none" ? false : i.dataset.group === pick;
    count();
  });
  el.querySelector(".xf-count").textContent = t("{n} of {total} chosen", { n: xfKeys(el).length, total: el.querySelectorAll("input").length });
}

function xfClock(seconds) {
  seconds = Math.max(0, Math.floor(seconds));
  if (seconds < 60) return t("{s} s", { s: seconds });
  if (seconds < 3600) return t("{m} min {s} s", { m: Math.floor(seconds / 60), s: seconds % 60 });
  return t("{h} h {m} min", { h: Math.floor(seconds / 3600), m: Math.floor((seconds % 3600) / 60) });
}

// "12,345 / 99,567 · 1 min 20 s gone · about 3 min left" (jobs.describe)
function xfDescribe(st) {
  let text = `${st.done.toLocaleString("en-US")} / ${st.total.toLocaleString("en-US")} · ` + t("{t} gone", { t: xfClock(st.gone) });
  if (st.done > 0 && st.done < st.total && st.gone > 3) text += " · " + t("about {t} left", { t: xfClock(st.gone * (st.total - st.done) / st.done) });
  return text;
}

// the bar at the bottom of the window: an export goes on when its dialog is closed
function xfBar() {
  let bar = $("xf-bar");
  if (!bar) {
    bar = document.createElement("div");
    bar.id = "xf-bar";
    bar.className = "catchup xf-bar";
    bar.innerHTML = `<span class="xf-bar-text"></span> <button class="qlink" data-a="stop">${esc(t("Stop"))}</button> <button class="qlink" data-a="show" hidden>${esc(t("Show in folder"))}</button> <button class="qlink" data-a="close" hidden>✕</button>`;
    document.body.appendChild(bar);
    bar.onclick = (e) => {
      const a = e.target.closest("[data-a]");
      if (!a) return;
      if (a.dataset.a === "stop") { invoke("xfer_stop"); a.disabled = true; }
      if (a.dataset.a === "show") invoke("xfer_show", { path: bar.dataset.path });
      if (a.dataset.a === "close") bar.remove();
    };
  }
  return bar;
}

async function xfWatch(onTick, onDone) {
  for (;;) {
    await new Promise((ok) => setTimeout(ok, 400));
    const st = await invoke("xfer_status");
    if (st.running) { onTick(st); continue; }
    onDone(st.result || { ok: false, message: "" });
    return;
  }
}

async function exportDialog(kind, preset) {
  closeMenus();
  preset = preset || {};
  let d;
  try { d = await invoke("xfer_page"); } catch (e) { infoDialog(t("Export"), `<p>${esc(String(e))}</p>`); return; }
  const old = $("xf-back");
  if (old) old.remove();
  const label = (f) => t(d.labels[f] || f);
  const books = d.books.map((w) => [w.key, `${w.title} (${w.key})`, w.tradition]);
  const first = preset.book || (d.books[0] ? d.books[0].key : "");
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.id = "xf-back";
  const row = (name, body, scope) => `<div class="xf-row"${scope ? ` data-scope="${scope}"` : ""}><div class="xf-name">${esc(name)}</div><div class="xf-field">${body}</div></div>`;
  back.innerHTML = `<div class="dialog xf" role="dialog"><div class="pd-title">${esc(t("Export"))}</div>
    <div class="xf-form">
    ${row(t("What"), `<select id="xf-kind">${d.kinds.map((k) => `<option value="${k.key}">${esc(t(XF_KIND_TEXT[k.key]))}</option>`).join("")}</select>`)}
    ${row("", `<div class="caption" id="xf-help"></div>`)}
    ${row(t("Hadith"), `<input type="text" id="xf-refs" spellcheck="false" placeholder="bukhari:1  bukhari:5-10  muslim:8" value="${esc(preset.refs || "")}">`, "hadith")}
    ${row(t("Books"), xfCheckList("xf-books", books, [first], true), "book")}
    ${row(t("Chapter"), `<select id="xf-chapter"></select>`, "book")}
    ${row("", `<label class="src-check" id="xf-separate-row"><input type="checkbox" id="xf-separate"> ${esc(t("Each book in its own file (in a folder)"))}</label>`, "book")}
    ${row(t("Search"), `<input type="text" id="xf-query" placeholder="${esc(t("the words to search for, as on the Search page"))}" value="${esc(preset.query || "")}">`, "search")}
    ${row("", `<div class="caption" id="xf-with"></div>`, "search")}
    ${row(t("Tradition"), `<select id="xf-road"><option value="sunni">${esc(t("Sunni"))}</option><option value="shia">${esc(t("Shia"))}</option><option value="">${esc(t("All"))}</option></select>`, "narrators")}
    ${row(t("Name"), `<input type="text" id="xf-narrator" placeholder="${esc(t("a name to narrow the list (empty: all of them)"))}">`, "narrators")}
    ${row(t("Books"), xfCheckList("xf-net", books, d.books.map((w) => w.key), true), "network")}
    ${row(t("Profiles"), xfCheckList("xf-profiles", d.profiles.map((p) => [p.id, p.name || t("Default"), ""]), [d.active], false), "profile")}
    ${row(t("Books"), `<select id="xf-stats"><option value="|sunni">${esc(t("All Sunni books"))}</option><option value="|shia">${esc(t("All Shia books"))}</option>${d.books.map((w) => `<option value="${esc(w.key)}|${w.tradition}">${esc(`${w.title} (${w.key})`)}</option>`).join("")}</select>`, "statistics")}
    ${row(t("Languages"), xfCheckList("xf-langs", d.languages.map((l) => [l, t(l), ""]), d.languages.filter((l) => !d.chosen.length || d.chosen.includes(l)), false), "document")}
    ${row("", `<label class="src-check"><input type="checkbox" id="xf-chains" checked> ${esc(t("With the chains (Arabic, Turkish, English)"))}</label>`, "document")}
    ${row(t("Format"), `<select id="xf-format"></select>`)}
    </div>
    <textarea id="xf-preview" readonly spellcheck="false" placeholder="${esc(t("Preview: what the file will hold"))}"></textarea>
    <progress id="xf-progress" hidden></progress>
    <div class="caption" id="xf-status"></div>
    <div class="pd-buttons xf-buttons"><button class="quiet small" id="xf-look">${esc(t("Preview"))}</button><span class="grow"></span>
      <button class="quiet small" id="xf-stop" hidden>${esc(t("Stop"))}</button><button class="pd-cancel" id="xf-close">${esc(t("Close"))}</button>
      <button class="primary small" id="xf-go">${esc(t("Export…"))}</button></div></div>`;
  document.body.appendChild(back);
  const q = (id) => back.querySelector("#" + id);
  const status = (m) => { q("xf-status").textContent = m; };
  const clear = () => { q("xf-preview").value = ""; status(""); };
  const kindNow = () => q("xf-kind").value;
  const spec = () => d.kinds.find((k) => k.key === kindNow());
  const document_ = () => ["hadith", "book", "search"].includes(kindNow());
  const searchWords = preset.query || "";
  const searchFilters = preset.search || null;

  const filtersText = async () => {
    const el = q("xf-with");
    if (!searchFilters || q("xf-query").value.trim() !== searchWords.trim()) {
      el.textContent = t("All books and languages, every grade (the words alone).");
      return;
    }
    const m = await invoke("xfer_method", { query: searchWords, search: searchFilters });
    const parts = [t("Books: {books}", { books: Array.isArray(m.books) ? m.books.join(", ") : t("all") })];
    if (Array.isArray(m.languages)) parts.push(t("Languages: {languages}", { languages: m.languages.join(", ") }));
    if (m.grades) parts.push(t("Grades: {grades}", { grades: m.grades }));
    if (m.chapters) parts.push(t("Chapter: {chapter}", { chapter: m.chapters.join("; ") }));
    el.textContent = t("With the choices of the Search page — {choices}", { choices: parts.join(" · ") });
  };

  const fillChapters = async (wanted) => {
    const keys = xfKeys(q("xf-books"));
    const several = keys.length > 1;
    const sel = q("xf-chapter");
    sel.disabled = keys.length !== 1;                       // a chapter of one book only
    q("xf-separate-row").hidden = !several;
    sel.innerHTML = `<option value="">${esc(several ? t("Every chapter of every book chosen") : t("The whole book"))}</option>`;
    clear();
    if (keys.length !== 1) return;
    const rows = await invoke("xfer_chapters", { book: keys[0] });
    if (xfKeys(q("xf-books")).join() !== keys.join()) return;
    sel.innerHTML += rows.map((r) => `<option value="${r.number}">${esc(`${r.number}. ${r.title}`.trim())}</option>`).join("");
    if (wanted != null && rows.some((r) => String(r.number) === String(wanted))) sel.value = String(wanted);
  };

  const kindChanged = () => {
    const kind = kindNow();
    q("xf-help").textContent = t(XF_KIND_HELP[kind]);
    for (const r of back.querySelectorAll("[data-scope]")) {
      r.hidden = r.dataset.scope === "document" ? !document_() : r.dataset.scope !== kind;
    }
    q("xf-format").innerHTML = spec().export.map((f) => `<option value="${f}">${esc(label(f))}</option>`).join("");
    clear();
  };

  const firstChapter = async (book) => {
    const rows = await invoke("xfer_chapters", { book });
    return rows.length ? rows[0].number : null;
  };

  const args = async (preview) => {
    const kind = kindNow(), fmt = q("xf-format").value;
    const a = { kind, fmt, languages: xfKeys(q("xf-langs")), chains: q("xf-chains").checked };
    if (kind === "hadith") a.refs = q("xf-refs").value.split(/\s+/).filter(Boolean);
    else if (kind === "book") {
      const keys = xfKeys(q("xf-books"));
      if (keys.length === 1 || (preview && keys.length)) {
        a.book = keys[0];                                   // the preview shows the first book chosen
        a.chapter = keys.length === 1 && q("xf-chapter").value !== "" ? Number(q("xf-chapter").value) : null;
        if (preview && a.chapter === null && fmt !== "isnady-book") a.chapter = await firstChapter(keys[0]);  // its first chapter, not the whole book
      } else {
        a.books = xfEvery(q("xf-books")) ? ["all"] : keys;
        a.separate = q("xf-separate").checked;
      }
    } else if (kind === "search") {
      a.query = q("xf-query").value.trim();
      if (searchFilters && a.query === searchWords.trim()) a.search = searchFilters;
    } else if (kind === "narrators") {
      a.tradition = q("xf-road").value || null;
      a.query = q("xf-narrator").value.trim();
    } else if (kind === "network") {
      a.books = xfEvery(q("xf-net")) ? [] : xfKeys(q("xf-net"));
      a.tradition = null;
    } else if (kind === "statistics") {
      const [key, road] = q("xf-stats").value.split("|");
      a.book = key;
      a.tradition = road;
    } else if (kind === "profile") {
      a.profiles = xfKeys(q("xf-profiles"));
      if (preview) a.profiles = a.profiles.slice(0, 1);
    }
    return a;
  };

  const suggested = (a) => {
    const kind = a.kind;
    let base;
    if (kind === "book" && a.books && a.books.length) base = a.books[0] === "all" && a.books.length === 1 ? "isnady_books" : "isnady_" + a.books.slice(0, 4).join("_");
    else if (kind === "book") {
      const w = d.books.find((b) => b.key === a.book);
      base = (w ? w.title : a.book || "") + (a.chapter != null ? `_chapter_${a.chapter}` : "");
    } else if (kind === "hadith") base = ("isnady_" + a.refs.join("_")).slice(0, 67).replace(/:/g, "-");
    else if (kind === "search") base = "isnady_search_" + a.query.slice(0, 40);
    else base = `isnady_${kind}`;
    base = [...base].map((c) => (/[\p{L}\p{N}\-_.]/u.test(c) ? c : "_")).join("").replace(/^_+|_+$/g, "") || "isnady";
    return base + d.suffix[a.fmt];
  };

  const preview = async () => {
    document.body.classList.add("busy");
    try {
      const a = await args(true);
      if (a.kind === "hadith" && a.refs.length > 5) a.refs = a.refs.slice(0, 5);
      const r = await invoke("xfer_preview", { args: a });
      const note = r.shown === a.fmt ? "" : t("(shown as {shown}; the file will be {fmt})", { shown: label(r.shown), fmt: label(a.fmt) });
      q("xf-preview").value = (note ? note + "\n\n" : "") + r.text + (r.more ? "\n…" : "");
      status("");
    } catch (e) {
      q("xf-preview").value = "";
      status(String(e));
    } finally {
      document.body.classList.remove("busy");
    }
  };

  const running = (on) => {
    q("xf-progress").hidden = !on;
    q("xf-stop").hidden = !on;
    q("xf-stop").disabled = false;
    q("xf-go").disabled = on;
  };

  const go = async () => {
    const a = await args(false);
    const fmt = a.fmt, kind = a.kind;
    if (kind === "book" && !a.book && !(a.books && a.books.length)) { status(t("Choose at least one book.")); return; }
    if (kind === "profile" && !a.profiles.length) { status(t("Choose at least one profile.")); return; }
    if (document_() && fmt !== "isnady-book" && d.languages.length && !a.languages.length) { status(t("Choose at least one language.")); return; }
    const many = !!((a.books && a.books.length && (a.separate || fmt === "isnady-book")) || (a.profiles && a.profiles.length > 1));
    const sep = d.home.includes("\\") ? "\\" : "/";
    let name = suggested(a);
    if (many) name = name.replace(/\.[^.]*$/, "").split(".isnady-book")[0];
    let path = await askText(many ? t("Export: the folder for the files") : t("Export"), many ? t("Folder") : t("File"), d.home + sep + name);
    if (!path || !path.trim()) return;
    path = path.trim();
    const suffix = d.suffix[fmt];
    if (!many && !path.endsWith(suffix) && !path.endsWith(suffix.slice(suffix.lastIndexOf(".")))) path += suffix;
    try {
      await invoke("xfer_export", { args: a, path });
    } catch (e) { status(t("Failed: ") + String(e)); return; }
    running(true);
    q("xf-progress").removeAttribute("value");
    status(t("Exporting… This window can be closed: the export goes on, its progress is shown at the bottom of the main window."));
    const bar = xfBar();
    const file = path.split(/[\\/]/).pop();
    bar.querySelector('[data-a="stop"]').hidden = false;
    bar.querySelector('[data-a="stop"]').disabled = false;
    bar.querySelector('[data-a="show"]').hidden = true;
    bar.querySelector('[data-a="close"]').hidden = true;
    const say = (m) => { bar.querySelector(".xf-bar-text").textContent = `${t("Export")} · ${file}: ${m}`; };
    say(t("Starting…"));
    let said = "";
    xfWatch((st) => {
      said = st.text || said;
      if (st.total > 0) {
        const text = (said.split(" · ")[0] ? said.split(" · ")[0] + " · " : "") + xfDescribe(st);
        if (back.isConnected) { q("xf-progress").max = st.total; q("xf-progress").value = st.done; status(text); }
        say(text);
      } else if (st.text) {
        if (back.isConnected) status(st.text);
        say(st.text);
      }
    }, (r) => {
      const text = r.ok ? t("Written: {path}", { path: r.message })
        : r.cancelled ? t("Stopped; the unfinished file was removed.") : t("Failed: ") + r.message;
      say(text);
      bar.querySelector('[data-a="stop"]').hidden = true;
      bar.querySelector('[data-a="close"]').hidden = false;
      if (r.ok) { bar.dataset.path = r.message; bar.querySelector('[data-a="show"]').hidden = false; }
      else setTimeout(() => bar.isConnected && bar.remove(), 15000);
      if (back.isConnected) { running(false); status(text); }
    });
  };

  q("xf-kind").value = d.kinds.some((k) => k.key === kind) ? kind : "hadith";
  for (const id of ["xf-books", "xf-net", "xf-profiles", "xf-langs"]) xfWireList(q(id), id === "xf-books" ? () => fillChapters() : clear);
  q("xf-kind").onchange = kindChanged;
  q("xf-format").onchange = clear;
  q("xf-separate").onchange = clear;
  q("xf-query").oninput = filtersText;
  q("xf-look").onclick = preview;
  q("xf-go").onclick = go;
  q("xf-stop").onclick = () => { invoke("xfer_stop"); q("xf-stop").disabled = true; status(t("Stopping…")); };
  q("xf-close").onclick = () => back.remove();
  back.addEventListener("keydown", (e) => { if (e.key === "Escape") back.remove(); });
  kindChanged();
  if (preset.format && spec().export.includes(preset.format)) q("xf-format").value = preset.format;
  await fillChapters(preset.chapter);
  filtersText();
  const focus = { hadith: "xf-refs", search: "xf-query", narrators: "xf-narrator" }[kindNow()];
  if (focus) q(focus).focus();
}

async function importDialog(kind) {
  closeMenus();
  let d;
  try { d = await invoke("xfer_page"); } catch (e) { infoDialog(t("Import"), `<p>${esc(String(e))}</p>`); return; }
  const kinds = d.kinds.filter((k) => k.import.length);
  const back = document.createElement("div");
  back.className = "dialog-back";
  const row = (name, body, id) => `<div class="xf-row"${id ? ` id="${id}"` : ""}><div class="xf-name">${esc(name)}</div><div class="xf-field">${body}</div></div>`;
  back.innerHTML = `<div class="dialog xf xf-in" role="dialog"><div class="pd-title">${esc(t("Import"))}</div>
    <div class="xf-form">
    ${row(t("What"), `<select id="xi-kind">${kinds.map((k) => `<option value="${k.key}">${esc(t(XF_KIND_TEXT[k.key]))}</option>`).join("")}</select>`)}
    ${row("", `<div class="caption" id="xi-help"></div>`)}
    ${row(t("File"), `<input type="text" id="xi-file" spellcheck="false" placeholder="${esc(t("the file to import, with its folder; several: with ; between them"))}">`)}
    ${row("", `<div class="caption" id="xi-detected"></div>`)}
    ${row("", `<label class="src-check"><input type="checkbox" id="xi-replace"> ${esc(t("Replace what is there (otherwise added to it)"))}</label>`, "xi-replace-row")}
    ${row(t("Folder"), `<input type="text" id="xi-folder" placeholder="${esc(t("optional: a folder to put them in, e.g. Imported"))}">`, "xi-folder-row")}
    ${row(t("Name"), `<input type="text" id="xi-name" placeholder="${esc(t("optional: the new profile's name"))}">`, "xi-name-row")}
    </div>
    <div class="caption xf-said" id="xi-status"></div><span class="grow"></span>
    <div class="pd-buttons"><button class="pd-cancel" id="xi-close">${esc(t("Close"))}</button><button class="primary small" id="xi-go">${esc(t("Import"))}</button></div></div>`;
  document.body.appendChild(back);
  const q = (id) => back.querySelector("#" + id);
  const kindNow = () => q("xi-kind").value;
  const paths = () => q("xi-file").value.split(";").map((p) => p.trim()).filter(Boolean);
  const extra = {
    book: "A book comes in as a source of your own: an isnady book (.isnady-book.zip, its folder or book.json) or an export of isnady's as JSON, JSON Lines, CSV or TSV.",
    bookmarks: "Added to your bookmarks; the ones you have stay.",
    settings: "The settings in the file are set; the others stay unless Replace is ticked.",
    profile: "Comes in as a new profile; yours stays as it is.",
    backup: "Your settings, place and bookmarks are replaced by the backup's.",
  };
  let token = 0;
  const detect = async () => {
    const list = paths(), mine = ++token;
    if (!list.length) { q("xi-detected").textContent = ""; q("xi-go").disabled = false; return; }
    const kind = kindNow();
    const got = await invoke("xfer_guess", { kind, files: list });
    if (mine !== token) return;
    const can = kinds.find((k) => k.key === kind).import;
    const wrong = got.filter((g) => !can.includes(g.format)).map((g) => `${g.name} (${g.format})`);
    if (wrong.length) q("xi-detected").textContent = t("Not read for this: {files}; choose other files.", { files: wrong.join(", ") });
    else if (got.length === 1) q("xi-detected").textContent = t("Read as {fmt}.", { fmt: t(got[0].label) });
    else q("xi-detected").textContent = t("{n} files, read as {formats}.", { n: got.length, formats: [...new Set(got.map((g) => t(g.label)))].join(", ") });
    q("xi-go").disabled = wrong.length > 0;
  };
  const kindChanged = () => {
    const kind = kindNow();
    const formats = kinds.find((k) => k.key === kind).import.map((f) => d.labels[f] || f).join(", ");
    q("xi-help").textContent = t("From: {formats}.", { formats }) + " " + t(extra[kind]);
    q("xi-replace-row").hidden = !["bookmarks", "settings"].includes(kind);
    q("xi-folder-row").hidden = kind !== "bookmarks";
    q("xi-name-row").hidden = kind !== "profile";
    detect();
  };
  const go = async () => {
    const kind = kindNow(), files = paths();
    if (!files.length) return;
    if (kind === "backup" && !(await askYesNo(t("Import"), t("Your settings, place and bookmarks will be replaced by the backup's. Go on?")))) return;
    const say = (m) => { q("xi-status").textContent = m; };
    try {
      await invoke("xfer_import", {
        kind, files, replace: q("xi-replace").checked && ["bookmarks", "settings"].includes(kind),
        folder: kind === "bookmarks" ? q("xi-folder").value.trim() : "", name: kind === "profile" ? q("xi-name").value.trim() : "",
      });
    } catch (e) { say(t("Failed: ") + String(e)); return; }
    q("xi-go").disabled = true;
    say(t("Importing…"));
    xfWatch((st) => { if (st.text && back.isConnected) say(st.text); }, async (r) => {
      if (back.isConnected) { q("xi-go").disabled = false; say(r.ok ? r.message : t("Failed: ") + r.message); }
      if (!r.ok) return;
      if (kind === "settings" || kind === "backup") { location.reload(); return; }   // the window drawn again with them
      if (kind === "book") {                                                         // every page brought up to date
        const s = await invoke("start");
        APP.books = s.books;
        APP.hadith = s.hadith;
        await fillSearchFilters();
        catchUp();
      }
      if (CURRENT) show(CURRENT);
      infoDialog(t("Import"), `<p>${esc(r.message).replace(/\n/g, "<br>")}</p>`);
    });
  };
  q("xi-kind").value = kinds.some((k) => k.key === kind) ? kind : "book";
  q("xi-kind").onchange = kindChanged;
  q("xi-file").oninput = detect;
  q("xi-go").onclick = go;
  q("xi-close").onclick = () => back.remove();
  back.addEventListener("keydown", (e) => { if (e.key === "Escape") back.remove(); });
  kindChanged();
  q("xi-file").focus();
}

// "Export these results" on the Search page: the words with the books, languages, grades and chapter chosen there
function exportSearch() {
  const words = $("q").value.trim();
  if (!words) return;
  exportDialog("search", {
    query: words,
    search: {
      mode: SEARCH.mode, whole_words: SEARCH.whole, collections: SEARCH.books || [], languages: SEARCH.languages || [],
      grades: SEARCH.grades, every_grader: SEARCH.every, sahihayn: SEARCH.sahihayn, chapters: SEARCH.chapters.map((c) => c.id),
    },
  });
}

// ⤓ on the Books page: this book, or the chapter being read
async function exportBook() {
  if (!BK.work) { exportDialog("book"); return; }
  const chapter = BK.chapter ? await invoke("xfer_chapter_number", { node: BK.chapter }) : null;
  exportDialog("book", { book: BK.work.key, chapter });
}

// Help → Check for Updates… / Check Installation… (update_dialog, install_dialog): the copies of isnady-rs, the newest on
// crates.io and what is new; Update brings every copy up to date where it is (cargo install), in the background
async function helpDialog(updates) {
  closeMenus();
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.innerHTML = `<div class="dialog xf xf-in" role="dialog"><div class="pd-title">${esc(t(updates ? "Check for Updates" : "Check Installation"))}</div>
    <div class="lead" id="hd-summary">${esc(updates ? t("Asking crates.io for the latest isnady…") : t("Looking for every copy of isnady on this computer…"))}</div>
    <textarea id="xf-preview" readonly spellcheck="false"></textarea>
    <div class="caption">${esc(updates ? t("The same in a terminal: iy-rs update --check, iy-rs update.") : t("The same in a terminal: iy-rs doctor, iy-rs doctor --fix."))}</div>
    <div class="pd-buttons">${updates ? "" : `<button class="quiet small" id="hd-copy">${esc(t("Copy report"))}</button>`}<span class="grow"></span>
      <button class="primary small" id="hd-update" hidden data-tip="${esc(t("Run the update commands shown in the report, one after the other"))}">${esc(t("Update"))}</button>
      <button class="pd-cancel" id="hd-close">${esc(t("Close"))}</button></div></div>`;
  document.body.appendChild(back);
  const q = (id) => back.querySelector("#" + id);
  q("hd-close").onclick = () => back.remove();
  back.addEventListener("keydown", (e) => { if (e.key === "Escape") back.remove(); });
  const r = await invoke("help_check", { updates });
  if (!back.isConnected) return;
  let text = r.text;
  if (updates) {
    if (!r.latest) q("hd-summary").textContent = t("crates.io could not be reached (no internet?). Installed: isnady {v}.", { v: r.version });
    else if (r.behind) q("hd-summary").textContent = t("A newer isnady is out ({where}).", { where: "crates.io " + r.latest });
    else q("hd-summary").textContent = t("isnady {v} is up to date", { v: r.version }) + ".";
    text = (r.news || []).map(([v, body]) => t("What is new in {version}:", { version: v }) + "\n" + body + "\n").join("\n") + (r.news && r.news.length ? "\n" : "") + r.text;
  } else if (r.problems) {
    q("hd-summary").textContent = t("{problems} thing(s) to update; see below. Update brings every copy up to date where it is installed.", { problems: r.problems });
  } else {
    q("hd-summary").textContent = t("One installation, isnady {version}; {commands}", { version: r.version,
      commands: r.commands ? t("every isnady command on PATH starts it.") : t("no isnady command was found on PATH (started from a virtual environment or a shortcut?).") });
  }
  q("xf-preview").value = text;
  q("hd-update").hidden = !r.fixes;
  if (q("hd-copy")) q("hd-copy").onclick = () => copyText(text);
  q("hd-update").onclick = async () => {
    q("hd-update").disabled = true;
    q("hd-summary").textContent = t("Updating every copy of isnady where it is installed…");
    try { await invoke("help_update"); } catch (e) { q("hd-summary").textContent = String(e); return; }
    for (;;) {
      await new Promise((ok) => setTimeout(ok, 700));
      const st = await invoke("help_update_status");
      if (back.isConnected) q("xf-preview").value = st.output || "";
      if (st.running) continue;
      if (!back.isConnected) return;
      q("hd-summary").textContent = st.failed
        ? t("{failed} of {total} update command(s) did not succeed; the output is below.", { failed: st.failed, total: st.total })
        : t("Updated. Close isnady and open it again to use the new version.");
      return;
    }
  };
}
