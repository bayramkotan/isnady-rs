"use strict";
// Preferences → Data sources, also File → Data Sources… (gui/sources_page.py): the repository chosen, its books by
// kind and road with what of each is imported, one language for every book, your own sources and the data folder.
// Removing, choosing and adding work here; importing comes with the chain reader (RS17), and says so.

const SRC = { kind: "hadith", road: "all", data: null, busy: false, status: "" };
// the kinds of book, as isnady-data's folders are (0.7.3): (kind, sign, name)
const SRC_KINDS = [["quran", "📖", "The Qur'an"], ["hadith", "📚", "Hadith collections"], ["sahifa", "📜", "Written sources"],
  ["sira", "🕌", "Sira"], ["tabaqat", "👥", "Tabaqat"], ["rijal", "⚖", "Rijal"]];
const SRC_ONE = { quran: "Qur'an", hadith: "Hadith collection", sahifa: "Written source", sira: "Sira", tabaqat: "Tabaqat", rijal: "Rijal" };

function pfSources() {
  return `<div id="src-page" class="src-page"><div class="caption">${esc(t("Loading…"))}</div></div>`;
}

async function pfSourcesWire() {
  await srcLoad();
}

async function srcLoad() {
  try {
    SRC.data = await invoke("src_page");
  } catch (e) {
    const page = $("src-page");
    if (page) page.innerHTML = `<div class="caption">${esc(String(e))}</div>`;
    return;
  }
  srcDraw();
}

const srcN = (n) => Number(n || 0);         // the numbers are grouped by t() ({n:,})

function srcWhy(u) {
  if (u.why === "new") return t("New in the repository: {title}", { title: u.name });
  if (u.why === "changed") return t("Changed in the repository: {title}", { title: u.name });
  return t("{title}: new language(s) {names}", { title: u.name, names: u.new.map((x) => t(x)).join(", ") });
}

function srcStateText(e) {
  const st = e.status;
  if (e.kind === "quran") return st.imported ? t("Downloaded: the Arabic and {n} translation(s)", { n: st.count }) : t("Not downloaded");
  if (!st.imported) return t("Not imported");
  const when = st.when ? ` · ${st.when.slice(0, 10)}` : "";
  if (["sira", "tabaqat", "sahifa"].includes(e.kind)) {
    return (e.kind === "sahifa" ? t("Imported: {n:,} hadith", { n: srcN(st.count) }) : t("Imported: {n:,} entries and sections", { n: srcN(st.count) })) + when;
  }
  const what = e.kind === "rijal" ? t("Imported: {n:,} narrators", { n: srcN(st.count) })
    : e.book ? t("Imported: {n:,} hadith", { n: srcN(st.count) }) : t("Imported: {n:,} texts", { n: srcN(st.count) });
  const langs = st.languages.length ? ` · ${st.languages.map((x) => t(x)).join(", ")}` : "";
  const tier = st.tier ? " · " + t("tier {tier}", { tier: st.tier }) : "";
  return what + langs + when + tier;
}

function srcButton(text, action, opts) {
  const o = opts || {};
  const allowed = o.allowed !== false && !SRC.busy;
  return `<button class="${o.primary ? "primary" : "quiet"}" data-src="${action}"${o.args ? ` data-args="${esc(JSON.stringify(o.args))}"` : ""}${allowed ? "" : " disabled"}${o.tip ? ` data-tip="${esc(o.tip)}"` : ""}>${esc(text)}</button>`;
}

