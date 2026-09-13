# Flashseek design

Windows instant search. Names are a complete catalog (Everything-class). Bodies are indexed only in folders the user chooses.

## Process

One library (`flashseek`) drives both `flashseek-cli` and the native `flashseek` window. Indexing I/O (directory walk today, `$MFT`/USN when privileged) is an adapter that emits `FileRecord` / `CatalogEvent`. Tests inject the same records without a real volume.

## Query

`parse_query` implements space=AND, `|=OR`, `!=NOT`, `*`/`?`, `ext:`, `size:`, `dm:`, `attrib:`.
`compile_nl` maps Korean (`지난주`, `오늘`, `어제`, `이번달`, `올해`, type words) onto that AST. Structured tokens (`ext:`, `size:`, `dm:`, `|`, `!`) always take the query-language path.

## Ranking

Name match > path match > content-only, plus a recency decay over seven days. Exact basename equality can add a further boost.

## Content

Extractors: txt/md (plain), html (strip tags), pdf (literal strings + UTF-8 runs), docx/xlsx/pptx (zip + XML text). Files larger than 32 MiB are name-only.

## Preview

Hits carry a snippet with highlight offsets. If there is no snippet, the UI looks up the Windows IPreviewHandler CLSID for the extension.

## Privileged path

`mft::try_index_volume` must actually open the volume. Access denied is expected without admin; the walker and injected catalogs remain valid substitutes.
