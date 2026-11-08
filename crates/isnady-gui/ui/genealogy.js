"use strict";
// Genealogies (gui/genealogy_page.py, GEN1): the prophets' lineage, Quraysh (the Prophet's family ﷺ, the caliphs, the
// Imams), a narrator's family read from the narrators' names, and a narrator's line of learning (teachers above,
// students below) — drawn as trees one can zoom, move and click.
//
// Genealogies  [The prophets] [Quraysh …] [A narrator's family] [A narrator's line of learning]
// [find …                               ] [★ Famous Companions] [−] [+] [Whole tree]
// the board: the tree, dragged and zoomed       | the box chosen: names, kind, his line, his life, the hadith
// the legend and the sources

const GT = { built: false, key: "prophets", person: null, tree: null, pos: {}, focus: null, chosen: null,
  scale: 1, tx: 0, ty: 0, seq: 0, drag: null };
const GT_W = 178, GT_H = 50, GT_GX = 18, GT_GY = 42;
// kind → (fill, border) in the light theme; the dark theme darkens the fills
const GT_KIND = { prophet: ["#F6E7BF", "#A47E24"], caliph: ["#DCE7F5", "#1D4777"], imam: ["#DDEFE3", "#2E7D5B"],
  narrator: ["#FFFFFF", "#8A97A8"], person: ["#F3F5F8", "#8A97A8"] };
const GT_KIND_NAMES = { prophet: "prophet", caliph: "caliph", imam: "Imam", narrator: "narrator", person: "person" };
const GT_VIEWS = [["prophets", "The prophets"], ["quraysh", "Quraysh and the Prophet's family"],
  ["family", "A narrator's family"], ["learning", "A narrator's line of learning"]];
const GT_WRITTEN = ["prophets", "quraysh"];

function genealogyBuild() {
  if (GT.built) return;
  GT.built = true;
  const page = $("page-genealogy");
  page.className = "page genealogy-page";
  page.innerHTML = `<div class="gt">
    <div class="gt-top"><div class="gt-title">${esc(t("Genealogies"))}</div>
      ${GT_VIEWS.map(([k, l]) => `<button class="seg" data-view="${k}">${esc(t(l))}</button>`).join("")}</div>
    <div class="gt-find">
      <input id="gt-q" type="search" autocomplete="off" spellcheck="false">
      <button class="quiet" id="gt-famous" data-tip="${esc(t("The four Caliphs, the ten promised Paradise, the Mothers of the Believers, the Prophet's household, those who narrated most …"))}">★ ${esc(t("Famous Companions"))}</button>
      <button class="quiet" id="gt-smaller" data-tip="${esc(t("Smaller"))}">−</button>
      <button class="quiet" id="gt-larger" data-tip="${esc(t("Larger"))}">+</button>
      <button class="quiet" id="gt-fit" data-tip="${esc(t("Fit the whole tree in the window"))}">${esc(t("Whole tree"))}</button>
    </div>
    <div class="gt-results" id="gt-results" hidden></div>
    <div class="gt-middle">
      <div class="gt-board" id="gt-board"><svg id="gt-svg"><g id="gt-g"></g></svg></div>
      <div class="gt-side" id="gt-side"></div>
    </div>
    <div class="caption gt-caption" id="gt-caption"></div>
  </div>`;
  page.querySelector(".gt-top").onclick = (e) => {
    const b = e.target.closest("[data-view]");
    if (b) showTreeView(b.dataset.view);
  };
  $("gt-q").addEventListener("keydown", (e) => { if (e.key === "Enter") treeFind(); });
  $("gt-q").addEventListener("input", treeFindLive);
  $("gt-famous").onclick = treeFamous;
  $("gt-smaller").onclick = () => treeZoom(1 / 1.25);
  $("gt-larger").onclick = () => treeZoom(1.25);
  $("gt-fit").onclick = treeFit;
  $("gt-results").onclick = (e) => {
    const r = e.target.closest("[data-person]");
    if (!r) return;
    $("gt-results").hidden = true;
    showTreeView(GT.key, Number(r.dataset.person));
  };
  const board = $("gt-board");
  board.addEventListener("mousedown", (e) => {
    if (e.button !== 0) return;
    GT.drag = { x: e.clientX, y: e.clientY, tx: GT.tx, ty: GT.ty, moved: false };
    board.classList.add("dragging");
  });
  window.addEventListener("mousemove", (e) => {
    if (!GT.drag) return;
    const dx = e.clientX - GT.drag.x, dy = e.clientY - GT.drag.y;
    if (Math.abs(dx) + Math.abs(dy) > 3) GT.drag.moved = true;
    GT.tx = GT.drag.tx + dx;
    GT.ty = GT.drag.ty + dy;
    treeTransform();
  });
  window.addEventListener("mouseup", (e) => {
    if (!GT.drag) return;
    const moved = GT.drag.moved;
    GT.drag = null;
    board.classList.remove("dragging");
    if (moved) return;
    const box = e.target.closest && e.target.closest("[data-node]");
    if (box && board.contains(box)) treeClicked(box.dataset.node);
  });
  board.addEventListener("wheel", (e) => {
    e.preventDefault();
    const r = board.getBoundingClientRect();
    treeZoom(e.deltaY < 0 ? 1.15 : 1 / 1.15, e.clientX - r.left, e.clientY - r.top);
  }, { passive: false });
  $("gt-side").addEventListener("click", genealogySideClick);
}

