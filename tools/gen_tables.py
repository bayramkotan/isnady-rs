"""The tables isnady-rs reads, written from the Python isnady itself — so the two use the same word lists, the same
forms and the same patterns, never a copy kept by hand.

    python tools/gen_tables.py                 # the Python isnady installed (pip install isnady), or ../isnady/src
    python tools/gen_tables.py --check         # exit 1 when data/tables.json or data/i18n/*.json is not what it writes

It also writes what the frame of the window needs (data/window.json: the sections, themes, languages, shortcuts,
search tips) and the interface catalogs (crates/isnady-core/data/i18n/<code>.json) of every language the Python isnady
translates, so the isnady-rs window speaks the same Turkish and Arabic as the Python one.

The Parity workflow runs --check against the Python isnady of the same version: a word added to the names list there
and not here stops the release.
"""

import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "crates", "isnady-core", "data", "tables.json")


def _import():
    try:
        import isnady  # noqa: F401
    except ImportError:
        sys.path.insert(0, os.path.join(HERE, "..", "..", "isnady", "src"))


def tables() -> dict:
    _import()
    from isnady.core import companions, i18n, isnad, name_parts, names, narrators, rijal, shia_rijal, traditions

    words, compounds, stops = names._tables()
    tr = i18n.catalog("tr")
    templates = dict(isnad.CHAIN_TEMPLATES)
    said = list(templates.values()) + [isnad.CHAIN_END, isnad.CHAIN_RAISED, isnad.CHAIN_CUT]
    return {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "parser_id": isnad.PARSER_ID,
        "matcher_id": narrators.MATCHER_ID,
        "death_rule": rijal.DEATH_RULE,
        "names": {
            "words": {k: list(v) for k, v in sorted(words.items())},
            "compounds": {" ".join(k): list(v) for k, v in sorted(compounds.items())},
            "stops": sorted(stops),
        },
        "companions": {"forms": sorted(companions.FORMS), "trim": companions._TRIM_RE.pattern},
        "traditions": {
            "group": list(traditions.GROUP_FORMS),
            "previous": list(traditions.PREVIOUS_FORMS),
            "vague": list(traditions.VAGUE_FORMS),
            "idda": traditions.KULAYNI_IDDA,
            "masum": {m.key: m.name for m in traditions.MASUMIN},
        },
        "isnad": {
            "terms": {k: [v[0], v[1]] for k, v in isnad.TERM_LABELS.items()},
            "term_why": {k: v[2] for k, v in isnad.TERM_LABELS.items()},
            "direct": isnad.DIRECT,
            "note": isnad.NOTE,
            "continued": isnad.CONTINUED,
            "no_chain": isnad.NO_CHAIN,
            "no_term": isnad._NO_TERM,
            "templates": templates,
            "end": isnad.CHAIN_END,
            "raised": isnad.CHAIN_RAISED,
            "cut": isnad.CHAIN_CUT,
            "borrowed": isnad.BORROWED,
            "languages": isnad.CHAIN_LANGUAGES,
            "tr_possessive": list(isnad._TR_POSSESSIVE),
            "masum_tr": isnad._MASUM_TR,
            "honour": isnad._HONOUR_RE.pattern,
            "kin": {k: list(v) for k, v in isnad._KIN.items()},
            "kin_alone": {k: list(v) for k, v in isnad._KIN_ALONE.items()},
            "kin_third": isnad._KIN_THIRD,
            "whose": sorted(isnad._WHOSE),
            "tr": {k: tr[k] for k in said if k in tr},
        },
        "rijal": {
            "note_start": rijal._NOTE_START.pattern,
            "note_words": rijal._NOTE_WORDS.pattern,
            "shia_ranks": {str(k): v[1] for k, v in shia_rijal.RANKS.items()},
            "ranks": {str(k): v[1] for k, v in rijal.RANK_LABELS.items()},
            "tabaqat": {str(k): v for k, v in rijal.TABAQA_LABELS.items()},
        },
        "narrators": narrators_tables(),
        "scholars": scholars_tables(),
        "name_parts": {
            "places": sorted(name_parts.PLACES),
            "not_names": sorted(name_parts._NOT_NAMES),
            "laqab": name_parts._LAQAB_PHRASE.pattern,
            "known": name_parts._KNOWN_PHRASE.pattern,
        },
    }


