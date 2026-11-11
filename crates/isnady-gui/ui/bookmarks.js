"use strict";
// Bookmarks (gui/bookmarks_page.py, BM1): what was marked with ☆ — in folders of the user's own (nested, a bookmark in
// as many folders as wanted, dragged onto a folder), with notes, opened with a click. The store is
// core::bookmarks (bookmarks.json in the profile's folder, the same file as the Python isnady's).
//
// Bookmarks                          | All bookmarks                     [Every kind ▾] [Find in the bookmarks]
// [New folder] [Export / import ▾]   | ❝ Hadith  Sahih al-Bukhari 1   Open  Folders  Note  Remove
// ★ All bookmarks (7)                |   the start of its text · 📁 Prayer · ✎ the note
// ☆ Not in a folder (3) · 📁 Prayer  |

const BM = { built: false, folder: "__all__", kind: "", data: null, chosen: new Set(), seq: 0 };
const BM_COLOURS = { hadith: "#2E7D5B", narrator: "#1D63A8", scholar: "#8A5A1E", term: "#7B3FA0", chapter: "#A23B54", tree: "#5B7A1E" };
const BM_ICONS = { hadith: "❝", narrator: "👤", scholar: "🎓", term: "📖", chapter: "📚", tree: "🌳" };

function bookmarksBuild() {
  if (BM.built) return;
  BM.built = true;
  const page = $("page-bookmarks");
  page.className = "page narrators-page bookmarks-page";
  page.innerHTML = `<div class="np">
    <div class="np-left bm-left">
      <div class="np-title">${esc(t("Bookmarks"))}</div>
      <div class="caption">${esc(t("Hadith, narrators, scholars, terms and chapters you marked with ☆ — in folders of your own. Drag a bookmark onto a folder to file it there; one bookmark can be in many folders."))}</div>
      <div class="bm-buttons"><button class="quiet" id="bm-new">${esc(t("New folder"))}</button>
        <button class="quiet" id="bm-more">${esc(t("Export / import"))} ▾</button></div>
      <div class="np-list bm-tree" id="bm-tree"></div>
    </div>
    <div class="bm-right">
      <div class="bm-bar"><div class="bm-heading" id="bm-heading"></div>
        <button class="combo" id="bm-kind"></button>
        <input id="bm-q" type="search" autocomplete="off" spellcheck="false" placeholder="${esc(t("Find in the bookmarks"))}"></div>
      <div class="bm-list" id="bm-list"></div>
      <div class="lead bm-empty" id="bm-empty" hidden></div>
    </div>
  </div>`;
  $("bm-new").onclick = () => bookmarkNewFolder(null);
  $("bm-more").onclick = (e) => popupAt(e, [
    act("Export bookmarks…", () => exportDialog("bookmarks")),
    act("Import bookmarks…", () => importDialog("bookmarks")),
  ]);
  const kinds = () => [["", t("Every kind")], ...Object.keys(BM_ICONS).map((k) => [k, `${BM_ICONS[k]}  ${t(KIND_NAMES[k])}`])];
  const drawKind = () => {
    const [, label] = kinds().find(([v]) => v === BM.kind);
    $("bm-kind").innerHTML = `<span class="lb">${esc(label)}</span><i class="chev"></i>`;
  };
  drawKind();
  $("bm-kind").onclick = () => comboPopup($("bm-kind"), kinds(), BM.kind, (v) => {
    BM.kind = v;
    drawKind();
    bookmarksFill();
  });
  $("bm-q").addEventListener("input", bookmarksFill);
  const tree = $("bm-tree");
  tree.onclick = (e) => {
    const row = e.target.closest("[data-folder]");
    if (!row) return;
    BM.folder = row.dataset.folder;
    bookmarksRefresh();
  };
  tree.addEventListener("contextmenu", (e) => {
    e.preventDefault();
    const row = e.target.closest("[data-folder]");
    bookmarkFolderMenu(e, row ? row.dataset.folder : null);
  });
  tree.addEventListener("dragover", (e) => {
    const row = e.target.closest("[data-folder]");
    if (!row || row.dataset.folder.startsWith("__")) return;
    e.preventDefault();
    for (const r of tree.querySelectorAll(".drop")) r.classList.remove("drop");
    row.classList.add("drop");
  });
  tree.addEventListener("dragleave", () => { for (const r of tree.querySelectorAll(".drop")) r.classList.remove("drop"); });
  tree.addEventListener("drop", async (e) => {
    const row = e.target.closest("[data-folder]");
    for (const r of tree.querySelectorAll(".drop")) r.classList.remove("drop");
    if (!row || row.dataset.folder.startsWith("__")) return;
    e.preventDefault();
    const ids = (e.dataTransfer.getData("text/x-isnady-bookmarks") || "").split(",").filter((x) => x);
    for (const id of ids) await invoke("bookmarks_change", { action: "file", item: id, folder: row.dataset.folder, on: true });
    bookmarksRefresh();
  });
  const list = $("bm-list");
  list.addEventListener("click", (e) => {
    const b = e.target.closest("[data-do]");
    const row = e.target.closest("[data-item]");
    if (!row) return;
    const id = row.dataset.item;
    if (b) {
      if (b.dataset.do === "open") bookmarkOpen(id);
      else if (b.dataset.do === "folders") bookmarkFoldersMenu(e, id);
      else if (b.dataset.do === "note") bookmarkNote(id);
      else if (b.dataset.do === "remove") bookmarkChange("remove", { item: id });
      return;
    }
    // a click chooses; Ctrl adds to the choice (ExtendedSelection)
    if (!(e.ctrlKey || e.metaKey)) BM.chosen.clear();
    if (BM.chosen.has(id) && (e.ctrlKey || e.metaKey)) BM.chosen.delete(id); else BM.chosen.add(id);
    for (const r of list.querySelectorAll("[data-item]")) r.classList.toggle("on", BM.chosen.has(r.dataset.item));
  });
  list.addEventListener("dblclick", (e) => {
    const row = e.target.closest("[data-item]");
    if (row && !e.target.closest("[data-do]")) bookmarkOpen(row.dataset.item);
  });
  list.addEventListener("contextmenu", (e) => {
    const row = e.target.closest("[data-item]");
    if (!row) return;
    e.preventDefault();
    bookmarkItemMenu(e, row.dataset.item);
  });
  list.addEventListener("dragstart", (e) => {
    const row = e.target.closest("[data-item]");
    if (!row) return;
    if (!BM.chosen.has(row.dataset.item)) {
      BM.chosen = new Set([row.dataset.item]);
      for (const r of list.querySelectorAll("[data-item]")) r.classList.toggle("on", BM.chosen.has(r.dataset.item));
    }
    e.dataTransfer.setData("text/x-isnady-bookmarks", [...BM.chosen].join(","));
    e.dataTransfer.effectAllowed = "copy";
  });
}