// the page shown (the section chosen): the view it had, or the prophets
async function genealogyRefresh() {
  genealogyBuild();
  if (!GT.tree && !GT.none) await showTreeView(GT.key);
}

// ------------------------------------------------------------------ the layout (a tidy tree, as _layout)

function treeLayout(tree, start, direction) {
  const pos = {};
  let cursor = 0;
  const place = (id, depth) => {
    const node = tree.nodes[id];
    let x;
    if (!node.children.length) {
      x = cursor;
      cursor += GT_W + GT_GX;
    } else {
      const xs = node.children.map((c) => place(c, depth + 1));
      x = (xs[0] + xs[xs.length - 1]) / 2;
    }
    pos[id] = [x, direction * depth * (GT_H + GT_GY)];
    return x;
  };
  place(start, 0);
  return pos;
}

function darker(hex, factor) {
  const n = parseInt(hex.slice(1), 16);
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v) => Math.round(v / factor));
  return `rgb(${c.join(",")})`;
}

const GT_MEASURE = document.createElement("canvas").getContext("2d");
function elide(text, font, width) {
  GT_MEASURE.font = font;
  if (GT_MEASURE.measureText(text).width <= width) return text;
  let lo = 0, hi = text.length;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (GT_MEASURE.measureText(text.slice(0, mid) + "…").width <= width) lo = mid; else hi = mid - 1;
  }
  return text.slice(0, lo) + "…";
}

function drawTree(focus) {
  const tree = GT.tree;
  const g = $("gt-g");
  if (!tree) {
    g.innerHTML = "";
    return;
  }
  const pos = treeLayout(tree, tree.root, 1);
  if (tree.up && tree.nodes[tree.up].children.length) {
    const up = treeLayout(tree, tree.up, -1);
    const shift = pos[tree.root][0] - up[tree.up][0];
    for (const [k, p] of Object.entries(up)) if (k !== tree.up) pos[k] = [p[0] + shift, p[1]];
  }
  GT.pos = pos;
  const css = getComputedStyle(document.documentElement);
  const v = (name) => css.getPropertyValue(name).trim();
  const scale = Number(v("--scale")) || 1;
  const heading = `700 ${(10.5 * scale).toFixed(2)}pt ${v("--heading") || "serif"}`;
  const ar = scriptOf("arabic");
  const arPt = ar.size * 0.62 * scale;
  const arFont = `${arPt.toFixed(2)}pt ${ar.family}`;
  let lines = "", boxes = "";
  for (const [id, p] of Object.entries(pos)) {
    const node = tree.nodes[id];
    const parent = node.parent !== tree.up ? node.parent : tree.root;
    if (!parent || !pos[parent]) continue;
    const a = pos[parent], b = p, upward = b[1] < a[1];
    const ax = a[0] + GT_W / 2, ay = a[1] + (upward ? 0 : GT_H);
    const bx = b[0] + GT_W / 2, by = b[1] + (upward ? GT_H : 0);
    const mid = (ay + by) / 2;
    lines += `<path d="M${ax},${ay} L${ax},${mid} L${bx},${mid} L${bx},${by}" class="${node.gap ? "gt-gap" : "gt-edge"}"/>`;
  }
  for (const [id, p] of Object.entries(pos)) {
    const node = tree.nodes[id];
    const [fill, border] = GT_KIND[node.kind] || GT_KIND.person;
    const fillC = DARK ? (["narrator", "person"].includes(node.kind) ? v("--surface") : darker(fill, 3.3)) : fill;
    const tip = [node.name];
    if (node.arabic && node.arabic !== node.name) tip.push(node.arabic);
    if (node.count) tip.push(t("{n:,} links in the imported chains", { n: node.count }));
    if (node.gap) tip.push(t("The generations between him and the one above are not known (dashed line)."));
    if (node.disputed) tip.push(t("Disputed: {note}", { note: node.disputed }));
    if (node.note) tip.push(node.note);
    let inner = `<rect width="${GT_W}" height="${GT_H}" rx="10" fill="${fillC}" stroke="${border}" stroke-width="${node.kind !== "narrator" ? 2.2 : 1.2}"${node.disputed ? ' stroke-dasharray="6 4"' : ""}/>`;
    inner += `<rect class="gt-sel" x="-2" y="-2" width="${GT_W + 4}" height="${GT_H + 4}" rx="12"/>`;
    inner += `<text x="${GT_W / 2}" y="20" class="gt-name" style="font:${esc(heading)}">${esc(elide(node.name, heading, GT_W - 14))}</text>`;
    if (node.arabic && node.arabic !== node.name) {
      inner += `<text x="${GT_W / 2}" y="${GT_H - 7}" class="gt-ar" style="font-family:${esc(ar.family)};font-size:${arPt.toFixed(2)}pt;fill:${esc(ar.color)}">${esc(elide(node.arabic, arFont, GT_W - 14))}</text>`;
    }
    boxes += `<g class="gt-box${id === GT.chosen ? " on" : ""}" data-node="${esc(id)}" transform="translate(${p[0]},${p[1]})" data-tip="${esc(tip.join("\n"))}">${inner}</g>`;
  }
  g.innerHTML = lines + boxes;
  GT.scale = 1;
  requestAnimationFrame(() => treeCentre(focus || tree.root));
}