def scholars_tables() -> dict:
    """The scholars of hadith (core.scholars): who they are, the short Turkish names, what each role means."""
    from isnady.core import scholars

    return {"list": scholars.SCHOLARS, "short_tr": scholars.SHORT_TR,
            "role_help": {k: list(v) for k, v in scholars.ROLE_HELP.items()}}


def narrators_tables() -> dict:
    """What `iy narrator` needs: Ibn Hajar's book marks, al-Najashi's schools, and the famous ones (core.famous):
    each person, the groups, and the spellings people type for them, already folded as the Python isnady folds them."""
    from isnady.core import famous, rijal, shia_rijal

    return {
        "book_marks": rijal.BOOK_MARKS,
        "collection_marks": {k: sorted(v) for k, v in rijal.COLLECTION_MARKS.items()},
        "madhhabs": shia_rijal.MADHHAB_LABELS,
        "famous": {
            "people": [[p[0], p[1], p[2], p[3], p[4], tradition] for p, tradition in famous._all()],
            "groups": [[g["id"], g["title"], g["tradition"], [p[0] for p in g["people"]]] for g in famous.GROUPS],
            "keys": [list(k) for k in famous._keys()],
            "honour": famous._HONOUR_RE.pattern,
            "honour_ar": famous._HONOUR_AR_RE.pattern,
            "ibn": sorted(famous._IBN),
            "too_short": sorted(famous._TOO_SHORT),
        },
    }


def text() -> str:
    return json.dumps(tables(), ensure_ascii=False, indent=1, sort_keys=True) + "\n"


I18N = os.path.join(HERE, "..", "crates", "isnady-core", "data", "i18n")


def catalogs() -> dict:
    """The interface in the languages the Python isnady translates: {code: text of data/i18n/<code>.json}."""
    _import()
    from isnady.core import i18n

    out = {}
    for code in sorted(i18n.LANGUAGES):
        cat = i18n.catalog(code)
        if code != "en" and cat:
            out[code] = json.dumps(dict(sorted(cat.items())), ensure_ascii=False, indent=0) + "\n"
    return out


BOOKS = os.path.join(HERE, "..", "crates", "isnady-core", "data", "books.json")


def books() -> str:
    """What the reader of the books needs (core.works, core.titles, core.chapters, core.sahifa): the catalog shipped
    with isnady (book key → title, Arabic title, full id), the short names of the library's works, which road each
    written source walks, the tables of contents isnady keeps (Musnad Ahmad, al-Darimi) and the book and chapter
    titles in Turkish and Arabic."""
    _import()
    import json as _json
    from importlib import resources

    from isnady.core import chapters, sahifa, titles, works
    from isnady.core.repositories import bundled

    catalog = {e["book"]: {"id": e.get("id", ""), "title": e.get("title", ""), "title_ar": e.get("title_ar", "")}
               for e in bundled().get("isnady", {}).get("entries", []) if e.get("book")}
    tables = {}
    folder = resources.files("isnady.assets").joinpath("chapters")
    for f in sorted(folder.iterdir(), key=lambda x: x.name):
        if f.name.endswith(".json"):
            key = f.name[:-5]
            data = chapters.table(key)
            if data:
                tables[key] = {"version": data.get("version", 1), "parts": data["parts"], "chapters": data["chapters"]}
    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "catalog": dict(sorted(catalog.items())),
        "aliases": [list(x) for x in works.ALIASES.items()],
        "kind_labels": works.KIND_LABELS,
        "sahifa_roads": {x["book"]: x["road"] for x in sahifa.SOURCES},
        "chapters": tables,
        "titles": {code: {"chapters": titles._table(code)["chapters"], "books": titles._table(code)["books"]}
                   for code in ("ar", "tr")},
        "edition_language": titles.EDITION_LANGUAGE,
    }
    return _json.dumps(out, ensure_ascii=False, indent=0, sort_keys=True) + "\n"


