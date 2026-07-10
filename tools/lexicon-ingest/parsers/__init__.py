"""Source parsers. Each `parse(path, lang) -> Iterator[Entry]` reads one raw
source into schema.Entry records (pre-normalization; the driver normalizes,
validates, filters, and merges).

`plainlist` works today against the shipped assets/words lists (the migration
path). The dictionary parsers (jmdict, cedict, kaikki, wordfreq, cmudict) are
real parsers guarded on the dump being present under tools/lexicon-ingest/
sources/ — absent, they raise SourceMissing with the exact URL to fetch, per
the pipeline's "no network in builds" rule.
"""


class SourceMissing(Exception):
    def __init__(self, name: str, url: str, path: str):
        super().__init__(f"source '{name}' not found at {path} — download from {url}")
        self.name, self.url, self.path = name, url, path