function srcRepoCard(d) {
  const r = d.repo;
  const how = r.how_error ? t("The catalog cannot be read: {exc}", { exc: r.how_error }) : t({
    network: "Catalog read from the repository just now.",
    "shipped-own": "This dataset publishes no catalog of its own; isnady ships one for it.",
    saved: "Catalog read from the repository in the last day (Refresh reads it again).",
    shipped: "The repository cannot be reached now, so the catalog shipped with isnady is used; each source is read from the repository, or from its origin when the repository has no copy.",
  }[r.how]);
  const links = r.homepage ? [`<a href="#" data-link="${esc(r.homepage)}">${esc(r.homepage)}</a>`] : [];
  const options = d.repos.map((x) => `<option value="${esc(x.id)}"${x.id === r.id ? " selected" : ""}>${esc(x.name + (x.builtin ? "" : "  " + t("(added by you)")))}</option>`).join("");
  return `<div class="ncard pf-card"><div class="src-head"><div class="ncard-title">${esc(t("Repository"))}</div>
      <select id="src-repo"${SRC.busy ? " disabled" : ""}>${options}</select></div>
    <div class="lead">${esc(t(r.description))}</div>
    <div class="caption">${links.concat([esc(how)]).join(" · ")}</div>
    <div class="src-bar">${srcButton(t("Refresh"), "refresh")}${srcButton(t("A folder on this computer…"), "folder-repo")}${srcButton(t("Add a repository…"), "add-repo")}${srcButton(t("Remove from list"), "remove-repo", { allowed: !r.builtin })}${srcButton(t("Use the default"), "default-repo", { allowed: !r.default })}</div></div>`;
}

function srcLanguagesCard(d) {
  const rows = d.languages.map((r) => `<b>${esc(t(r.label))}</b><span class="caption">${esc(t("in {have} of the {offered} books that have it", { have: r.imported, offered: r.offered }))}</span>
    ${srcButton(t("Add to every book"), "import", { allowed: r.imported < r.offered })}
    ${srcButton(t("Remove from every book"), "remove-language", { allowed: r.code !== "ara" && r.imported > 0, args: { code: r.code, label: r.label }, tip: r.code === "ara" ? t("Arabic stays: the chains and the narrators are read from it.") : "" })}`).join("");
  return `<div class="ncard pf-card"><div class="ncard-title">${esc(t("Languages for every book"))}</div>
    <div class="caption">${esc(t("Add a language to every imported book that has it, or take it out of all of them, in one step. Arabic stays: the chains are read from it. Each book's card below can still differ."))}</div>
    <div class="src-grid">${rows}</div></div>`;
}

function srcLanguageRow(e) {
  let html = `<span class="caption">${esc(t("Languages:"))}</span>`;
  if (e.kind === "quran") {
    const have = new Set(e.quran_have);
    html += `<label class="src-check" data-tip="${esc(t("The Arabic text is always there"))}"><input type="checkbox" checked disabled> ${esc(t("Arabic"))}</label>`;
    e.languages.forEach((lang, i) => {
      const on = have.has(lang) || (!have.size && e.quran_wanted.includes(lang));
      html += `<label class="src-check" data-tip="${esc(t("Ticked: one translation in this language ({name}); untick to remove its translations. Translators… chooses others.", { name: e.quran_names[i] || "" }))}"><input type="checkbox" data-code="${esc(lang)}"${on ? " checked" : ""}> ${esc(t(lang))}</label>`;
    });
    html += srcButton("➕ " + t("Translators…"), "import", { tip: t("Choose the translations to download, of 98 languages; untick one to remove it") });
  } else if (e.book && e.languages.length && e.kind === "hadith") {
    const chosen = e.status.languages.length ? e.status.languages : ["Arabic", "Turkish", "English"];
    e.languages.forEach((code, i) => {
      const label = e.language_labels[i];
      html += `<label class="src-check"${code === "ara" ? ` data-tip="${esc(t("Arabic stays: the chains and the narrators are read from it."))}"` : ""}><input type="checkbox" data-code="${esc(code)}"${chosen.includes(label) ? " checked" : ""}> ${esc(t(label))}</label>`;
    });
  } else {
    html += `<label class="src-check" data-tip="${esc(t("This work is in Arabic only, as its author wrote it"))}"><input type="checkbox" checked disabled> ${esc(t("Arabic"))}</label>`;
  }
  return `<div class="src-langs">${html}</div>`;
}