WINDOW = os.path.join(HERE, "..", "crates", "isnady-core", "data", "window.json")


class _Source:
    """The plain values of a module of the Python window, read from its source (ast) — no Qt is imported, so the
    Parity workflow needs no display libraries. N_("text") is its text; a call like Tokens(...) is its keywords."""

    def __init__(self, module: str) -> None:
        import ast
        from importlib.util import find_spec

        self.ast = ast
        self.file = find_spec(module).origin
        with open(self.file, encoding="utf-8") as f:
            self.tree = ast.parse(f.read())
        self.names = {}
        for node in self.tree.body:
            if isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name):
                        self.names[target.id] = node.value
                    elif isinstance(target, ast.Tuple) and isinstance(node.value, ast.Tuple):
                        for t, v in zip(target.elts, node.value.elts):
                            self.names[t.id] = v

    def value(self, node):
        ast = self.ast
        if isinstance(node, str):
            node = self.names[node]
        if isinstance(node, ast.Constant):
            return node.value
        if isinstance(node, (ast.Tuple, ast.List)):
            return [self.value(e) for e in node.elts]
        if isinstance(node, ast.Dict):
            return {self.value(k): self.value(v) for k, v in zip(node.keys, node.values)}
        if isinstance(node, ast.Name):
            return self.value(self.names[node.id])
        if isinstance(node, ast.BinOp) and isinstance(node.op, ast.Add):
            return self.value(node.left) + self.value(node.right)
        if isinstance(node, ast.Call):
            if getattr(node.func, "id", "") == "N_":
                return self.value(node.args[0])
            return {k.arg: self.value(k.value) for k in node.keywords}
        raise SystemExit(f"{self.file}: cannot read {ast.dump(node)[:80]}")

    def tr_texts(self, function: str) -> list:
        """The texts a function passes to tr(), in order."""
        ast = self.ast
        for node in ast.walk(self.tree):
            if isinstance(node, ast.FunctionDef) and node.name == function:
                return [c.args[0].value for c in ast.walk(node)
                        if isinstance(c, ast.Call) and getattr(c.func, "id", "") == "tr" and c.args
                        and isinstance(c.args[0], ast.Constant) and isinstance(c.args[0].value, str)]
        raise SystemExit(f"no {function} in {self.file}")


