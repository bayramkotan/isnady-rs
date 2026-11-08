"use strict";
// Help → Log (gui/log_viewer.py, LOG1): isnady's log in a window of its own — the file (isnady.log, its older parts,
// the crash reports), followed live, only errors or a text, larger or smaller, copied, its folder opened, what is
// older than a day removed.

const LV = { timer: null, size: -1, font: 10 };

async function openLog() {
  if ($("lv-back")) return;
  const back = document.createElement("div");
  back.className = "dialog-back";
  back.id = "lv-back";
  const week = new Date(Date.now() - 7 * 864e5).toISOString().slice(0, 10);
  back.innerHTML = `<div class="dialog lv" role="dialog"><h2>${esc(t("isnady log"))}</h2>
    <div class="lv-top"><span>${esc(t("File"))}</span><select id="lv-file"></select>
      <select id="lv-level"><option value="">${esc(t("Everything"))}</option><option value="warning">${esc(t("Warnings and errors"))}</option><option value="error">${esc(t("Errors only"))}</option></select>
      <input type="search" id="lv-find" placeholder="${esc(t("Only lines holding…"))}">
      <label class="check" data-tip="${esc(t("Read the log again every two seconds and keep the newest line in view"))}"><input type="checkbox" id="lv-follow" checked><span class="box"></span>${esc(t("Follow"))}</label>
      <button class="quiet" id="lv-small">A−</button><button class="quiet" id="lv-large">A+</button></div>
    <textarea id="lv-text" readonly wrap="off" spellcheck="false"></textarea>
    <div class="caption" id="lv-info"></div>
    <div class="lv-bottom"><button class="quiet" id="lv-copy">${esc(t("Copy all"))}</button><button class="quiet" id="lv-folder">${esc(t("Open the folder"))}</button>
      <span class="grow"></span><span>${esc(t("Before"))}</span><input type="date" id="lv-day" value="${week}">
      <button class="quiet" id="lv-older">${esc(t("Delete what is older"))}</button><button class="quiet" id="lv-clear">${esc(t("Clear the log"))}</button><button class="quiet" id="lv-close">${esc(t("Close"))}</button></div></div>`;
  document.body.appendChild(back);
  const close = () => { clearInterval(LV.timer); back.remove(); };
  $("lv-close").onclick = close;
  back.addEventListener("keydown", (e) => { if (e.key === "Escape") close(); });
  $("lv-file").onchange = () => lvLoad(true);
  $("lv-level").onchange = () => lvLoad(true);
  $("lv-find").oninput = () => lvLoad(true);
  $("lv-small").onclick = () => lvZoom(-1);
  $("lv-large").onclick = () => lvZoom(1);
  $("lv-copy").onclick = async () => {
    const head = await invoke("log_change", { action: "environment", day: null });
    await navigator.clipboard.writeText(head + "\n" + $("lv-text").value).catch(() => {});
    $("lv-info").textContent = t("Copied, with the version and the system at its head.");
  };
  $("lv-folder").onclick = () => invoke("log_change", { action: "folder", day: null });
  $("lv-older").onclick = () => {
    const day = $("lv-day").value;
    confirmDialog(t("isnady log"), t("Remove what was logged before {day}?", { day }), async () => {
      const n = await invoke("log_change", { action: "before", day });
      await lvFiles();
      $("lv-info").textContent = t("{n} line(s) before {day} removed.", { n: Number(n).toLocaleString("en"), day });
    });
  };
  $("lv-clear").onclick = () => confirmDialog(t("isnady log"), t("Empty the log and remove the crash reports?"), async () => {
    await invoke("log_change", { action: "clear", day: null });
    await lvFiles();
  });
  LV.timer = setInterval(() => { if ($("lv-follow").checked) lvLoad(false); }, 2000);
  lvZoom(0);
  await lvFiles();
}

async function lvFiles() {
  const sel = $("lv-file");
  const current = sel.value;
  const files = await invoke("log_files");
  sel.innerHTML = files.map((f) => `<option value="${esc(f.path)}">${esc(`${f.name}  ·  ${t(f.crash ? "crash report" : "log")}  ·  ${f.stamp}  ·  ${f.size}`)}</option>`).join("");
  if (files.some((f) => f.path === current)) sel.value = current;
  await lvLoad(true);
}

async function lvLoad(force) {
  const box = $("lv-text");
  if (!box) return;
  const d = await invoke("log_read", { path: $("lv-file").value, level: $("lv-level").value, text: $("lv-find").value.trim() });
  if (!force && d.size === LV.size) return;
  LV.size = d.size;
  const atEnd = box.scrollTop >= box.scrollHeight - box.clientHeight - 4;
  box.value = d.body;
  if ($("lv-follow").checked || atEnd || force) {
    const end = () => { box.scrollTop = box.scrollHeight; box.scrollLeft = 0; };
    end();
    requestAnimationFrame(end);
  }
  const lines = d.body ? d.body.split("\n").length : 0;
  $("lv-info").textContent = t("{lines} lines · {path}", { lines: lines.toLocaleString("en"), path: $("lv-file").value });
}

function lvZoom(step) {
  LV.font = Math.max(7, Math.min(24, LV.font + step));
  $("lv-text").style.fontSize = LV.font + "pt";
}

// Tools → Clear the Cache (MainWindow._clear_cache)
async function clearCache() {
  const d = await invoke("cache_clear");
  infoDialog(t("Clear the Cache"), `<p>${esc(t("{n} kept file(s) deleted ({size:,.0f} KB). isnady counts them again when a page next needs them; this is needed only if something looks out of date.", { n: d.n, size: d.size }))}</p>`);
}

// Tools → Create Desktop Shortcut… (MainWindow._create_shortcut)
async function createShortcut() {
  try {
    const d = await invoke("shortcut_create");
    const where = { desktop: t("On the desktop"), menu: t("In the applications menu") };
    const lines = d.made.map(([w, p]) => `<li><b>${esc(where[w] || w)}</b>: <code>${esc(p)}</code></li>`).join("");
    infoDialog(t("Create Desktop Shortcut"), `<p>${esc(t("isnady now has a shortcut, with its own icon:"))}</p><ul>${lines}</ul>
      <p>${esc(t("It starts {how}:", { how: d.how }))}<br><code>${esc(d.program)}</code></p>`);
  } catch (e) {
    infoDialog(t("Create Desktop Shortcut"), `<p>${esc(t("The shortcut could not be created."))}</p><p>${esc(String(e))}</p>`);
  }
}
