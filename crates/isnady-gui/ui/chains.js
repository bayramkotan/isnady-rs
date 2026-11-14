"use strict";
// Isnad Chains (gui/isnad_page.py, gui/chain_widgets.py ChainNode, gui/chapter_picker.py): one hadith's chain drawn
// as a timeline, the Arabic text with the chain and the text (matn) told apart, and per-book statistics.

const CH = { built: false, filled: false, book: null, current: null, tradition: "sunni", nav: null, groups: [] };

// Python's f"{x:.1f}": halves of the exact value to the even digit
function fixed1(x) {
  const k = x * 10;
  if (Number.isInteger(k * 2) && !Number.isInteger(k)) {
    const f = Math.floor(k);
    return ((f % 2 === 0 ? f : f + 1) / 10).toFixed(1);
  }
  return x.toFixed(1);
}

const thousands = (n) => Number(n).toLocaleString("en-US");

function chainsBuild() {
  if (CH.built) return;
  CH.built = true;
  const page = $("page-chains");
  page.classList.add("chains-page");
  page.innerHTML = `<div class="cp">
      <div class="cp-top">
        <div class="cp-row">
          <span class="flabel">${esc(t("Book"))}</span>
          <button class="combo cp-book" id="c-book"><span class="lb"></span><i class="chev"></i></button>
          <span class="cp-gap"></span>
          <button class="quiet" id="c-contents" data-tip="${esc(t("Choose a chapter of this book, then one of its hadith, without knowing its number"))}">📖  ${esc(t("From the contents…"))}</button>
          <button class="quiet" id="c-random" data-tip="${esc(t("A hadith of this book chosen at random, among those whose chain was read"))}">${esc(t("Random hadith"))}</button>
        </div>
        <div id="c-nav"></div>
      </div>
      <div class="cp-body" id="c-body"></div>
    </div>`;
  CH.nav = hadithNavigator($("c-nav"), t("Show chain"), chainGo);
  $("c-book").onclick = bookPopup;
  $("c-random").onclick = chainRandom;
  $("c-contents").onclick = fromContents;
}

// the page is shown the first time (main_window._lazy: refresh): the books, then the hadith shown or the overview;
// once, whoever asks first (the sidebar, or a hadith opened from another page)
function chainsRefresh() {
  chainsBuild();
  if (!CH.refreshing) {
    CH.refreshing = (async () => {
      if (!SEARCH.groups.length) await fillSearchFilters();
      CH.groups = SEARCH.groups;
      CH.filled = true;
      const first = CH.groups.length && CH.groups[0].books.length ? CH.groups[0].books[0].key : null;
      if (!CH.book) CH.nav.ready = setBook(first);
      if (CH.current) await showHadith(CH.current.id);
      else await showOverview();
    })();
  }
  return CH.refreshing;
}

function bookName(key) {
  for (const g of CH.groups) {
    const b = g.books.find((x) => x.key === key);
    if (b) return `    ${b.name} (${thousands(b.count)})`;
  }
  return "";
}

function setBook(key) {
  CH.book = key;
  $("c-book").querySelector(".lb").textContent = key ? bookName(key) : "";
  return CH.nav.setBook(key);
}

// the book list: the Sunni and Shia collections apart, each under its own heading (widgets.fill_book_combo)
function bookPopup() {
  closeMenus();
  const button = $("c-book");
  const r = button.getBoundingClientRect();
  const el = document.createElement("div");
  el.className = "menu combo-list book-list";
  el.style.minWidth = r.width + "px";
  let html = "";
  CH.groups.forEach((g, i) => {
    if (i) html += '<div class="sep"></div>';
    html += `<div class="mi head"><span class="lb">${esc(t("{group} collections", { group: t(g.label) }))}</span></div>`;
    for (const b of g.books) {
      html += `<div class="mi${b.key === CH.book ? " on" : ""}" data-v="${esc(b.key)}"><span class="lb">${esc(bookName(b.key))}</span></div>`;
    }
  });
  el.innerHTML = html;
  el.onmousedown = (e) => e.preventDefault();
  el.onclick = (e) => {
    const row = e.target.closest(".mi[data-v]");
    if (!row) return;
    closeMenus();
    if (row.dataset.v !== CH.book) setBook(row.dataset.v);
  };
  el.onmouseover = (e) => {
    const row = e.target.closest(".mi[data-v]");
    if (!row) return;
    for (const x of el.querySelectorAll(".mi.on")) x.classList.remove("on");
    row.classList.add("on");
  };
  document.body.appendChild(el);
  // twenty at a time, the rest under a scroll bar (setMaxVisibleItems(20))
  const rows = el.querySelectorAll(".mi, .sep");
  if (rows.length > 20) {
    el.style.maxHeight = (rows[20].offsetTop - rows[0].offsetTop) + "px";
    el.style.overflowY = "auto";
  }
  placeMenu(el, { x: isRtl() ? r.right : r.left, y: r.bottom, top: r.top, flipUp: true, alignEnd: isRtl() });
  MENU.stack.push({ el, items: [], index: -1, item: null });
  const on = el.querySelector(".mi.on");
  if (on) on.scrollIntoView({ block: "nearest" });
}