function srcCard(e) {
  const st = e.status;
  let tags = "";
  if (e.builtin) {
    const items = [];
    if (e.update) items.push(["⟳ " + t({ new: "New", changed: "Changed", languages: "New language" }[e.update.why]), "tag-gold", srcWhy(e.update)]);
    items.push([t(SRC_ONE[e.kind] || e.kind), "tag-lapis", ""]);
    items.push([e.road === null ? t("Both roads") : e.road === "shia" ? t("Shia") : t("Sunni"), e.road === "shia" ? "tag-gold" : "tag-muted",
      t("The tradition of the book: Shia books are kept apart from Sunni ones, and their grades and narrators are read in their own terms.")]);
    items.push([e.license || t("licence not stated"), "tag-muted", e.shareable ? t("May be shared further (tier A/B)") : t("No licence allowing it to be shared further: for this computer (tier C)")]);
    if (e.compiler && APP.ui_language === "en") items.push([e.compiler, "tag-muted", ""]);
    if (e.size && e.size >= 2 ** 19) {
      items.push([t("about {mb} MB", { mb: Math.max(1, Math.round(e.size / 2 ** 20)) }), "tag-muted",
        e.kind === "quran" ? t("Downloaded once; the Qur'an's translations each when chosen") : t("Downloaded once")]);
    }
    if (!e.ours && e.origin_url) items.push([t("read from its publisher"), "tag-muted", `${e.origin_name}\n${e.origin_url}`]);
    tags = items.map(([text, cls, tip]) => `<span class="${cls}"${tip ? ` data-tip="${esc(tip)}"` : ""}>${esc(text)}</span>`).join("");
    if (e.hadith) tags += `<span class="caption">${esc(t("{n:,} hadith", { n: srcN(e.hadith) }))}</span>`;
    if (e.origin_url) tags += `<a class="caption" href="#" data-link="${esc(e.origin_url)}">${esc(e.origin_name || e.origin_url)}</a>`;
    tags = `<div class="src-tags">${tags}</div>`;
  }
  const args = { id: e.id, repo: e.builtin ? e.repo : "", title: e.title };
  const buttons = [
    srcButton(st.imported ? t("Update") : t("Import"), "import", { primary: !st.imported }),
    srcButton(t("Remove data"), "remove", { allowed: st.imported, args, tip: st.imported ? "" : t("Nothing of it is imported yet") }),
    srcButton("📚 " + t("Open in Books"), "open", { allowed: !!e.work_key, args: { key: e.work_key }, tip: e.work_key ? "" : t("Import it first; then it is read in Books") }),
    srcButton("⤴ " + t("Export…"), "export", { allowed: st.imported && e.kind === "hadith" && !!e.book,
      tip: !st.imported ? t("Import it first") : t("Export writes hadith collections (EPUB, PDF, CSV …); this kind is read in Books") }),
  ];
  if (!e.builtin) buttons.push(srcButton(t("Delete from list"), "delete", { args }));
  return `<div class="ncard pf-card src-card"><div class="src-chead"><div class="ncard-title src-name" data-tip="${esc(e.title)}">${esc(e.name)}</div>
      ${e.title_ar ? `<div class="ar src-ar">${esc(e.title_ar)}</div>` : ""}<div class="caption src-state">${esc(srcStateText(e))}</div></div>
    ${tags}<div class="caption">${esc(e.about + (e.builtin ? "" : `  ·  ${e.location}`))}</div>
    ${srcLanguageRow(e)}<div class="src-buttons">${buttons.join("")}</div></div>`;
}