// the page shown, or a bookmark added or taken out elsewhere: the folders and the bookmarks again
async function bookmarksRefresh() {
  bookmarksBuild();
  const seq = ++BM.seq;
  const d = await invoke("bookmarks_page", { folder: BM.folder });
  if (seq !== BM.seq) return;
  BM.data = d;
  BM.folder = d.folder;
  let html = `<div class="bm-folder${d.folder === "__all__" ? " on" : ""}" data-folder="__all__">★  ${esc(t("All bookmarks"))}  (${d.total})</div>`;
  html += `<div class="bm-folder${d.folder === "__unfiled__" ? " on" : ""}" data-folder="__unfiled__">☆  ${esc(t("Not in a folder"))}  (${d.unfiled})</div>`;
  const add = (folders, depth) => {
    for (const f of folders) {
      html += `<div class="bm-folder${d.folder === f.id ? " on" : ""}" data-folder="${esc(f.id)}" style="padding-inline-start:${6 + depth * 18}px" data-tip="${esc(t("Drop bookmarks here to file them in {folder}. Right click: new folder inside, rename, delete.", { folder: f.path }))}">📁  ${esc(f.name)}  (${f.count})</div>`;
      add(f.children, depth + 1);
    }
  };
  add(d.tree, 1);
  $("bm-tree").innerHTML = html;
  bookmarksFill();
}