async function chainRandom() {
  if (!CH.book) return;
  const n = await invoke("random_number", { book: CH.book });
  if (n) {
    CH.nav.field.value = n;
    chainGo();
  }
}

async function openFirst(book) {
  setBook(book);
  const rng = await invoke("book_numbers", { book });
  if (rng) {
    CH.nav.field.value = rng.first;
    chainGo();
  }
}

// a number to show (Enter, Show chain, a step): its hadith, or why there is none
async function chainGo() {
  const book = CH.book, number = CH.nav.number();
  if (!book || !number) return;
  const d = await invoke("hadith_by_number", { book, number });
  if (d.id) {
    await showHadith(d.id);
    return;
  }
  const body = $("c-body");
  const hint = d.sample.some((x) => x.includes(":"))
    ? " " + t("Its numbers are written like {sample} …", { sample: d.sample.join(", ") }) + " — " + t("volume:hadith.") : "";
  const span = d.range ? " " + t("Its numbers run from {first} to {last}.", { first: d.range.first, last: d.range.last }) : "";
  let html = `<div class="lead c-wrong">${esc(t("There is no hadith {number} in this book.{hint}", { number, hint }) + span)}</div>`;
  if (d.near) html += `<div class="sp8"></div><div><button class="quiet" id="c-near">${esc(t("Show {number} instead", { number: d.near }))}</button></div>`;
  body.innerHTML = html;
  if (d.near) $("c-near").onclick = () => { CH.nav.field.value = d.near; chainGo(); };
}

// ------------------------------------------------------------------ the overview

async function showOverview() {
  const body = $("c-body");
  await CH.nav.ready;
  $("c-nav").querySelector(".hn-prev").disabled = true;
  $("c-nav").querySelector(".hn-next").disabled = true;
  const d = await invoke("chains_overview");
  if (d.not_read) {
    body.innerHTML = `<div class="notice">${esc(t("Bringing the chains up to date after the new version (once)…"))}</div>`;
    return;
  }
  let html = `<div class="c-hero">${esc(t("Chains of transmission"))}</div><div class="sp6"></div>
    <div class="lead c-lead">${esc(t("Every chain below was read from the Arabic text: who narrated to whom, with which words, and whom it reaches. Each book is read along the road of its own tradition: a Sunni book's chain ends at the Prophet ﷺ and its narrators are identified in Ibn Hajar's Taqrib; a Shia book's chain is read the Shia way, ends at a Ma'sum and its narrators are identified in al-Najashi. Names are shown exactly as written. Where the wording could not be read with confidence, the chain is kept whole and not split, so nothing is guessed. Choose a book and a number, or open a chain from a search result."))}</div>
    <div class="sp18"></div>`;
  for (const b of d.books) html += statsCard(b) + '<div class="sp12"></div>';
  body.innerHTML = html;
  body.scrollTop = 0;
  requestAnimationFrame(() => squareBoxes(body, ".tag-gold"));
  for (const c of body.querySelectorAll(".c-stats[data-key]")) {
    c.addEventListener("click", (e) => { if (e.button === 0) openFirst(c.dataset.key); });
  }
}

function statsCard(b) {
  const shia = b.tradition === "shia";
  const split = b.chains ? 100 * b.split / b.chains : 0;
  const marfu = b.split ? 100 * b.marfu / b.split : 0;
  const avg = b.split ? b.links / b.split : 0;
  const cells = [[thousands(b.chains), t("chains read")], [fixed1(split) + "%", t("split into narrators")],
    [fixed1(marfu) + "%", shia ? t("of those reach a Ma'sum") : t("of those reach the Prophet")],
    [fixed1(avg), t("narrators per chain")]];
  if (b.known) cells.push([fixed1(100 * (b.known[1] || 0) / b.known[0]) + "%", shia ? t("identified in al-Najashi") : t("identified in the Taqrib")]);
  let html = `<div class="c-card c-stats"${b.key ? ` data-key="${esc(b.key)}" data-tip="${esc(t("Click to read the chains of this book, from its first hadith"))}"` : ""}>
    <div class="c-stats-top"><span class="c-card-title">${esc(b.name)}</span>${shia ? `<span class="tag-gold">${esc(t("Shia"))}</span>` : ""}</div>
    <div class="c-cells">${cells.map(([v, c]) => `<div class="c-cell"><div class="c-value">${esc(v)}</div><div class="caption">${esc(c)}</div></div>`).join("")}</div>`;
  if (b.problems.length) {
    const lines = b.problems.map(([p, n]) => `${thousands(n)} &nbsp;${esc(t(p))}`).join("<br>");
    html += `<div class="caption">${t("<span style='font-weight:600'>Kept whole, not split:</span><br>{lines}", { lines })}</div>`;
  }
  return html + "</div>";
}

