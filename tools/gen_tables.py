"""The tables isnady-rs reads, written from the Python isnady itself — so the two use the same word lists, the same
forms and the same patterns, never a copy kept by hand.

    python tools/gen_tables.py                 # the Python isnady installed (pip install isnady), or ../isnady/src
    python tools/gen_tables.py --check         # exit 1 when crates/isnady-core/data/tables.json is not what it writes

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
        },
        "name_parts": {
            "not_names": sorted(name_parts._NOT_NAMES),
            "laqab": name_parts._LAQAB_PHRASE.pattern,
            "known": name_parts._KNOWN_PHRASE.pattern,
        },
    }


def text() -> str:
    return json.dumps(tables(), ensure_ascii=False, indent=1, sort_keys=True) + "\n"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="only say whether the tables are up to date")
    a = ap.parse_args()
    new = text()
    if a.check:
        try:
            with open(OUT, encoding="utf-8") as f:
                old = f.read()
        except FileNotFoundError:
            old = ""
        if old != new:
            print("crates/isnady-core/data/tables.json is not the Python isnady's: run python tools/gen_tables.py")
            return 1
        print("tables.json is the Python isnady's")
        return 0
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write(new)
    print(f"wrote {os.path.relpath(OUT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