def window() -> str:
    """What the frame of the window needs (gui/main_window.py, gui/theme.py, gui/dialogs.py, core/i18n.py): the
    sections and their order and signs, the themes, the reading text sizes, the interface languages with their names
    and how much of each is translated, the keyboard shortcuts and the search tips."""
    _import()
    from isnady import config
    from isnady.core import catalog, genealogy, i18n, learn, rijal, scholars, shia_rijal, traditions

    main_window = _Source("isnady.gui.main_window")
    people_results = _Source("isnady.gui.people_results")
    theme = _Source("isnady.gui.theme")
    dialogs = _Source("isnady.gui.dialogs")
    themes = theme.value("THEMES")
    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "sections": [[k, label] for k, label, _tip in main_window.value("SECTIONS")],
        "sidebar": list(config.SIDEBAR),
        "signs": theme.value("NAV_SIGNS"),
        "nav_signs": {k: bool(v[2].get("nav_signs", False)) for k, v in themes.items()},
        "themes": [[k, v[0]] for k, v in themes.items()],
        "theme_texts": {k: v[1] for k, v in themes.items()},
        # every token of every theme (colours, typefaces, how round, how roomy): the window draws them as Python does
        "tokens": {k: v[2] for k, v in themes.items()},
        "rank_colors": theme.value("RANK_COLORS"),
        # the reader's typeface of each script by default (config.SCRIPTS): family, size in points, line height %
        "scripts": {k: {"family": v[2], "size": v[3], "line_height": v[4]} for k, v in config.SCRIPTS.items()},
        "script_of_language": dict(config.SCRIPT_OF_LANGUAGE),
        # the compiler of each book, drawn at the head of its chains (isnad.compiler_of)
        "compilers": {key: {"arabic": sc.get("known_ar") or sc.get("arabic", ""), "tr": sc["tr"].split(" (")[0],
                            "en": sc["name"]}
                      for sc in scholars.SCHOLARS for key in sc.get("compiler_of", [])},
        # whom a chain reaches, in Arabic, and whether his life is in the tree of Quraysh (chain_widgets.end_node)
        "masum_ar": {k: m.name_ar for k, m in traditions.MASUM.items()},
        "quraysh": sorted(k for k in list(traditions.MASUM) + ["muhammad"] if k in genealogy.written("quraysh").nodes),
        "catalog_languages": dict(catalog.LANGUAGE_NAMES),
        # the glossary's words for a grade's group, on the hover of a grade (learn.for_group), in each of its languages
        "grade_terms": {lang: {str(g): learn.for_group(g, lang) for g in learn.GROUP_TERM} for lang in learn.LANGUAGES},
        # the Narrators page (gui/narrators_page.py): the glossary's words on the hover of a verdict, and the written
        # trees a written life has its man in (genealogy.LIFE_NODES: "his place in the tree of Quraysh")
        "narrator_terms": {lang: {k: learn.short(k, lang) for k in ("rank", "thiqa", "mamduh", "muwaththaq",
                                                                     "daif_narrator", "imami")}
                           for lang in learn.LANGUAGES},
        "life_nodes": {k: v[0] for k, v in genealogy.LIFE_NODES.items()},
        "text_scale": [theme.value("TEXT_SCALE_MIN"), theme.value("TEXT_SCALE_MAX"), theme.value("TEXT_SCALE_STEP")],
        "languages": [[code, own, english, rtl, round(i18n.coverage(code), 4)]
                      for code, (own, english, rtl) in i18n.LANGUAGES.items()],
        "names": i18n.NAMES,
        "learn_languages": list(learn.LANGUAGES),
        "shortcuts": dialogs.value("SHORTCUTS"),
        "search_tips": dialogs.tr_texts("search_tips"),
        # Search → Narrators and Scholars (gui/people_results.py): the ranks in Arabic and English, the Shia
        # categories with their meanings and colours, the tabaqat, the schools, what a scholar's role means
        "people": {
            "ranks": {str(k): list(v) for k, v in rijal.RANK_LABELS.items()},
            "shia_ranks": {str(k): list(v) for k, v in shia_rijal.RANKS.items()},
            "shia_color": {str(k): v for k, v in people_results.value("SHIA_COLOR").items()},
            "tabaqat": {str(k): v for k, v in rijal.TABAQA_LABELS.items()},
            "madhhabs": shia_rijal.MADHHAB_LABELS,
            "tradition": people_results.value("TRADITION"),
            "role_help": {k: list(v) for k, v in scholars.ROLE_HELP.items()},
            "tips": {k: people_results.value(k) for k in ("CLOSE_TIP", "WEAK_TIP", "UNREAD_TIP")},
        },
        "repo_url": dialogs.value("REPO_URL"),
        "issues_url": dialogs.value("ISSUES_URL"),
        "pypi_url": dialogs.value("PYPI_URL"),
    }
    return json.dumps(out, ensure_ascii=False, indent=1) + "\n"


LIVES = os.path.join(HERE, "..", "crates", "isnady-core", "data", "lives.json")