// ------------------------------------------------------------------ one hadith

// a hadith opened from elsewhere (main_window._open_chain): the page, and the place in Back and Forward
async function openChain(hid) {
  await place(["chain", hid], async () => {
    show("chains");
    await chainsRefresh();
    await showHadith(hid);
  });
}

// a hadith by its book and number, opened from elsewhere (a search result, a narrator's hadith, Books)
async function showChain(book, number) {
  const d = await invoke("hadith_by_number", { book, number: String(number) });
  if (d.id) await openChain(d.id);
}

async function showHadith(hid) {
  const d = await invoke("chain_page", { id: hid });
  if (!d) {
    CH.current = null;
    await showOverview();
    return;
  }
  const info = d.info;
  CH.current = info;
  if (info.book !== CH.book) {
    CH.book = info.book;
    $("c-book").querySelector(".lb").textContent = bookName(info.book);
    await CH.nav.setBook(info.book);
  }
  CH.nav.setNumber(info.number);
  CH.tradition = d.tradition;
  const P = d.problems;
  const chains = d.chains;
  const chain = chains[0] || null;

  let html = `<div class="c-head"><span class="c-title">${esc(info.book_name)}</span><span class="c-number">${esc(t("hadith {number}", { number: info.number }))}</span>`;
  if (d.tradition === "shia") {
    html += `<span class="tag-gold" data-tip="${esc(t("Road B: the teacher named without a term, al-Kulayni's groups, the chain ending at a Ma'sum; the narrators identified only in the Shia rijal works and judged in the Imami terms."))}">${esc(t("Shia — read the Shia way"))}</span>`;
  }
  html += "</div>";
  const drawn = chains.filter((c) => c.links.length || c.problem === P.direct);
  if (!drawn.length) {
    // nothing to draw: said plainly, never an empty page
    let why;
    if (!d.text) why = t("No Arabic text for this hadith, so no chain was read.");
    else if (chain && chain.problem === P.continued) why = t("This text goes on from the hadith before it and names no chain of its own; none is drawn for it. The text is below.");
    else if (chain && chain.problem === P.note) why = t("These are the compiler's own words, not a narration: there is no chain to draw. The text is below.");
    else if (chain && chain.problem && chain.problem !== P.no_term && chain.problem !== P.no_chain) why = t("No chain could be read from this hadith's text: {reason}. Its text is below.", { reason: t(chain.problem) });
    else why = t("This book gives the hadith without a chain (the compiler names only the Companion, or none); its text is below.");
    html += `<div class="lead c-lead">${esc(why)}</div>`;
  } else if (!chain || !chain.problem || chain.problem === P.borrowed) {
    html += `<div class="lead c-lead">${esc(verdictWords(chain, P))}</div>`;
  }
  const borrowed = drawn.filter((c) => c.problem === P.borrowed);
  if (borrowed.length && chain && chain.problem !== P.borrowed) {
    html += `<div class="lead c-lead">${esc(t("This road ends where its text says \"بهذا الإسناد\" (with the chain of the hadith before). The chain marked \"from the hadith before\" goes on as that hadith's chain does; it is not written here and is not counted in the statistics."))}</div>`;
  }
  html += '<div class="sp16"></div>';
  const own = drawn.filter((c) => c.problem !== P.borrowed);
  if (own.length > 1) {
    html += `<div class="c-count">${esc(n_("{n} chain", "{n} chains of this hadith: the compiler joined roads (tahwil, ح), a second road follows a link (\"وعن\", \"وحدثنا\") or two narrators stand at one link; each is drawn whole, the links they share included.", own.length, { n: own.length }))}</div><div class="sp8"></div>`;
  }
  drawn.forEach((c, i) => {
    if (drawn.length > 1) {
      html += `<div class="c-section">${esc(t("Chain {i}", { i: i + 1 }) + (c.problem === P.borrowed ? " — " + t("from the hadith before (بهذا الإسناد), not counted in the statistics") : ""))}</div>`;
    }
    html += drawChain(c, info, P);
    html += '<div class="sp14"></div>';
  });
  html += '<div class="sp4"></div>';
  if (d.text) html += textCard(d, chain, drawn, P);
  const body = $("c-body");
  body.innerHTML = html;
  body.scrollTop = 0;
  requestAnimationFrame(() => {
    squareBoxes(body, ".tag-gold");
    drawGutters(body);
  });
}

