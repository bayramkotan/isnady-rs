"use strict";
// Companions and the Household (gui/companions_page.py, AM1, EB1): the ten promised Paradise, the people of the cloak,
// the Mothers of the Believers on the Sunni road; the Fourteen Infallibles on the Shia road — each road on its own (SH1).
//
// [Companions and the Household] | the group: its title, its road, what the sources say of it
// [Sunni] [Shia]                 | its members, two to a row, with what the books imported here hold of each
// the groups of the road         | the hadith that name it, as Search shows them

const CP = { built: false, road: "sunni", current: null, seq: 0 };

function companionsBuild() {
  if (CP.built) return;
  CP.built = true;
  const page = $("page-companions");
  page.className = "page narrators-page companions-page";
  page.innerHTML = `<div class="np">
    <div class="np-left cmp-left">
      <div class="np-title">${esc(t("Companions and the Household"))}</div>
      <div class="np-segs" id="cmp-segs">${[["sunni", "Sunni"], ["shia", "Shia"]].map(([k, l]) =>
        `<button class="seg" data-road="${k}">${esc(t(l))}</button>`).join("")}</div>
      <div class="caption">${esc(t("Each group as its own road gives it: the hadith that names it, what its scholars read in it, and what the books imported here hold of its members."))}</div>
      <div class="np-list" id="cmp-list"></div>
    </div>
    <div class="np-right" id="cmp-view"></div>
  </div>`;
  $("cmp-segs").onclick = async (e) => {
    const b = e.target.closest("button[data-road]");
    if (!b) return;
    CP.road = b.dataset.road;
    await companionsRefresh();
    if (CP.current) record(["group", CP.current]);
  };
  $("cmp-list").onclick = (e) => {
    const row = e.target.closest("[data-group]");
    if (row) place(["group", row.dataset.group], () => showGroup(row.dataset.group));
  };
  $("cmp-view").addEventListener("click", (e) => {
    const b = e.target.closest("[data-person], [data-node]");
    if (!b) return;
    if (b.dataset.person) {
      const id = Number(b.dataset.person);
      place(["narrator", id], async () => {
        NARR.id = id;
        show("narrators");
        await selectNarrator(id);
      });
    } else {
      openTree("quraysh", b.dataset.node);
    }
  });
}

// the list of the road chosen, the group chosen in it (or its first)
async function companionsRefresh() {
  companionsBuild();
  for (const b of $("cmp-segs").children) b.classList.toggle("on", b.dataset.road === CP.road);
  const groups = await invoke("companions_groups", { road: CP.road });
  $("cmp-list").innerHTML = groups.map((g) => `<div class="cmp-row" data-group="${esc(g.id)}">${esc(g.title)}</div>`).join("");
  const id = groups.some((g) => g.id === CP.current) ? CP.current : (groups[0] || {}).id;
  if (id) await showGroup(id);
}

// a group opened from elsewhere (Back, the start): its road, it chosen
async function openGroupAt(id) {
  show("companions");
  companionsBuild();
  const d = await invoke("companion_group", { id });
  if (d.error) return;
  CP.current = id;
  CP.road = d.road;
  await companionsRefresh();
}

function openGroup(id) {
  return place(["group", id], () => openGroupAt(id));
}

function memberCard(m, shia) {
  let html = `<div class="cmp-name">${esc(m.name)}</div>`;
  if (m.arabic && m.arabic !== m.name) html += `<div class="cmp-ar">${arabicSpan(m.arabic, 0.7)}</div>`;
  const facts = [];
  if (m.died) facts.push(esc(m.died));
  if (shia) {
    facts.push(m.ends ? t("<b>{n:,}</b> chains end here in the Shia books", { n: m.ends }) : esc(t("no chain ends here in the books imported here")));
  } else if (m.person && m.links) {
    facts.push(t("in <b>{links:,}</b> chain links, of <b>{hadith:,}</b> hadith", { links: m.links || 0, hadith: m.hadith || 0 }));
  } else if (m.person) {
    facts.push(esc(t("not identified in the chains imported here")));
  }
  if (facts.length) html += `<div class="caption">${facts.join(" · ")}</div>`;
  if (m.summary) {
    const text = m.summary;
    const short = text.length <= 220 ? text : text.slice(0, 220).replace(/ [^ ]*$/, "") + " …";
    html += `<div class="lead" data-tip="${esc(text)}">${esc(short)}</div>`;
  }
  const buttons = [];
  if (m.person) buttons.push(`<button class="quiet" data-person="${m.person}">${esc(t("Life and narrations"))}</button>`);
  if (m.node) buttons.push(`<button class="quiet" data-node="${esc(m.node)}">🌳 ${esc(t("In the family tree"))}</button>`);
  if (buttons.length) html += `<div class="sc-buttons">${buttons.join("")}</div>`;
  return `<div class="cmp-member">${html}</div>`;
}

async function showGroup(id) {
  const seq = ++CP.seq;
  const d = await invoke("companion_group", { id });
  if (seq !== CP.seq || d.error) return;
  CP.current = id;
  for (const r of $("cmp-list").children) r.classList.toggle("on", r.dataset.group === id);
  const shia = d.road === "shia";
  let html = `<div class="ncard"><div class="cmp-title">${esc(d.title)}</div>
    <div class="nc-tags"><span class="${shia ? "tag-gold" : "tag-muted"}">${esc(t(shia ? "Shia road" : "Sunni road"))}</span></div>
    <div class="lead">${esc(d.lead)}</div></div>`;
  let m = "";
  if (shia && d.shared) {
    m += `<div class="caption">${esc(t("{n:,} more chains of the Shia books end at an Imam not named, or named by a kunya several of them share (Abu al-Hasan); they are not counted under one.", { n: d.shared }))}</div>`;
  }
  m += `<div class="cmp-grid">${d.members.map((x) => memberCard(x, shia)).join("")}</div>`;
  html += `<div class="ncard"><div class="ncard-title cmp-card-title">${esc(t("Its members") + ` (${d.members.length})`)}</div>${m}</div>`;
  if (d.refs.length) {
    let c = "";
    if (!d.found.length) c += `<div class="caption">${esc(t("Not in the books imported here: {refs}", { refs: d.refs.join(", ") }))}</div>`;
    c += `<div class="caption">${esc(t("As the books imported here give them; each opens in its book and its chain."))}</div>`;
    html += `<div class="ncard"><div class="ncard-title cmp-card-title">${esc(t("The hadith that name them"))}</div>${c}</div>`;
  }
  const view = $("cmp-view");
  view.innerHTML = html;
  view.scrollTop = 0;
  if (d.found.length) {
    const cards = await invoke("hadith_cards", { ids: d.found });
    if (seq !== CP.seq) return;
    for (const r of cards) view.appendChild(card(r));
  }
}
