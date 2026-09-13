# Query language

Flashseek accepts Everything-class operators and Korean natural language. Both compile to the same `Query` AST.

## Operators

- space — AND
- `|` — OR (`foo|bar` or `foo | bar`)
- `!` — NOT
- `"quoted phrase"` — one term
- `*` `?` — glob on a term
- `ext:pdf` `ext:pdf;txt`
- `size:>1mb` `size:<10kb` `size:>=100`
- `dm:today` `dm:yesterday` `dm:lastweek` `dm:YYYY-MM-DD`
- `n:` / `name:` — basename only
- `file:` / `folder:`
- `case:` — case-sensitive
- `ww:` / `wholeword:` — whole word (Hangul counts as a word character)
- `regex:` / `r:` — `.` `*` `.*` `^` `$` on names (std matcher, no extra crate)
- `parent:` / `path:` — parent folder equals / path contains
- `attrib:R` `attrib:H` `attrib:D` — readonly, hidden, directory (`R|H` is any)
- `sort:size` `sort:date` `sort:name` — replace score order
- `attrib:R` / `attrib:H` / `attrib:D` — readonly, hidden, directory (Windows file attributes)

## Korean

| Phrase | Effect |
| --- | --- |
| 지난주 / 지난 주 / 최근 / 일주일 | last 7 days |
| 오늘 | last 24 hours |
| 어제 | 24–48 hours ago |
| 이번달 / 이번 달 | last 30 days |
| 올해 | last 365 days |
| 문서 | docx + pdf |
| 스프레드시트 | xlsx |
| 슬라이드 | pptx |
| 사진 / 이미지 | png + jpg + jpeg |

`지난주 세금 pdf` → 7-day window + `ext:pdf` + token `세금` matched in the name **or** the body.