// what the page says first about the chain (isnad_page._verdict)
function verdictWords(chain, P) {
  if (!chain) return t("No Arabic text for this hadith, so no chain was read.");
  if (chain.problem === P.borrowed) {
    return t("This hadith names no chain of its own: the book says it is the chain of the hadith before it (\"وعنه\", \"وبهذا الإسناد\"). The chain drawn is taken from that hadith and is not counted in the statistics.");
  }
  if (chain.problem) return t("This chain was kept whole and not split: {reason}. The full wording is shown below.", { reason: t(chain.problem) });
  const n = chain.links.length;
  const known = chain.links.filter((l) => l.person).length;
  const looked = chain.links.some((l) => l.candidates !== null && l.candidates !== undefined);
  if (CH.tradition === "shia" && n) {
    let text;
    if (chain.end) text = n_("Read as a Shia chain: {n} link between the compiler and {end}.", "Read as a Shia chain: {n} links between the compiler and {end}.", n, { n, end: chain.end });
    else if (chain.raised) {
      text = n_("Read as a Shia chain: {n} link, then raised (رفعه) without the rest of its links — a gap (marfu' in the Imami usage): whom it reaches is not named.",
        "Read as a Shia chain: {n} links, then raised (رفعه) without the rest of its links — a gap (marfu' in the Imami usage): whom it reaches is not named.", n, { n });
    } else {
      text = n_("Read as a Shia chain: {n} link; it stops before a Ma'sum is named (maqtu', or the last link could not be read with confidence).",
        "Read as a Shia chain: {n} links; it stops before a Ma'sum is named (maqtu', or the last link could not be read with confidence).", n, { n });
    }
    if (looked) text += " " + t("{known} of them identified in the Shia rijal works (al-Najashi …).", { known });
    if (chain.jamian) text += " " + t("A second road joins it (\"jami'an\"); the first road is drawn.");
    return text;
  }
  if (chain.reaches_prophet) {
    let text = n_("{n} narrator between the compiler and the Prophet, as read from the text.", "{n} narrators between the compiler and the Prophet, as read from the text.", n, { n });
    if (looked) text += " " + t("{known} of them identified in Ibn Hajar's Taqrib al-Tahdhib.", { known });
    return text;
  }
  return t("The chain as read stops before the Prophet: the report may be from a Companion or a later narrator, or the last link could not be read with confidence.");
}

function meetingWords(m, links, i) {
  const before = i ? links[i - 1].short : "";
  const here = links[i].short;
  if (m.said) {
    return t("The tahwil (ح): this road joins the others here — the text says they all narrate from {here} (“كلهم عن …”), and {before} is found narrating from him in {n} other hadith.",
      { here, before, n: m.attested });
  }
  return t("The tahwil (ح): the text does not say where this road joins the others; it goes on here because {before} is found narrating from {here} in {n} other hadith of these books, in chains read whole.",
    { before, here, n: m.attested });
}

// one chain from the compiler's book to whom it reaches, node by node (isnad_page._draw_chain)
function drawChain(chain, info, P) {
  if (!chain || !(chain.links.length || chain.problem === P.direct)) return "";
  let html = chainNode({ title: info.book_name, subtitle: t("The compiler's book, where the chain begins"), first: true, latin: true,
    click: `book:${info.id}`, clickTip: t("Click to read this hadith in its book, among the hadith of its chapter") });
  const links = chain.links;
  if (!links.length) {
    // the words of the Prophet ﷺ or the Ma'sum with no narrator between
    html += chainNode({ title: "…", subtitle: t("The compiler gives no narrators between himself and whom the words are from") });
  }
  const m = chain.meeting;
  links.forEach((l, i) => {
    if (m && l.position === m.first_position) html += `<div class="caption c-meet">${esc("⤷ " + meetingWords(m, links, i))}</div>`;
    let click = null;
    if (!l.person && !l.note && l.candidates) click = `find:${l.short}`;
    html += chainNode({ title: l.short, term: l, fullName: l.raw, person: l.person, candidates: l.candidates, note: l.note,
      confidence: l.confidence, click, clickTip: t("Click to look for this name among the narrators"), readingText: l.name,
      reading: l.reading, spelled: l.spelled });
  });
  if (chain.reaches_prophet || chain.ends_with) {
    const key = chain.ends_with || "prophet";
    const node = endNode(key);
    html += chainNode({ title: endChip(chain), subtitle: key === "prophet" ? t("The Messenger of God") : chain.end, last: true, prophet: true,
      click: node ? `tree:${node}` : null,
      clickTip: key === "prophet" ? t("Click to read his life ﷺ and his place in the family of the Prophet ﷺ") : t("Click to read his life and his place in the family of the Prophet ﷺ") });
  } else if (chain.raised) {
    html += chainNode({ title: "رفعه", subtitle: t("Raised (marfu'): the rest of the chain is left out — whom it reaches is not named"), last: true });
  } else if (chain.problem && chain.problem !== P.borrowed) {
    html += chainNode({ title: "…", subtitle: t("The chain is read as far as here; the rest is in the text. ({reason})", { reason: t(chain.problem) }), last: true });
  }
  return html;
}

