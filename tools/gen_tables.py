"""The tables isnady-rs reads, written from the Python isnady itself — so the two use the same word lists, the same
forms and the same patterns, never a copy kept by hand.

    python tools/gen_tables.py                 # the Python isnady installed (pip install isnady), or ../isnady/src
    python tools/gen_tables.py --check         # exit 1 when data/tables.json or data/i18n/*.json is not what it writes

It also writes the interface catalogs (crates/isnady-core/data/i18n/<code>.json) of every language the Python isnady
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
        "name_parts": {
            "not_names": sorted(name_parts._NOT_NAMES),
            "laqab": name_parts._LAQAB_PHRASE.pattern,
            "known": name_parts._KNOWN_PHRASE.pattern,
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


def outputs() -> dict:
    files = {OUT: text(), BOOKS: books()}
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
