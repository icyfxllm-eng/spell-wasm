# -*- coding: utf-8 -*-
"""Stdlib-only .xlsx reader (no openpyxl on the build machines). Returns raw cell
strings exactly as stored -- NO strip, NO normalisation: the ingest guards must see
leading/trailing whitespace, so a reader that tidies would hide the faults."""
import re
import zipfile
import xml.etree.ElementTree as ET

NS = {"m": "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
      "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships"}


def _col(ref):
    n = 0
    for ch in re.match(r"[A-Z]+", ref).group(0):
        n = n * 26 + ord(ch) - 64
    return n - 1


def read_sheet(path, sheet_name):
    """Rows of one sheet as lists of str ('' for empty cells), row 1 = header."""
    z = zipfile.ZipFile(path)
    wb = ET.fromstring(z.read("xl/workbook.xml"))
    rels = ET.fromstring(z.read("xl/_rels/workbook.xml.rels"))
    rid = None
    for s in wb.find("m:sheets", NS):
        if s.get("name") == sheet_name:
            rid = s.get("{%s}id" % NS["r"])
    if rid is None:
        raise KeyError("no sheet named %r" % sheet_name)
    target = next(r.get("Target") for r in rels if r.get("Id") == rid)
    target = target.lstrip("/")
    if not target.startswith("xl/"):
        target = "xl/" + target
    shared = []
    if "xl/sharedStrings.xml" in z.namelist():
        for si in ET.fromstring(z.read("xl/sharedStrings.xml")):
            shared.append("".join(t.text or "" for t in si.iter("{%s}t" % NS["m"])))
    out = []
    for row in ET.fromstring(z.read(target)).iter("{%s}row" % NS["m"]):
        cells = {}
        for c in row:
            kind = c.get("t")
            if kind == "inlineStr":
                v = "".join(t.text or "" for t in c.iter("{%s}t" % NS["m"]))
            else:
                ve = c.find("m:v", NS)
                v = "" if ve is None or ve.text is None else ve.text
                if kind == "s":
                    v = shared[int(v)]
            cells[_col(c.get("r"))] = v
        out.append([cells.get(i, "") for i in range(max(cells) + 1)] if cells else [])
    return out
