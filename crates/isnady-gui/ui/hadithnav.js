"use strict";
// Going through a book by its hadith numbers (gui/hadith_nav.py, NAV3): the number field, the last number beside it,
// the first / previous / next / last buttons and where the hadith stands among all of the book's. One widget, on
// Isnad Chains and on Books.
//
// [Hadith no] [ 93 ] / 7563  [Show]  ⏮ ‹ › ⏭  hadith 93 of 7,583 in this book. Wraps onto a second line when narrow.
// `activated` is called when a number is to be shown (Enter, Show, a step); the page reads number().

function hadithNavigator(host, goText, activated) {
  const nav = { book: null, range: null, field: null, popup: null, seq: 0 };
  host.classList.add("hnav");
  host.innerHTML = `<span class="flabel">${esc(t("Hadith no"))}</span>
    <input type="text" class="numfield" autocomplete="off" spellcheck="false">
    <span class="flabel hn-last"></span>
    <button class="primary hn-go">${esc(goText)}</button>
    <button class="quiet hn-first" data-tip="${esc(t("The first hadith of the book"))}">⏮</button>
    <button class="quiet hn-prev" data-tip="${esc(t("The hadith before (↓ or the mouse wheel in the number)"))}">‹  ${esc(t("Previous"))}</button>
    <button class="quiet hn-next" data-tip="${esc(t("The hadith after (↑ or the mouse wheel in the number)"))}">${esc(t("Next"))}  ›</button>
    <button class="quiet hn-lastb" data-tip="${esc(t("The last hadith of the book"))}">⏭</button>
    <span class="caption hn-place"></span>`;
  const q = (c) => host.querySelector(c);
  const field = q(".numfield");
  nav.field = field;
  const buttons = [q(".hn-first"), q(".hn-prev"), q(".hn-next"), q(".hn-lastb"), q(".hn-go")];
  field.placeholder = t("Hadith number");

  nav.number = () => field.value.trim();

  // ---- the numbers that begin with what is typed (QCompleter, UnfilteredPopupCompletion: seven at a time)
  const closePopup = () => {
    if (nav.popup) nav.popup.remove();
    nav.popup = null;
  };
  const popupRows = () => (nav.popup ? [...nav.popup.querySelectorAll(".mi")] : []);
  const choose = (text) => {
    closePopup();
    field.value = text;
    go();
  };
  async function typed() {
    const seq = ++nav.seq;
    if (!nav.book) return;
    const list = await invoke("numbers_starting", { book: nav.book, prefix: field.value });
    if (seq !== nav.seq) return;
    closePopup();
    if (!list.length || document.activeElement !== field) return;
    const r = field.getBoundingClientRect();
    const el = document.createElement("div");
    el.className = "menu combo-list completer";
    el.style.width = r.width + "px";
    el.style.minWidth = "0";
    el.innerHTML = list.map((n) => `<div class="mi"><span class="lb">${esc(n)}</span></div>`).join("");
    el.onmousedown = (e) => e.preventDefault();
    el.onclick = (e) => {
      const row = e.target.closest(".mi");
      if (row) choose(row.textContent);
    };
    el.onmouseover = (e) => {
      const row = e.target.closest(".mi");
      if (!row) return;
      for (const x of el.querySelectorAll(".mi.on")) x.classList.remove("on");
      row.classList.add("on");
    };
    document.body.appendChild(el);
    const rows = el.querySelectorAll(".mi");
    if (rows.length > 7) {
      el.style.maxHeight = (rows[7].offsetTop - rows[0].offsetTop + 2) + "px";
      el.style.overflowY = "auto";
    }
    el.style.left = r.left + "px";
    el.style.top = r.bottom + "px";
    nav.popup = el;
  }

  // ---- moving
  async function step(delta) {
    if (!nav.book) return;
    const number = await invoke("step_number", { book: nav.book, number: nav.number(), delta });
    if (number) {
      field.value = number;
      go();
    }
  }
  nav.step = step;

  function jump(end) {
    if (nav.range) {
      field.value = nav.range[end];
      go();
    }
  }

  function go() {
    closePopup();
    showPlace();
    if (nav.number()) activated();
  }

  async function showPlace() {
    const rng = nav.range;
    const place = q(".hn-place");
    if (!rng) {
      place.textContent = "";
      return;
    }
    const at = nav.number() ? await invoke("number_place", { book: nav.book, number: nav.number() }) : null;
    if (rng !== nav.range) return;
    if (!at) {
      place.textContent = t("{n} hadith · numbers {first} – {last}", { n: rng.count.toLocaleString("en-US"), first: rng.first, last: rng.last });
      q(".hn-prev").disabled = false;
      q(".hn-next").disabled = false;
      return;
    }
    const [n, count] = at;
    place.textContent = t("hadith {place} of {count} in this book", { place: n.toLocaleString("en-US"), count: count.toLocaleString("en-US") });
    q(".hn-first").disabled = n <= 1;
    q(".hn-prev").disabled = n <= 1;
    q(".hn-next").disabled = n >= count;
    q(".hn-lastb").disabled = n >= count;
  }

  // ---- the book
  nav.setBook = async (book) => {
    nav.book = book;
    const rng = book ? await invoke("book_numbers", { book }) : null;
    if (book !== nav.book) return;
    nav.range = rng;
    for (const b of buttons) b.disabled = !rng;
    if (!rng) {
      field.placeholder = t("Hadith number");
      delete field.dataset.tip;
      q(".hn-last").textContent = "";
      delete q(".hn-last").dataset.tip;
      q(".hn-place").textContent = "";
      delete q(".hn-place").dataset.tip;
      return;
    }
    field.placeholder = `${rng.first} – ${rng.last}`;
    q(".hn-last").textContent = "/ " + rng.last;
    const why = t("The numbers are the book's own, as its edition gives them, not a count: some numbers are missing, some are split (402.2) and some books begin at 0, so the last number and the number of hadith differ.");
    q(".hn-last").dataset.tip = t("The last number of this book") + "\n" + why;
    field.dataset.tip = t("This book's numbers run from {first} to {last} ({n} hadith). The arrow keys and the mouse wheel go to the next or the previous one; typing shows the numbers that begin so.",
      { first: rng.first, last: rng.last, n: rng.count.toLocaleString("en-US") }) + "\n" + why;
    q(".hn-place").dataset.tip = why;
    await showPlace();
  };

  // show a number without going to it (the page shows that hadith already)
  nav.setNumber = (number) => {
    field.value = number || "";
    showPlace();
  };

  field.addEventListener("input", typed);
  field.addEventListener("blur", () => setTimeout(closePopup, 0));
  field.addEventListener("keydown", (e) => {
    const rows = popupRows();
    if (e.key === "ArrowUp" || e.key === "ArrowDown") {
      e.preventDefault();
      if (!rows.length) {
        step(e.key === "ArrowUp" ? 1 : -1);
        return;
      }
      const at = rows.findIndex((r) => r.classList.contains("on"));
      const next = e.key === "ArrowDown" ? Math.min(at + 1, rows.length - 1) : Math.max(at - 1, 0);
      for (const r of rows) r.classList.remove("on");
      rows[next].classList.add("on");
      rows[next].scrollIntoView({ block: "nearest" });
    } else if (e.key === "Enter") {
      e.preventDefault();
      const on = rows.find((r) => r.classList.contains("on"));
      if (on) choose(on.textContent); else go();
    } else if (e.key === "Escape" && rows.length) {
      e.preventDefault();
      closePopup();
    }
  });
  field.addEventListener("wheel", (e) => {
    e.preventDefault();
    step(e.deltaY < 0 ? 1 : -1);
  }, { passive: false });
  q(".hn-go").onclick = go;
  q(".hn-first").onclick = () => jump("first");
  q(".hn-lastb").onclick = () => jump("last");
  q(".hn-prev").onclick = () => step(-1);
  q(".hn-next").onclick = () => step(1);
  for (const b of buttons) b.disabled = true;
  return nav;
}