function bookmarksFill() {
  const d = BM.data;
  if (!d) return;
  $("bm-heading").textContent = d.folder === "__all__" ? t("All bookmarks") : d.folder === "__unfiled__" ? t("Not in a folder") : d.heading;
  const words = $("bm-q").value.toLowerCase().split(/\s+/).filter((w) => w);
  const shown = d.items.filter((i) => (!BM.kind || i.kind === BM.kind)
    && words.every((w) => `${i.title} ${i.subtitle} ${i.note}`.toLowerCase().includes(w)));
  $("bm-list").innerHTML = shown.map((i) => {
    const colour = BM_COLOURS[i.kind];
    let h = `<div class="bm-top"><span class="bm-pill" style="color:${colour}">${BM_ICONS[i.kind]} ${esc(t(KIND_NAMES[i.kind]))}</span>
      <span class="bm-title">${esc(i.title)}</span>
      <button class="linkbtn" data-do="open" data-tip="${esc(t("Open it where it belongs"))}">${esc(t("Open"))}</button>
      <button class="linkbtn" data-do="folders" data-tip="${esc(t("Put it in folders, or take it out of them"))}">${esc(t("Folders"))} ▾</button>
      <button class="linkbtn" data-do="note" data-tip="${esc(t("Write a note of your own"))}">${esc(t("Note"))}</button>
      <button class="linkbtn" data-do="remove" data-tip="${esc(t("Take it out of the bookmarks"))}">${esc(t("Remove"))}</button></div>`;
    if (i.detail) h += `<div class="caption">${esc(i.detail)}</div>`;
    if (i.subtitle) {
      h += /[؀-ۿ]/.test(i.subtitle.slice(0, 40))
        ? `<div class="caption bm-ar">${arabicSpan(i.subtitle, 0.62)}</div>`
        : `<div class="caption">${esc(i.subtitle)}</div>`;
    }
    if (i.folders.length) h += `<div class="caption">${i.folders.map((f) => `<span style="color:var(--lapis)">📁 ${esc(f.path)}</span>`).join(" &nbsp; ")}</div>`;
    if (i.note) h += `<div class="lead bm-note">✎ ${esc(i.note)}</div>`;
    return `<div class="bm-item${BM.chosen.has(i.id) ? " on" : ""}" data-item="${esc(i.id)}" draggable="true" style="border-inline-start-color:${colour}">${h}</div>`;
  }).join("");
  const empty = $("bm-empty");
  empty.textContent = shown.length ? "" : (d.total
    ? t("No bookmark here matches.")
    : t("Nothing here yet. Mark anything with ☆ — a hadith card, a narrator, a scholar, a term in Learn, a chapter in Books — and it is kept here."));
  empty.hidden = shown.length > 0;
}

async function bookmarkChange(action, args) {
  const made = await invoke("bookmarks_change", { action, ...args });
  await bookmarksRefresh();
  return made;
}

async function bookmarkNewFolder(parent, withItem) {
  const name = await askText(t("New folder"), t("Name of the folder (Prayer, or Prayer/Friday for one inside another):"), "");
  if (!name || !name.trim()) return;
  await bookmarkChange("new_folder", { folder: parent, name, item: withItem || null });
}

async function bookmarkNote(id) {
  const item = BM.data.items.find((i) => i.id === id);
  if (!item) return;
  const text = await askText(t("Note"), item.title, item.note, true);
  if (text === null) return;
  await bookmarkChange("note", { item: id, name: text });
}