function srcFolderCard(d) {
  const f = d.folder;
  const how = { default: t("the default for this system"), chosen: t("chosen by you"), portable: t("chosen by you"),
    environment: t("set by the ISNADY_DATA_DIR environment variable") }[f.source];
  const env = f.source === "environment";
  return `<div class="ncard pf-card"><div class="src-head"><div class="ncard-title">${esc(t("Data folder"))}</div><span class="caption">${esc(how)}</span></div>
    <div class="lead">${esc(f.path)}</div>
    <div class="caption">${esc(t("The database, your settings and your own sources live here. A new folder is used from the next start of isnady. A ready database (isnady.db, or the isnady-db zip of isnady-data) can be taken from a file on this computer instead of importing book by book."))}</div>
    <div class="src-bar"><button class="quiet" data-src="open-folder">${esc(t("Open"))}</button>${srcButton(t("Change…"), "change-folder", { allowed: !env })}${srcButton(t("Use default"), "default-folder", { allowed: !env && f.source !== "default" })}${srcButton(t("Use a ready database…"), "ready")}</div></div>`;
}

function srcDraw() {
  const page = $("src-page");
  const d = SRC.data;
  if (!page || !d) return;
  const entries = d.entries;
  let html = `<div class="lead">${esc(t("isnady reads every book from a REPOSITORY: its own by default (isnady-data). Its books stand in one folder per kind — the Qur'an, hadith, written sources, sira, tabaqat, rijal — and in each the Sunni road and the Shia road have their own. This page shows them the same way: choose a kind, then a road. Every book has the same buttons and its languages. The same from the command line: iy repo, iy catalog."))}</div>`;
  html += srcRepoCard(d);
  if (SRC.busy) html += `<div class="src-progress"><i></i></div>`;
  const status = SRC.status || (d.entries_error ? t("The repository cannot be read: {exc}", { exc: d.entries_error }) : "");
  html += `<div class="caption" id="src-status">${esc(status)}</div>`;
  const n = d.updates.length;
  html += `<div class="src-head"><div class="ncard-title src-big">${esc(t("Sources of {name}", { name: d.repo.name }))}</div><span class="src-fill"></span>
    ${srcButton("⟳ " + (n ? t("Get updates ({n})", { n }) : t("Up to date")), "import", { primary: !!n, allowed: !!n,
      tip: n ? d.updates.map(srcWhy).join("\n") : t("Nothing new in the repository: no new book, no changed book, no new language of the languages you read") })}
    ${srcButton(t("Import all"), "import", { primary: !n, allowed: entries.length > 0, tip: t("Every source of this repository, each in the languages ticked on its card; the search index, chains and narrators are brought up to date once, at the end") })}</div>`;
  const kinds = SRC_KINDS.filter((k) => entries.some((e) => e.kind === k[0]));
  if (kinds.length && !kinds.some((k) => k[0] === SRC.kind)) SRC.kind = kinds[0][0];
  if (kinds.length) {
    html += `<div class="src-bar">${kinds.map(([kind, sign, name]) => `<button class="seg${kind === SRC.kind ? " on" : ""}" data-kind="${kind}" data-tip="${esc(t("In the repository: {folder}", { folder: `isnady-data/${kind}/` }))}">${esc(`${sign}  ${t(name)}  ·  ${entries.filter((e) => e.kind === kind).length}`)}</button>`).join("")}</div>`;
  }
  const shown = entries.filter((e) => e.kind === SRC.kind);
  const roads = ["sunni", "shia"].filter((r) => shown.some((e) => e.road === r));
  if (roads.length > 1) {
    html += `<div class="src-bar">${[["all", "Both roads"], ["sunni", "Sunni"], ["shia", "Shia"]].map(([k, x]) => `<button class="seg${SRC.road === k ? " on" : ""}" data-road="${k}">${esc(t(x))}</button>`).join("")}</div>`;
  } else if (!["all", ...roads].includes(SRC.road)) {
    SRC.road = "all";
  }
  const sign = (SRC_KINDS.find((k) => k[0] === SRC.kind) || [0, "•", SRC.kind])[1];
  const kindName = (SRC_KINDS.find((k) => k[0] === SRC.kind) || [0, 0, SRC.kind])[2];
  for (const road of (roads.length ? roads : [null])) {
    const group = shown.filter((e) => road === null || e.road === road);
    if (!group.length || (road !== null && !["all", road].includes(SRC.road))) continue;
    const folder = road ? `${SRC.kind}/${road}/` : `${SRC.kind}/`;
    const color = road ? ` style="color:var(--${road === "shia" ? "gold" : "lapis"});padding-top:8px"` : "";
    html += `<div class="ncard-title src-band"${color}>${esc(`${sign}  ${t(kindName)}` + (road ? ` · ${road === "sunni" ? t("Sunni") : t("Shia")}` : "") + `  ·  ${group.length}`)}</div>`;
    html += `<div class="caption">${esc(t("In the repository: {folder}", { folder: `isnady-data/${folder}` }))}</div>`;
    html += group.map(srcCard).join("");
  }
  if (SRC.kind === "hadith" && shown.length) html += srcLanguagesCard(d);
  html += `<div class="ncard-title src-big">${esc(t("Your sources"))}</div>`;
  html += d.mine.map(srcCard).join("") || `<div class="caption">${esc(t("None yet: a file or a URL of your own, in any format isnady reads."))}</div>`;
  html += `<div class="src-bar">${srcButton(t("Add a source…"), "add")}</div>`;
  html += srcFolderCard(d);
  page.innerHTML = html;
  const sel = $("src-repo");
  if (sel) sel.onchange = () => srcAct("use-repo", { id: sel.value });
  page.onclick = (ev) => {
    const link = ev.target.closest("[data-link]");
    if (link) { ev.preventDefault(); invoke("open_url", { url: link.dataset.link }); return; }
    const k = ev.target.closest("[data-kind]");
    if (k) { SRC.kind = k.dataset.kind; SRC.status = ""; srcDraw(); return; }
    const r = ev.target.closest("[data-road]");
    if (r) { SRC.road = r.dataset.road; srcDraw(); return; }
    const b = ev.target.closest("[data-src]");
    if (b && !b.disabled) srcAct(b.dataset.src, b.dataset.args ? JSON.parse(b.dataset.args) : {});
  };
}

