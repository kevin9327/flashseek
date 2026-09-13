# Flashseek

Windows instant file search. Name catalog is Everything-class (complete tree, live apply, operators). Content search is limited to folders you choose.

## Why

[Everything](https://www.voidtools.com/) is already the fastest filename search on NTFS. Flashseek keeps that bar for **names** and adds what Everything leaves on the table: **body search** in selected folders, **Korean natural-language** queries, and a **preview snippet** so you can see why a hit matched.

## Query language

| Input | Meaning |
| --- | --- |
| `report budget` | AND (space) |
| `report\|memo` | OR |
| `report !tmp` | NOT |
| `"foo bar"` | quoted phrase |
| `*.pdf` / `invoice*.docx` | wildcards `*` `?` |
| `ext:pdf` / `ext:pdf;txt` | extension filter |
| `size:>1mb` `size:<10kb` | size filter |
| `dm:today` `dm:lastweek` | date-modified filter |
| `n:alpha` / `name:alpha` | match basename only |
| `file:` / `folder:` | files or directories |
| `case:` / `ww:` | case-sensitive / whole-word |
| `지난주 세금 pdf` | last ~7 days + `ext:pdf` + token `세금` (name **or** body) |
| `최근` `일주일` `오늘` `어제` `이번달` `올해` | Korean date windows |
| `문서` `스프레드시트` `슬라이드` `사진` | Korean type words |

## Usage

Headless (this is the proof path):

```
flashseek-cli --name-root C:\ --content-root C:\Users\you\Documents --query "지난주 세금 pdf" --json --out hits.json
```

Config file (`name_root=` / `content_root=` lines):

```
flashseek-cli --config %USERPROFILE%\flashseek.conf --query "invoice ext:pdf" --files-only --max-results 20
```

Window (native, not a webview):

```
cargo run --bin flashseek --features ui
```

## Architecture

- **Name catalog** — in-memory records. Filled from a directory walk today; NTFS `$MFT` / USN adapter when privileged.
- **Content index** — extracted text for `txt` `md` `html` `pdf` `docx` `xlsx` `pptx` under configured folders only.
- **Same functions** power the CLI and the window: parse → name match → content match → rank.

Repo: https://github.com/kevin9327/flashseek

## License

MIT