function treeTransform() {
  $("gt-g").setAttribute("transform", `translate(${GT.tx},${GT.ty}) scale(${GT.scale})`);
}

function treeCentre(id) {
  const p = GT.pos[id];
  const board = $("gt-board");
  if (!p) return;
  treeChoose(id);
  GT.tx = board.clientWidth / 2 - (p[0] + GT_W / 2) * GT.scale;
  GT.ty = board.clientHeight / 2 - (p[1] + GT_H / 2) * GT.scale;
  treeTransform();
}

function treeChoose(id) {
  GT.chosen = id;
  for (const b of $("gt-g").querySelectorAll(".gt-box")) b.classList.toggle("on", b.dataset.node === id);
}

function treeZoom(factor, x, y) {
  const board = $("gt-board");
  const cx = x === undefined ? board.clientWidth / 2 : x, cy = y === undefined ? board.clientHeight / 2 : y;
  GT.tx = cx - (cx - GT.tx) * factor;
  GT.ty = cy - (cy - GT.ty) * factor;
  GT.scale *= factor;
  treeTransform();
}

function treeFit() {
  const xs = Object.values(GT.pos);
  if (!xs.length) return;
  const minX = Math.min(...xs.map((p) => p[0])) - 40, maxX = Math.max(...xs.map((p) => p[0])) + GT_W + 40;
  const minY = Math.min(...xs.map((p) => p[1])) - 40, maxY = Math.max(...xs.map((p) => p[1])) + GT_H + 40;
  const board = $("gt-board");
  GT.scale = Math.min(board.clientWidth / (maxX - minX), board.clientHeight / (maxY - minY));
  GT.tx = (board.clientWidth - (maxX - minX) * GT.scale) / 2 - minX * GT.scale;
  GT.ty = (board.clientHeight - (maxY - minY) * GT.scale) / 2 - minY * GT.scale;
  treeTransform();
}

// ------------------------------------------------------------------ the views

function treeLegend() {
  return `<span style="color:${GT_KIND.prophet[1]}">■</span> ${esc(t("prophet"))} &nbsp; `
    + `<span style="color:${GT_KIND.caliph[1]}">■</span> ${esc(t("caliph"))} &nbsp; `
    + `<span style="color:${GT_KIND.imam[1]}">■</span> ${esc(t("Imam"))} &nbsp; `;
}

function familyNote(n) {
  return esc(t("A family read from the narrators' own names: \"X b. Y b. Z\" is taken as the son of \"Y b. Z\" when exactly one narrator of the same tradition and of an earlier or the same generation has that name — {n:,} such links in the imported rijal works. A reading of the names, to be checked against the biographies.", { n }));
}

