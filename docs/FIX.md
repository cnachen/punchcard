## Encoding Errors Fix
Some characters are not correctly encoded in src/encoding.rs.
They do not obey original IBM 029 encoding spec. Fix them.

## With Sequence Fix
`with_sequence` will overwrite existing data on a punch card. Fix it.

## Command Line Args Fix
`-s` and `--style` are meaningful if and only if `--render` is applied.
So they should be a sub arg of `--render`. Fix it.
Also rename those args as following:
`render` as a subcommand (alias `r`), `-s,--seq`, `-S,--style`

## Avoid Hard Coded String Line
Function `render_ascii` in src/punchcards.rs includes hard coded lines.
Avoid using them to improve readability.

## Improve Documentation Comments (DC) Quality
Add DC if needed. Remove useless DC. Keep DCs in a single style.