// the short Arabic name of whom a chain reaches: the Prophet ﷺ, or on road B one of the Imams (chain_widgets.end_chip)
function endChip(chain) {
  const key = chain.ends_with || "prophet";
  if (key === "prophet") return "النبي ﷺ";
  return (WIN.masum_ar || {})[key] || "الإمام عليه السلام";
}

function endNode(key) {
  const node = key === "prophet" ? "muhammad" : key;
  return (WIN.quraysh || []).includes(node) ? node : null;
}

// how a narrator of a chain was identified, from the matcher's confidence
function identifiedHow(c) {
  if (c === null || c === undefined) return "";
  if (c >= 1.0) return t("Identified by his name as the rijal work gives it.");
  if (c >= 0.8) return t("Identified by the rijal work's words and the chain around him: the book's mark, the generations, his patron, his father's name in his son's entry, the compiler's own teacher.");
  if (c >= 0.7) return t("isnady's guess, by its own rules: a list of who narrates from whom, a name isnady knows him by, a name found inside an entry, or a father weighed from his son's nasab — not the rijal work's words (Tools → Narrator Guesses).");
  if (c >= 0.6) return t("isnady's guess: the narrators around him — this teacher and student are seen with him in the chains that name him in full (Tools → Narrator Guesses).");
  return t("isnady's guess: the famous narrator of this name — nothing else decided between those who bear it (Tools → Narrator Guesses).");
}

// a narrator isnady's own inference decided, not the rijal works' words (chain_widgets.is_guess)
function isGuess(c) {
  return c !== null && c !== undefined && c < 0.8;
}


function shorten(text, limit) {
  const chars = Array.from(text || "");
  if (chars.length <= limit) return text;
  const cut = chars.slice(0, limit).join("");
  const at = cut.lastIndexOf(" ");
  return (at >= 0 ? cut.slice(0, at) : cut) + " …";
}

function nodeCritic(v) {
  if (v.shia) return t(v.critic_name || "al-Najashi");
  return (v.critic_name || "").includes("حجر") || !v.critic_name ? t("Ibn Hajar") : v.critic_name;
}