// one of the four views (show_view); for a narrator's family or line of learning, his (or the first narrator's)
async function showTreeView(key, person, focus) {
  genealogyBuild();
  GT.key = key;
  if (person !== undefined && person !== null) GT.person = person;
  for (const b of document.querySelectorAll(".gt-top [data-view]")) b.classList.toggle("on", b.dataset.view === key);
  $("gt-results").hidden = true;
  const written = GT_WRITTEN.includes(key);
  $("gt-q").placeholder = written ? t("Find in this tree: Ibrahim, Musa, Hashim, Ali …") : t("A narrator: name, kunya or nisba — Arabic or Latin letters");
  $("gt-famous").hidden = written;
  const seq = ++GT.seq;
  const d = await invoke("genealogy_view", { key, person: written ? null : GT.person });
  if (seq !== GT.seq) return;
  const legend = treeLegend();
  const gapLegend = `<span style="color:var(--gold)">╌</span> ${esc(t("generations between not known"))} &nbsp; ▭ ${esc(t("dashed box: the reports differ"))}`;
  GT.none = false;
  if (written) {
    GT.tree = d.tree;
    drawTree(focus);
    const about = key === "prophets"
      ? t("Sources: the Qur'an where it names a relation; al-Bukhari, Sahih (Manaqib al-Ansar: the Prophet's lineage up to 'Adnan); Ibn Hisham, al-Sira; Ibn Sa'd, al-Tabaqat; al-Tabari, Tarikh; Ibn Kathir, al-Bidaya wa-l-Nihaya. The names between Adam and Ibrahim and between Isma'il and 'Adnan are the historians'; they took them partly from the People of the Book, and neither the Qur'an nor an authentic hadith gives them. Click a name for what the sources say.")
      : t("Sources: Ibn Hisham, al-Sira; Ibn Sa'd, al-Tabaqat; Mus'ab al-Zubayri, Nasab Quraysh; Ibn Hazm, Jamharat Ansab al-'Arab. The line of the Prophet's family ﷺ down to the Imams is recorded by the Sunni genealogists too; that they are Imams is the Shia's belief and is shown as theirs. Click a name for what the sources say.");
    $("gt-caption").innerHTML = legend + gapLegend + "<br>" + esc(about);
    await showTreeNode(focus || d.tree.root);
    return;
  }
  if (d.none) {
    GT.tree = null;
    GT.none = true;
    drawTree();
    treeSide(`<div class="lead">${esc(t("No narrator is identified in the imported chains yet."))}</div>`);
    return;
  }
  GT.person = d.person;
  if (key === "family" && !d.tree) {
    GT.tree = null;
    drawTree();
    treeSide(`<div class="lead">${esc(t("No kin of {name} among the narrators: no narrator's name makes him a father or a son. Try another narrator, or his line of learning.", { name: d.no_kin }))}</div>`);
    $("gt-caption").innerHTML = familyNote(d.family_count);
    return;
  }
  GT.tree = d.tree;
  drawTree(`p${d.person}`);
  $("gt-caption").innerHTML = key === "family"
    ? legend.replace(/ &nbsp; $/, "") + "<br>" + familyNote(d.family_count)
    : esc(t("His teachers above him and their teachers, his students below him and theirs — the four most frequent at each step, counted from the imported chains (the number in each box when you point at it). A line of learning, not of blood."));
  await showTreeNode(`p${d.person}`);
}

function treeClicked(id) {
  treeChoose(id);
  showTreeNode(id);
}

function treeSide(html) {
  const side = $("gt-side");
  side.innerHTML = html;
  side.scrollTop = 0;
}