def lives() -> str:
    """The lives written for isnady (core.lives, assets/lives/*.json, in their order), what their kinds, facts and
    sources are called, which entry of a classical work each is summarised from (core.source_entries,
    assets/sources/lives.json) and al-Dhahabi's huffaz by their Taqrib key (core.huffaz): what the life card and
    `iy life` need."""
    _import()
    import json as _json
    from importlib import resources

    from isnady.core import huffaz, lives as core_lives, source_entries

    table = _json.loads(resources.files("isnady.assets").joinpath("sources", "lives.json").read_text(encoding="utf-8"))
    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "lives": list(core_lives.written()),
        "kinds": core_lives.KINDS,
        "facts": core_lives.FACTS,
        "sources": core_lives.SOURCES,
        "entries": table.get("works", {}),
        "works": {k: {"title": v["title"], "size_mb": v["size_mb"]} for k, v in source_entries.WORKS.items()},
        "huffaz_work": huffaz.WORK,
        "huffaz": [[h["taqrib"], h["n"], h["tabaqa"]] for h in huffaz.index() if h["taqrib"]],
    }
    return _json.dumps(out, ensure_ascii=False, indent=0, sort_keys=True) + "\n"


HUFFAZ = os.path.join(HERE, "..", "crates", "isnady-core", "data", "huffaz.json")


def huffaz_index() -> str:
    """al-Dhahabi's Tadhkirat al-Huffaz as isnady indexes it (core.huffaz: assets/scholars/huffaz.json), the sigla he
    puts before a name and how many he says a generation holds: what the Hadith Scholars page and `iy scholar --huffaz`
    need."""
    _import()
    import json as _json

    from isnady.core import huffaz

    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "work": huffaz.WORK,
        "rumuz": [[k, v] for k, v in huffaz.RUMUZ.items()],
        "stated": [[k, v] for k, v in huffaz.STATED.items()],
        "huffaz": huffaz.index(),
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


SAHABA = os.path.join(HERE, "..", "crates", "isnady-core", "data", "sahaba.json")


def sahaba() -> str:
    """Companions and the Household (core.sahaba): the groups as written for isnady, the chain ends that name no one
    Imam, the boxes of the tree of Quraysh the Shia group is drawn from (names and notes, core.genealogy) and the box of
    each written life in that tree (genealogy.LIFE_NODES): what the Companions page and `iy group` need."""
    _import()
    import json as _json

    from isnady.core import genealogy, sahaba as core_sahaba

    tree = genealogy.written("quraysh", "en")
    wanted = {n for g in core_sahaba.GROUPS for n, _y in g.get("nodes", [])}
    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "groups": core_sahaba.GROUPS,
        "shared_kunyas": list(core_sahaba.SHARED_KUNYAS),
        "quraysh": {k: {"names": tree.nodes[k].names, "note": tree.nodes[k].note}
                    for k in sorted(wanted) if k in tree.nodes},
        "life_nodes": {k: list(v) for k, v in genealogy.LIFE_NODES.items()},
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


SIRA = os.path.join(HERE, "..", "crates", "isnady-core", "data", "sira.json")


def sira() -> str:
    """The Sira as a timeline (core.sira): its periods and events as written for isnady."""
    _import()
    import json as _json

    from isnady.core import sira as core_sira

    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "work": core_sira.SIRA_WORK,
        "periods": [[k, v] for k, v in core_sira.PERIODS.items()],
        "events": core_sira.EVENTS,
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


GENEALOGY = os.path.join(HERE, "..", "crates", "isnady-core", "data", "genealogy.json")


def genealogy() -> str:
    """The genealogies (core.genealogy): the two written trees (assets/genealogy/trees.json), the words that name each
    prophet in a hadith, and the box of each written life in a tree."""
    _import()
    import json as _json

    from isnady.core import genealogy as g

    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "sources": g.sources(),
        "trees": g._data()["trees"],
        "written": list(g.WRITTEN),
        "prophet_words": [[k, v[0], list(v[1])] for k, v in g.PROPHET_WORDS.items()],
        "honoured": list(g._HONOURED),
        "road_b_only_own": sorted(g._ROAD_B_ONLY_OWN),
        "not_him": g._NOT_HIM.pattern,
        "name_head": list(g._NAME_HEAD),
        "life_nodes": [[k, v[0], v[1]] for k, v in g.LIFE_NODES.items()],
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