function srcSay(text) {
  SRC.status = text;
  const s = $("src-status");
  if (s) s.textContent = text;
}

async function srcRun(first, job) {
  SRC.busy = true;
  SRC.status = first;
  srcDraw();
  let message;
  try {
    message = await job();
  } catch (e) {
    message = t("Failed: ") + String(e);
  }
  SRC.busy = false;
  await srcLoad();
  srcSay(message || "");
  PF.booksChanged = true;
}

// a question with three answers (copy, as it is, cancel)
function srcThree(title, text, a, b) {
  return new Promise((resolve) => {
    const back = document.createElement("div");
    back.className = "dialog-back";
    back.style.zIndex = 70;
    back.innerHTML = `<div class="dialog ask" role="dialog"><div class="pd-title">${esc(title)}</div><div>${esc(text)}</div>
      <div class="pd-buttons"><button class="primary small" data-a="copy">${esc(a)}</button><button class="quiet" data-a="as-is">${esc(b)}</button><button class="pd-cancel" data-a="">${esc(t("Cancel"))}</button></div></div>`;
    back.onclick = (e) => {
      const x = e.target.closest("[data-a]");
      if (!x) return;
      back.remove();
      resolve(x.dataset.a);
    };
    document.body.appendChild(back);
  });
}

async function srcRelocate(target, mode) {
  let made;
  try {
    made = await invoke("src_relocate", { target, mode });
  } catch (e) {
    infoDialog(t("Data folder"), `<p>${esc(String(e))}</p>`);
    return;
  }
  const text = (mode === "copy" ? t("isnady will use\n{new}\nfrom its next start; your data was copied there and the old folder is kept.", { new: made })
    : t("isnady will use\n{new}\nfrom its next start.", { new: made })) + "\n\n" + t("Quit isnady now?");
  const quit = await askYesNo(t("Data folder"), text);
  await srcLoad();
  if (quit) invoke("quit");
}