// the side panel (_show_node): the box's names, its kind, its line, its note, his life or the hadith, his entry
async function showTreeNode(id) {
  const tree = GT.tree;
  const node = tree && tree.nodes[id];
  if (!node) return;
  const seq = ++GT.seq;
  const written = GT_WRITTEN.includes(GT.key);
  const extra = await invoke("genealogy_node", { key: GT.key, node: id, person: node.person, current: GT.person });
  if (seq !== GT.seq) return;
  let h = `<div class="gt-name-big">${esc(node.name)}</div>`;
  if (node.arabic && node.arabic !== node.name) h += `<div class="gt-ar-big">${arabicSpan(node.arabic, 1.0)}</div>`;
  const others = [["en", "English"], ["tr", "Türkçe"]].filter(([c]) => node[c] && node[c] !== node.name)
    .map(([c, lang]) => `${lang}: <i>${esc(node[c])}</i>`);
  if (others.length && node.kind !== "narrator") h += `<div class="caption">${others.join(" &nbsp;·&nbsp; ")}</div>`;
  const [, colour] = GT_KIND[node.kind] || GT_KIND.person;
  h += `<div class="caption"><span style="color:${colour}">■</span> ${esc(t(GT_KIND_NAMES[node.kind] || "person"))}</div>`;
  if (written) {
    const line = [];
    for (let n = node; n; n = n.parent ? tree.nodes[n.parent] : null) line.unshift(n);
    if (line.length > 1) {
      const parent = line[line.length - 2];
      h += `<div class="lead">${esc(t("Son of {parent}", { parent: parent.name }) + (node.gap ? " — " + t("generations between not known") : ""))}</div>`;
    }
    const kids = node.children.map((c) => tree.nodes[c].name);
    if (kids.length) h += `<div class="lead">${esc(t("Children shown: {names}", { names: kids.join(", ") }))}</div>`;
    if (line.length > 1) h += `<div class="caption">${esc(t("{n} generations from {root}", { n: line.length - 1, root: line[0].name }))}</div>`;
  }
  if (node.note) h += `<div class="lead">${esc(node.note)}</div>`;
  if (node.disputed) h += `<div class="lead gt-disputed">⚠ ${esc(node.disputed)}</div>`;
  if (node.count) h += `<div class="caption">${esc(t("{n:,} links in the imported chains", { n: node.count }))}</div>`;
  if (extra.life) {
    if (extra.life_person) h += `<div><button class="linkbtn" data-person="${extra.life_person}">${esc(t("Open in Narrators"))}</button></div>`;
    h += lifeCard(extra.life, true);
  } else if (extra.prophet) {
    const f = extra.prophet;
    h += `<div class="gt-sub">${esc(t("Hadith that speak of him"))}</div>`;
    if (!f.total) {
      h += `<div class="caption">${esc(t("No hadith in the imported books names him in words that can only mean him."))}</div>`;
    } else {
      h += `<div class="caption">${esc(t("{n:,} hadith name him (his name with “peace be upon him”, “the prophet of Allah …”, and the words the hadith use of him) — never his bare name, which many narrators bear.", { n: f.total }))}</div>`;
      for (const [road, heading, count] of [["sunni", "Sunni books", f.count_sunni], ["shia", "Shia books", f.count_shia]]) {
        const items = f[road];
        if (!items.length) continue;
        h += `<div class="lead"><b>${esc(t(heading))}</b> · ${count.toLocaleString("en-US")}</div><div class="gt-links">`;
        h += items.map((i) => `<button class="linkbtn" data-hadith="${i.hadith}">${esc(i.label)}</button>`).join("");
        if (count > items.length) h += `<div class="caption">${esc(t("… and {n:,} more", { n: count - items.length }))}</div>`;
        h += "</div>";
      }
      h += `<div class="caption">${esc(t("The verses of the Qur'an about him will be shown here when the Qur'an is in isnady (Q1)."))}</div>`;
    }
  }
  if (node.person) {
    h += `<div class="gt-links">
      <button class="linkbtn" data-person="${node.person}">${esc(t("Open in Narrators"))}</button>
      <button class="linkbtn" data-view-of="family" data-of="${node.person}">${esc(t("His family"))}</button>
      <button class="linkbtn" data-view-of="learning" data-of="${node.person}">${esc(t("His line of learning"))}</button></div>`;
    const r = extra.narrator;
    if (r) h += `<div class="gt-star"><button class="star${r.marked ? " on" : ""}" data-star="narrator">${r.marked ? "★" : "☆"}</button><span class="caption">${esc(t("Bookmark this narrator"))}</span></div>`;
  }
  const ts = extra.tree_star;
  if (ts) {
    const text = written ? t("Bookmark this place in the tree") : GT.key === "family" ? t("Bookmark his family tree") : t("Bookmark his line of learning");
    h += `<div class="gt-star"><button class="star${ts.marked ? " on" : ""}" data-star="tree">${ts.marked ? "★" : "☆"}</button><span class="caption">${esc(text)}</span></div>`;
  }
  treeSide(h);
  if (extra.life) fillEntries(extra.life, "#gt-side");
  const side = $("gt-side");
  const nstar = side.querySelector('[data-star="narrator"]');
  if (nstar && extra.narrator) wireStar(nstar, "narrator", extra.narrator.key, extra.narrator.title, extra.narrator.arabic || "");
  const tstar = side.querySelector('[data-star="tree"]');
  if (tstar) wireStar(tstar, "tree", ts.ref, ts.title, ts.subtitle || "");
}