// one step of the vertical chain: the term that links it to the previous step, then the name (ChainNode)
function chainNode(o) {
  const ar = scriptOf("arabic");
  const scale = `var(--scale)`;
  let card = "";
  if (o.term) {
    const l = o.term;
    card += `<div class="c-term" dir="auto" data-tip="${esc(`${l.term_ar}: ${l.term_meaning}\n${l.term_why}`)}"><span class="c-term-ar">${esc(l.term_ar)}</span>&nbsp;&nbsp;${esc(l.term_meaning)}</div>`;
  }
  const nameTip = o.fullName && o.fullName !== o.title ? ` data-tip="${esc(o.fullName)}"` : "";
  let name;
  if (o.latin) name = `<div class="c-name latin"${nameTip}>${esc(o.title)}</div>`;
  else {
    name = `<div class="c-name" dir="rtl"${nameTip} style="font-family:${esc(ar.family)};font-size:calc(${(ar.size * 0.9).toFixed(2)}pt * ${scale});color:${esc(ar.color)}${o.prophet ? ";font-weight:700" : ""}">${esc(o.title)}</div>`;
  }
  let reading = null;
  if (!o.latin && !o.prophet && !isRtl()) {
    let text = o.reading || "";
    if (o.readingText) text = o.readingText.charAt(0).toUpperCase() + o.readingText.slice(1);
    if (text) {
      const tip = t("The name as it is written in the chain, in Latin letters.") + (o.spelled ? "\n" + t("≈ a word not in isnady's list of names was spelled letter by letter: its short vowels are a guess.") : "");
      reading = `<div class="c-reading" data-tip="${esc(tip)}">${esc(text)}${o.spelled ? ' <span class="muted">≈</span>' : ""}</div>`;
    }
  }
  card += reading ? `<div class="c-line">${reading}${name}</div>` : name;
  if (o.subtitle) card += `<div class="caption c-sub">${esc(o.subtitle)}</div>`;
  const p = o.person;
  let click = o.click, clickTip = o.clickTip;
  if (p) {
    const how = identifiedHow(o.confidence);
    const tip = p.name_ar + (p.other_names.length ? "\n" + p.other_names.join("، ") : "") + (how ? `\n\n${how}` : "") + t("\n\nClick to open this narrator.");
    card += `<div class="c-who" dir="rtl" data-tip="${esc(tip)}" style="font-family:${esc(ar.family)};font-size:calc(${(ar.size * 0.62).toFixed(2)}pt * ${scale})">${isGuess(o.confidence) ? "≟ " + esc(t("isnady's guess")) + " · " : ""}${esc(shorten(p.display_name, 90))}</div>`;
    const readingName = p.reading || p.latin;
    if (readingName) {
      card += `<div class="c-facts"${p.how ? ` data-tip="${esc(t("known by {how}", { how: p.how }))}"` : ""}><b>${esc(readingName)}</b>${p.full_reading && p.full_reading !== readingName ? ` &nbsp;·&nbsp; ${esc(p.full_reading)}` : ""}</div>`;
    }
    const v = p.verdict;
    if (v) {
      let rank;
      if (v.shia) {
        rank = v.rank ? `&nbsp;&nbsp;<span style="color:${rankColor(SHIA_COLOR[v.rank])}">●</span> ${esc(v.label)}` : "&nbsp;&nbsp;<i>" + esc(t("no judgment in these words")) + "</i>";
        if (v.madhhab) rank += ` · ${esc(v.madhhab)}`;
      } else {
        rank = v.rank ? `&nbsp;&nbsp;<span style="color:${rankColor(v.rank)}">●</span> ` + esc(t("rank {rank} of 12", { rank: v.rank })) + `: ${esc(v.label)}` : "";
      }
      const mark = isRtl() ? "‏" : "‎";
      card += `<div class="c-verdict" data-tip="${esc(`${v.critic_name}, ${v.work || "None"}:\n${v.phrase}`)}">${esc(nodeCritic(v))}: <span style="font-family:${esc(ar.family)};font-size:calc(${(ar.size * 0.68).toFixed(2)}pt * ${scale})">${esc(shorten(v.phrase, 70))}</span>${mark}${rank}</div>`;
    }
    const facts = [];
    if (p.tabaqa) facts.push(t("tabaqa {tabaqa}", { tabaqa: p.tabaqa }) + `: ${p.tabaqa_label}`);
    if (p.death) facts.push(t("died {year} AH", { year: p.death }));
    else if (p.death_note) facts.push(p.death_note);
    if (facts.length) card += `<div class="c-facts" dir="auto">${esc(facts.join(" · "))}</div>`;
    click = `narrator:${p.id}`;
    clickTip = t("Click to open this narrator.");
  } else if (o.note && !o.prophet) {
    card += `<div class="c-unknown">${esc(o.note)}</div>`;
  } else if (o.candidates !== null && o.candidates !== undefined && !o.prophet && !o.latin) {
    card += `<div class="c-unknown">${esc(o.candidates ? t("Not identified yet: {n} narrators in the rijal work could have this name", { n: o.candidates })
      : t("Not identified: no narrator of this name in the rijal work"))}</div>`;
  }
  const cls = "c-node" + (o.prophet ? " prophet" : "") + (click ? " click" : "");
  return `<div class="c-row${o.first ? " first" : ""}${o.last ? " last" : ""}${o.prophet ? " prophet" : ""}"><canvas class="c-gutter"></canvas>
    <div class="c-holder"><div class="${cls}"${click ? ` data-go="${esc(click)}" data-tip="${esc(clickTip)}"` : ""}>${card}</div></div></div>`;
}

// the timeline gutter: a line through the column and a dot for each node (chain_widgets._Gutter)
function drawGutters(root) {
  const css = getComputedStyle(document.documentElement);
  const color = (v) => css.getPropertyValue(v).trim();
  for (const row of root.querySelectorAll(".c-row")) {
    const c = row.querySelector(".c-gutter");
    const h = row.offsetHeight;
    const dpr = window.devicePixelRatio || 1;
    c.width = 34 * dpr;
    c.height = h * dpr;
    c.style.height = h + "px";
    const g = c.getContext("2d");
    g.scale(dpr, dpr);
    const x = 17, dotY = 26;
    const prophet = row.classList.contains("prophet");
    g.lineWidth = 2;
    g.strokeStyle = color("--border");
    g.beginPath();
    if (!row.classList.contains("first")) { g.moveTo(x, 0); g.lineTo(x, dotY); }
    if (!row.classList.contains("last")) { g.moveTo(x, dotY); g.lineTo(x, h); }
    g.stroke();
    const radius = prophet ? 8 : 6;
    g.beginPath();
    g.arc(x, dotY, radius, 0, 2 * Math.PI);
    g.fillStyle = color(prophet ? "--gold" : "--surface");
    g.fill();
    g.strokeStyle = color(prophet ? "--gold" : "--lapis");
    g.stroke();
  }
}