LEARN = os.path.join(HERE, "..", "crates", "isnady-core", "data", "learn.json")


def learn_data() -> str:
    """Learn (core.learn, core.learn_history): the glossary's terms (assets/learn/terms.json), the articles of History
    and method (assets/learn/articles.json) and the wordings the live numbers look for."""
    _import()
    import json as _json
    from importlib import resources

    from isnady.core import learn, learn_history as H

    assets = resources.files("isnady") / "assets" / "learn"
    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "languages": [[k, v] for k, v in learn.LANGUAGES.items()],
        "group_term": [[k, v] for k, v in learn.GROUP_TERM.items()],
        "terms": _json.loads((assets / "terms.json").read_text(encoding="utf-8")),
        "articles": _json.loads((assets / "articles.json").read_text(encoding="utf-8")),
        "writing_forms": [list(x) for x in H.WRITING_FORMS],
        "writing_in_text": [list(x) for x in H.WRITING_IN_TEXT],
        "akhbarana_books": list(H.AKHBARANA_BOOKS),
        "hammam_key": H.HAMMAM_KEY,
        "mutawatir_forms": H.MUTAWATIR_FORMS,
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


SAHIFA = os.path.join(HERE, "..", "crates", "isnady-core", "data", "sahifa.json")


def sahifa() -> str:
    """The written sources (core.sahifa): every source with its texts and the words that find its hadith."""
    _import()
    import json as _json

    from isnady.core import sahifa as s

    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "hammam_name": s.HAMMAM_NAME,
        "schema": s.SCHEMA,
        "sources": s.SOURCES,
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


SETTINGS = os.path.join(HERE, "..", "crates", "isnady-core", "data", "settings.json")


def settings() -> str:
    """The settings (isnady/config.py): every key with its default in Python's order, the scripts, the colour tokens,
    the sidebar's order and the themes a setting may name."""
    _import()
    import ast
    import inspect
    import json as _json

    from isnady import config

    source = inspect.getsource(config.validate)
    themes = None
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Assign) and getattr(node.targets[0], "id", "") == "allowed":
            themes = [e.value for e in node.value.elts]
    out = {
        "about": "Written by tools/gen_tables.py from the Python isnady; do not edit by hand.",
        "defaults": [[k, v] for k, v in config.DEFAULTS.items()],
        "scripts": [[k, list(v)] for k, v in config.SCRIPTS.items()],
        "script_of_language": config.SCRIPT_OF_LANGUAGE,
        "color_tokens": [[k, v] for k, v in config.COLOR_TOKENS.items()],
        "sidebar": list(config.SIDEBAR),
        "themes": themes,
    }
    return _json.dumps(out, ensure_ascii=False, indent=0) + "\n"


def outputs() -> dict:
    files = {OUT: text(), BOOKS: books(), WINDOW: window(), LIVES: lives(), HUFFAZ: huffaz_index(),
             SAHABA: sahaba(), SIRA: sira(),
             GENEALOGY: genealogy(), LEARN: learn_data(), SAHIFA: sahifa(), SETTINGS: settings()}
    for code, body in catalogs().items():
        files[os.path.join(I18N, f"{code}.json")] = body
    return files


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="only say whether the tables are up to date")
    a = ap.parse_args()
    files = outputs()
    if a.check:
        stale = []
        for path, new in files.items():
            try:
                with open(path, encoding="utf-8") as f:
                    old = f.read()
            except FileNotFoundError:
                old = ""
            if old != new:
                stale.append(os.path.relpath(path, os.path.join(HERE, "..")))
        if stale:
            print("not the Python isnady's: " + ", ".join(stale) + " — run python tools/gen_tables.py")
            return 1
        print("tables.json and the interface catalogs are the Python isnady's")
        return 0
    for path, new in files.items():
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(new)
        print(f"wrote {os.path.relpath(path, os.path.join(HERE, '..'))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