function genealogySideClick(e) {
  const a = e.target.closest("[data-person], [data-view-of], [data-hadith], a[data-name], a[data-book], a[data-url]");
  if (!a) return;
  if (a.dataset.viewOf) return showTreeView(a.dataset.viewOf, Number(a.dataset.of));
  if (a.dataset.person) {
    const id = Number(a.dataset.person);
    return place(["narrator", id], async () => {
      NARR.id = id;
      show("narrators");
      await selectNarrator(id);
    });
  }
  if (a.dataset.hadith) return openInBookPlace(Number(a.dataset.hadith));
  if (a.dataset.name) return findNarrator(a.dataset.name);
  if (a.dataset.book) return openBookAt(a.dataset.book);
  if (a.dataset.url) return invoke("open_url", { url: a.dataset.url });
}

// ------------------------------------------------------------------ finding

async function treeFindLive() {
  const text = $("gt-q").value.trim();
  if (!GT_WRITTEN.includes(GT.key) || !GT.tree || text.length < 2) return;
  const hits = await invoke("genealogy_find_in_tree", { key: GT.key, text });
  if (hits.length && $("gt-q").value.trim() === text) treeCentre(hits[0]);
}

async function treeFind() {
  const text = $("gt-q").value.trim();
  if (!text) return;
  if (GT_WRITTEN.includes(GT.key) && GT.tree) {
    const hits = await invoke("genealogy_find_in_tree", { key: GT.key, text });
    if (hits.length) {
      treeCentre(hits[0]);
      showTreeNode(hits[0]);
    }
    return;
  }
  const rows = await invoke("genealogy_find_narrators", { text });
  treeResults(rows.map((r) => resultRow(r)).join(""), 170);
}

function treeResults(html, height) {
  const box = $("gt-results");
  box.innerHTML = html;
  box.style.maxHeight = height + "px";
  box.hidden = !html;
}

// a narrator found: the name people know him by, when, and his Arabic name (_result_item, FS1)
function resultRow(r) {
  const view = r.view || {};
  const known = r.famous || view.reading || view.full_reading || "";
  const facts = [];
  if (r.tradition === "shia") facts.push(t("Shia"));
  if (r.tabaqa) facts.push(t("tabaqa {tabaqa}", { tabaqa: r.tabaqa }));
  if (r.death) facts.push(t("d. {year}", { year: r.death }));
  if (r.in_chains) facts.push(t("{n:,} in chains", { n: r.in_chains }));
  let label = (known ? [known, ...facts] : facts).join(" · ");
  if (!(r.famous && APP.ui_language === "ar")) label += "   " + (view.arabic || r.name);
  const tip = r.famous && view.reading ? ` data-tip="${esc(view.full_reading || view.reading)}"` : "";
  return `<div class="gt-result" data-person="${r.id}"${tip}>${esc(label.trim())}</div>`;
}

// the famous Companions group by group (the four Caliphs, the Ten …), each a click from his tree
async function treeFamous() {
  const d = await invoke("narrators_famous", { tradition: "" });
  let html = "";
  for (const g of d.groups) {
    html += `<div class="gt-result-head">${esc(t(g.title))}</div>`;
    html += g.rows.map((r) => resultRow(r)).join("");
  }
  treeResults(html, 360);
}

// ------------------------------------------------------------------ opened from elsewhere

// a place in a genealogy (history ["tree", which, target]): a box of a written tree, or a narrator's family / line
async function openTreeAt(which, target) {
  show("genealogy");
  genealogyBuild();
  if (GT_WRITTEN.includes(which)) {
    await showTreeView(which, null, target);
    return;
  }
  await showTreeView(which, Number(target));
}

function openTree(which, target) {
  return place(["tree", which, target], () => openTreeAt(which, target));
}

// a narrator's trees from the Narrators page (history ["genealogy", person, which]): his family, his line of
// learning, or his box in the tree of Quraysh when his life is written
async function openGenealogyAt(person, which) {
  show("genealogy");
  genealogyBuild();
  if (GT_WRITTEN.includes(which)) {
    const where = await invoke("genealogy_place_of", { person });
    if (where) {
      await showTreeView(where[0], null, where[1]);
      return;
    }
  }
  await showTreeView(which, person);
}

function openGenealogy(person, which) {
  return place(["genealogy", person, which], () => openGenealogyAt(person, which));
}