// the Arabic text with the chain in lighter ink, and the chain in words under it (isnad_page._text_card)
function textCard(d, chain, drawn, P) {
  const splitKnown = !!(chain && chain.links.length && !chain.problem);
  let isnadPart = "", matn = d.text;
  if (splitKnown && chain.raw) {
    const at = d.text.indexOf(chain.raw);
    if (at >= 0) {
      isnadPart = d.text.slice(0, at + chain.raw.length);
      matn = d.text.slice(at + chain.raw.length);
    }
  }
  const s = scriptOf("arabic");
  let html = `<div class="c-card c-text"><div class="caption">${esc(splitKnown ? t("The chain in lighter ink, the text of the hadith (matn) in full ink") : t("The full wording as written; the chain was not separated from the text"))}</div>
    <div class="tb rtl c-arabic" dir="rtl" style="${esc(scriptStyle("arabic"))}"><span style="color:var(--muted)">${esc(isnadPart)}</span><span style="color:${esc(s.color)}">${esc(matn)}</span></div>`;
  const roads = drawn.filter((c) => !c.problem || c.links.length).length;
  for (const w of d.words) {
    const name = { tr: "Turkish", en: "English" }[w.code];
    w.roads.forEach((words, i) => {
      if (!words) return;
      const head = t(name) + (roads > 1 ? " — " + t("Chain {i}", { i: i + 1 }) : "");
      html += `<div class="c-words" data-tip="${esc(t("The chain as the Arabic says it, link by link: each name read from the chain's own wording, with the term that brings it"))}"><b>${esc(head)}:</b> ${esc(words)}</div>`;
    });
  }
  return html + "</div>";
}

// ------------------------------------------------------------------ the page's clicks

function chainsClick(e) {
  if (e.button !== 0) return;
  const go = e.target.closest("[data-go]");
  if (!go || String(window.getSelection() || "")) return;
  const [kind, ...rest] = go.dataset.go.split(":");
  const v = rest.join(":");
  if (kind === "narrator") openNarratorFromChain(Number(v));
  else if (kind === "book") openInBookPlace(Number(v));
  else if (kind === "find") findNarrator(v);
  else if (kind === "tree") openTree("quraysh", v);
}

async function openNarratorFromChain(id) {
  await place(["narrator", id], async () => {
    NARR.id = id;
    show("narrators");
    await selectNarrator(id);
  });
}

async function openInBookPlace(hid) {
  await place(["in_book", hid], () => openInBook(hid));
}

// ------------------------------------------------------------------ From the contents… (ChapterPicker)

async function fromContents() {
  const book = CH.book;
  if (!book) return;
  const d = await invoke("contents_tree", { book, hadith: CH.current && CH.current.book === book ? CH.current.id : null });
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.innerHTML = `<div class="dialog picker-dialog" role="dialog">
      <div class="pd-title">${esc(t("Choose from the contents") + (d ? ` — ${d.title}` : ""))}</div>
      <div class="pd-lead">${esc(t("Choose a chapter, then a hadith of it: the start of its text and how many narrators its chain was read into are shown beside its number."))}</div>
      <div class="pd-split"><div class="pd-tree" tabindex="0"></div>
        <div class="pd-right"><div class="caption pd-where"></div>
          <div class="pd-find"><input type="search" placeholder="${esc(t("Look for a word in this chapter"))}" autocomplete="off" spellcheck="false"></div>
          <div class="pd-list" tabindex="0"></div></div></div>
      <div class="pd-buttons"><button class="primary pd-show">${esc(t("Show chain"))}</button><button class="pd-cancel"><span class="x">✘</span>${esc(t("Cancel"))}</button></div>
    </div>`;
  document.body.appendChild(back);
  const q = (c) => back.querySelector(c);
  const close = () => back.remove();
  const tree = q(".pd-tree"), list = q(".pd-list"), find = q(".pd-find input");
  const arabic = /[؀-ۿ]/;
  const ar = scriptOf("arabic");
  let first = null, chosen = null;
  const add = (nodes, depth) => nodes.map((c) => {
    const label = c.label + (c.leaves ? `   (${c.leaves})` : "");
    const style = arabic.test(c.label) ? ` style="font-family:${esc(ar.family)};font-size:calc(${(ar.size * 0.62).toFixed(2)}pt * var(--scale))"` : "";
    const id = c.leaves ? c.id : "";
    if (c.leaves && first === null) first = c.id;
    if (d.here && c.id === d.here) chosen = c.id;
    const kids = c.children.length ? `<div class="pd-kids">${add(c.children, depth + 1)}</div>` : "";
    return `<div class="pd-node"><div class="pd-item" data-id="${id}" data-node="${c.id}" style="padding-inline-start:${depth * 20 + 25}px">${c.children.length ? '<span class="pd-arrow"></span>' : ""}<span${style}>${esc(label)}</span></div>${kids}</div>`;
  }).join("");
  if (d) tree.innerHTML = add(d.tree, 0);
  let current = null;
  const fill = async (item) => {
    for (const x of tree.querySelectorAll(".pd-item.on")) x.classList.remove("on");
    item.classList.add("on");
    item.scrollIntoView({ block: "nearest" });
    list.innerHTML = "";
    current = null;
    if (!item.dataset.id) {
      q(".pd-where").textContent = t("This heading holds chapters only: choose one of them.");
      return;
    }
    const c = await invoke("contents_chapter", { chapter: Number(item.dataset.id) });
    q(".pd-where").textContent = c.trail.join(" › ");
    list.innerHTML = c.hadith.map((h) => {
      const read = h.links ? t("{n} narrators", { n: h.links }) : t("chain not read");
      const text = `${h.number}   ·   ${read}\n${h.words}`;
      return `<div class="pd-h" data-id="${h.id}" data-hay="${esc(text)}">${esc(text)}</div>`;
    }).join("");
    const firstRow = list.querySelector(".pd-h");
    if (firstRow) select(firstRow);
    filter();
  };
  const select = (row) => {
    for (const x of list.querySelectorAll(".pd-h.on")) x.classList.remove("on");
    row.classList.add("on");
    current = row;
  };
  const filter = () => {
    const words = find.value.split(/\s+/).filter(Boolean);
    for (const row of list.querySelectorAll(".pd-h")) {
      const hay = row.dataset.hay;
      row.hidden = !words.every((w) => arabic.test(w) ? normalizeText(hay).includes(normalizeText(w)) : foldLatinName(hay).includes(foldLatinName(w)));
    }
  };
  const accept = () => {
    if (!current || current.hidden) return;
    const hid = Number(current.dataset.id);
    close();
    showHadith(hid);
  };
  tree.addEventListener("click", (e) => {
    const arrow = e.target.closest(".pd-arrow");
    if (arrow) { arrow.closest(".pd-node").classList.toggle("open"); return; }
    const item = e.target.closest(".pd-item");
    if (item) fill(item);
  });
  list.addEventListener("click", (e) => { const r = e.target.closest(".pd-h"); if (r) select(r); });
  list.addEventListener("dblclick", (e) => { const r = e.target.closest(".pd-h"); if (r) { select(r); accept(); } });
  find.addEventListener("input", filter);
  q(".pd-show").onclick = accept;
  q(".pd-cancel").onclick = close;
  back.addEventListener("keydown", (e) => { if (e.key === "Escape") close(); });
  const target = chosen !== null ? chosen : first;
  if (target !== null) {
    const item = tree.querySelector(`.pd-item[data-node="${target}"]`);
    // scrollToItem: the chapters above it opened
    for (let n = item.parentElement.closest(".pd-node"); n; n = n.parentElement.closest(".pd-node")) {
      if (n !== item.parentElement) n.classList.add("open");
    }
    fill(item);
  }
  else q(".pd-where").textContent = "";
  find.focus();
}