function srcAddDialog(formats) {
  return new Promise((resolve) => {
    const back = document.createElement("div");
    back.className = "dialog-back";
    back.style.zIndex = 70;
    const row = (label, field) => `<label class="src-frow"><span>${esc(t(label))}</span>${field}</label>`;
    back.innerHTML = `<div class="dialog ask src-add" role="dialog"><div class="pd-title">${esc(t("Add a source"))}</div>
      ${row("Title", `<input type="text" data-f="title">`)}
      ${row("Format", `<select data-f="format_id">${formats.map(([id, title]) => `<option value="${esc(id)}">${esc(title)}</option>`).join("")}</select>`)}
      ${row("File or URL", `<input type="text" data-f="location" placeholder="${esc(t("https://… or a file on this computer"))}">`)}
      ${row("Book", `<input type="text" data-f="book" placeholder="${esc(t("optional, for index files: bukhari, muslim …"))}">`)}
      ${row("Licence", `<input type="text" data-f="license" placeholder="${esc(t("e.g. CC-BY-4.0; empty = not redistributed (tier C)"))}">`)}
      ${row("Authentication", `<select data-f="auth_kind">${[["none", "None"], ["basic", "Username and password"], ["bearer", "Bearer token"], ["apikey", "API key"]].map(([k, x]) => `<option value="${k}">${esc(t(x))}</option>`).join("")}</select>`)}
      ${row("API key header", `<input type="text" data-f="key_name" placeholder="${esc(t("header name for an API key, e.g. X-API-Key"))}">`)}
      <div class="caption">${esc(t("Passwords, tokens and keys are asked for when you import and are never stored."))}</div>
      <div class="pd-buttons"><button class="primary small ok">${esc(t("OK"))}</button><button class="pd-cancel">${esc(t("Cancel"))}</button></div></div>`;
    document.body.appendChild(back);
    const done = (v) => { back.remove(); resolve(v); };
    back.querySelector(".ok").onclick = () => {
      const v = {};
      back.querySelectorAll("[data-f]").forEach((x) => { v[x.dataset.f] = x.value.trim() || null; });
      v.title = v.title || "";
      v.location = v.location || "";
      done(v);
    };
    back.querySelector(".pd-cancel").onclick = () => done(null);
    back.querySelector('[data-f="title"]').focus();
  });
}

async function srcAct(action, a) {
  const d = SRC.data;
  switch (action) {
    case "use-repo":
      try { await invoke("src_use_repo", { id: a.id }); } catch (e) { infoDialog(t("Repository"), `<p>${esc(String(e))}</p>`); }
      SRC.status = "";
      return srcLoad();
    case "default-repo":
      await invoke("src_use_repo", { id: d.default_id });
      SRC.status = "";
      return srcLoad();
    case "refresh":
      try {
        const where = await invoke("src_refresh_repo");
        await srcLoad();
        srcSay(where === "network" ? t("Catalog read again.") : t("The repository cannot be reached; the saved or shipped catalog is used."));
      } catch (e) {
        srcSay(t("Failed: {exc}", { exc: String(e) }));
      }
      return;
    case "add-repo": {
      const text = await askText(t("Add a repository"), t("The address of a repository with an isnady catalog (catalog.json):\n  a GitHub repository   https://github.com/owner/name\n  a Hugging Face dataset   https://huggingface.co/datasets/owner/name\n  a catalog URL, or a folder on this computer"), "");
      if (!text || !text.trim()) return;
      try {
        const name = await invoke("src_add_repo", { text });
        await srcLoad();
        srcSay(t("Added {name}; it is the repository now.", { name }));
      } catch (e) {
        infoDialog(t("Add a repository"), `<p style="white-space:pre-wrap">${esc(String(e))}</p>`);
      }
      return;
    }
    case "folder-repo": {
      const folder = await askText(t("A folder with a catalog.json (e.g. your copy of isnady-data)"), t("Folder"), "");
      if (!folder || !folder.trim()) return;
      try {
        const name = await invoke("src_add_repo", { text: folder });
        await srcLoad();
        srcSay(t("Added {name} ({folder}); it is the repository now, read from this computer.", { name, folder }));
      } catch (e) {
        infoDialog(t("A folder on this computer"), `<p style="white-space:pre-wrap">${esc(String(e))}</p>`);
      }
      return;
    }
    case "remove-repo":
      if (await askYesNo(t("Repository"), t("Remove {name} from the list? What you imported from it stays until you remove it.", { name: d.repo.name }))) {
        await invoke("src_remove_repo");
        SRC.status = "";
        await srcLoad();
      }
      return;
    case "import":
      try { await invoke("src_import"); } catch (e) { srcSay(t("Failed: ") + t(String(e))); }
      return;
    case "remove":
      if (await askYesNo(t("Remove data"), t("Remove everything imported from {title}?", { title: a.title }))) {
        await srcRun(t("Removing {title}…", { title: a.title }), async () => {
          const title = await invoke("src_remove", { id: a.id, repo: a.repo });
          return t("Removed {title}.", { title });
        });
      }
      return;
    case "remove-language": {
      const label = t(a.label);
      if (await askYesNo(t("Remove a language"), t("Take {language} out of every book?", { language: label }))) {
        await srcRun(t("Removing {language} from every book…", { language: label }), async () => {
          const n = await invoke("src_remove_language", { code: a.code });
          return t("{names} removed from every book ({n} edition(s)).", { names: label, n });
        });
      }
      return;
    }
    case "delete":
      await invoke("src_delete", { id: a.id });
      return srcLoad();
    case "add": {
      const v = await srcAddDialog(d.formats);
      if (!v) return;
      try {
        await invoke("src_add", v);
      } catch (e) {
        infoDialog(t("Add a source"), `<p>${esc(String(e))}</p>`);
        return;
      }
      return srcLoad();
    }
    case "open":
      pfClose();
      return openBookAt(a.key);
    case "export":
      return later(["&File", "Export…"])();
    case "open-folder":
      return invoke("open_data_folder");
    case "change-folder": {
      const folder = await askText(t("Choose a folder for isnady's data"), t("Folder"), "");
      if (!folder || !folder.trim()) return;
      const mode = await srcThree(t("Data folder"), t("Use {folder} for isnady's data?", { folder }), t("Copy my data there"), t("Use the folder as it is"));
      if (mode) await srcRelocate(folder.trim(), mode);
      return;
    }
    case "default-folder": {
      const mode = await srcThree(t("Data folder"), t("Go back to the default folder for this system?"), t("Copy my data there"), t("Use it as it is"));
      if (mode) await srcRelocate(null, mode);
      return;
    }
    case "ready": {
      const path = await askText(t("A ready isnady database"), t("isnady database") + " (isnady-db-*.zip isnady*.db *.zip *.db)", "");
      if (!path || !path.trim()) return;
      let info;
      try {
        info = await invoke("src_stage", { path: path.trim() });
      } catch (e) {
        infoDialog(t("Ready database"), `<p>${esc(String(e))}</p>`);
        return;
      }
      const quit = await askYesNo(t("Ready database"), t("{books} books, {hadith:,} hadith. It takes the place of your database at the next start of isnady; your database is kept beside it as isnady.<date>.bak.db.\n\nQuit isnady now?", { books: info.books, hadith: srcN(info.hadith) }));
      srcSay(t("A ready database is waiting; it is used from the next start."));
      if (quit) invoke("quit");
      return;
    }
    default:
  }
}