function bookmarkFoldersMenu(e, id) {
  const item = BM.data.items.find((i) => i.id === id);
  const inside = new Set(item.folders.map((f) => f.id));
  const entries = [];
  if (!BM.data.folders.length) entries.push(act("No folder yet: make one with New folder", null, "", { disabled: true }));
  const add = (folders, depth) => {
    for (const f of folders) {
      const on = inside.has(f.id);
      entries.push(act(("    ".repeat(depth)) + (on ? "✓ " : "   ") + f.name,
        () => bookmarkChange("file", { item: id, folder: f.id, on: !on }), "", { raw: true }));
      add(f.children, depth + 1);
    }
  };
  add(BM.data.tree, 0);
  entries.push(SEP, act("New folder with it…", () => bookmarkNewFolder(null, id)));
  popupAt(e, entries);
}

function bookmarkItemMenu(e, id) {
  const item = BM.data.items.find((i) => i.id === id);
  if (!item) return;
  popupAt(e, [
    act("Open", () => bookmarkOpen(id)),
    act("Note…", () => bookmarkNote(id)),
    act("File in a folder", null, "", { submenu: () => BM.data.folders.map((f) =>
      act(f.path, () => bookmarkChange("file", { item: id, folder: f.id, on: true }), "", { raw: true })) }),
    act("New folder with it…", () => bookmarkNewFolder(null, id)),
    SEP,
    act("Copy the title", () => copyText(item.title)),
    act("✕  " + t("Remove from bookmarks"), () => bookmarkChange("remove", { item: id }), "", { raw: true }),
  ]);
}

function bookmarkFolderMenu(e, folder) {
  if (!folder || folder.startsWith("__")) {
    popupAt(e, [act("New folder", () => bookmarkNewFolder(null))]);
    return;
  }
  const below = new Set();
  const walk = (folders, inside) => {
    for (const f of folders) {
      if (inside) below.add(f.id);
      walk(f.children, inside || f.id === folder);
    }
  };
  walk(BM.data.tree, false);
  popupAt(e, [
    act("New folder inside", () => bookmarkNewFolder(folder)),
    act("Rename", async () => {
      const f = BM.data.folders.find((x) => x.id === folder);
      const name = await askText(t("Rename"), t("New name:"), f ? f.path.split("/").pop() : "");
      if (name && name.trim()) await bookmarkChange("rename", { folder, name });
    }),
    act("Move into", null, "", { submenu: () => [
      act("(the top)", () => bookmarkChange("move", { folder, name: "" })),
      ...BM.data.folders.filter((f) => f.id !== folder && !below.has(f.id))
        .map((f) => act(f.path, () => bookmarkChange("move", { folder, name: f.id }), "", { raw: true })),
    ] }),
    SEP,
    act("Delete the folder", async () => {
      const f = BM.data.folders.find((x) => x.id === folder);
      if (await askYesNo(t("Delete the folder"), t("Delete the folder {folder} and the folders inside it? The bookmarks in them stay among your bookmarks.", { folder: f ? f.path : "" }))) {
        await bookmarkChange("delete", { folder });
      }
    }),
  ]);
}

// a bookmark opened where it belongs: its hadith in its book, the narrator, the scholar, the term, the chapter, the tree
async function bookmarkOpen(id) {
  const d = await invoke("bookmark_target", { id });
  if (!d) return;
  if (d.kind === "scholar") return openScholar(d.target);
  if (d.kind === "term") return openTerm(d.target);
  if (d.target === null || d.target === undefined) {
    infoDialog(t("Bookmarks"), `<p>${esc(t("{title} is not in the database now (was its book removed?). The bookmark stays.", { title: d.title }))}</p>`);
    return;
  }
  if (d.kind === "tree") return openTree(d.target[0], d.target[1]);
  if (d.kind === "hadith") return openInBookPlace(d.target);
  if (d.kind === "narrator") {
    const pid = d.target;
    return place(["narrator", pid], async () => {
      NARR.id = pid;
      show("narrators");
      await selectNarrator(pid);
    });
  }
  return place(["chapter", d.target], () => openChapterAt(d.target));
}