// the text as isnady compares it (core/normalize.py normalize): no vowel marks or tatweel, one form of alif, ya, waw
// and ha, Arabic digits as 0-9; Latin letters case-folded without accents or apostrophes
function normalizeText(text) {
  let out = "";
  for (const ch of text || "") {
    const c = ch.codePointAt(0);
    if ((c >= 0x0610 && c <= 0x061A) || (c >= 0x064B && c <= 0x065F) || (c >= 0x06D6 && c <= 0x06DC) ||
        (c >= 0x06DF && c <= 0x06E8) || (c >= 0x06EA && c <= 0x06ED) || "ـٰ'`‘’ʿʾʼ".includes(ch)) continue;
    const map = { "أ": "ا", "إ": "ا", "آ": "ا", "ٱ": "ا", "ى": "ي", "ئ": "ي",
      "ی": "ي", "ؤ": "و", "ة": "ه", "ک": "ك", "ı": "i" };
    if (map[ch]) { out += map[ch]; continue; }
    if (c >= 0x0660 && c <= 0x0669) { out += String(c - 0x0660); continue; }
    if (c >= 0x06F0 && c <= 0x06F9) { out += String(c - 0x06F0); continue; }
    if (c < 0x0600 || c > 0x06FF) {
      out += ch.normalize("NFKD").replace(/\p{M}/gu, "").toLowerCase().replace(/ı/g, "i");
      continue;
    }
    out += ch;
  }
  return out;
}

// a Latin spelling reduced so that English and Turkish spellings of one name compare closely (narrators.fold_latin)
function foldLatinName(text) {
  let w = (text || "").replace(/I/g, "ı").replace(/İ/g, "i").toLowerCase();
  for (const [a, b] of [["ş", "sh"], ["ı", "i"], ["ç", "ch"], ["c", "j"], ["v", "w"], ["ğ", ""], ["q", "k"]]) w = w.split(a).join(b);
  w = w.normalize("NFKD").replace(/\p{M}/gu, "");
  const words = [];
  for (let word of w.split(/\s+/)) {
    word = word.replace(/^(al|el|an|ar|as|at|az|ad|ash|adh|en|er|es|et|ez|ed)-/, "").replace(/[^a-z]/g, "");
    if (["ibn", "bin", "bn", "b", "bint", "bt", "ibni", "ibnu", "bini", "binti"].includes(word)) continue;
    words.push(word.replace(/e/g, "a").replace(/o/g, "u").replace(/y/g, "i"));
  }
  return words.filter(Boolean).join(" ");
}
