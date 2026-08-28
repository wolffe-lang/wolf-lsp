" Vim syntax file
" Language:  wolf
" Source:    DERIVED, not written. See clients/nvim/inventory.md.
"
" This is the real highlighting story for wolf in Neovim today, not a stopgap
" for one: `wolffe-lang/tree-sitter-wolf` is scaffold-only, so a `.lu` buffer
" is coloured by these patterns unless a parser has been installed by hand.
" `wolf.nvim` detects that and switches without a message.
"
" EVERY token below is derived from the pinned grammar
" (`vendor/upstream/spec/grammar.ebnf` at the commit in `vendor/upstream/PIN`),
" and `cargo xtask nvim-check` re-derives the keyword set and fails on a
" difference. Nothing here is remembered, guessed, or extrapolated from another
" language that looks similar. The same discipline produced fackr's and
" facsimile's token tables; three independently-written tables from one pinned
" grammar is a cross-check on the extraction.

if exists("b:current_syntax")
  finish
endif

let s:cpo_save = &cpo
set cpo&vim

syn case match

" ---------------------------------------------------------------- keywords --
"
" The closed set of 50 `[gram.inv.kw]`, verbatim from `reserved_kw`, split into
" groups only for colouring. The split is cosmetic; the SET is the derived
" artifact, and the markers below are what the drift check reads.
"
" reserved-kw-begin
syn keyword wolfBoolean      true false
syn keyword wolfConditional  if else match when
syn keyword wolfRepeat       for while loop in
syn keyword wolfStatement    break continue return defer errdefer handle
syn keyword wolfStorageClass const let var mut pub comptime extern export
syn keyword wolfStorageClass distinct dyn shared weak
syn keyword wolfStructure    struct enum trait impl type fn region scope proc
syn keyword wolfKeyword      as asm assume borrow copy freeze import move
syn keyword wolfKeyword      select spawn take unsafe use
" reserved-kw-end
"
" CONTEXTUAL keywords are deliberately absent (`[gram.inv.ctx]`): `c`, `rc`,
" `pool`, `from`, `timeout`,
" `noalias`, `pkg`, `reg`, `self`, and the asm operand directions. Each is an
" ordinary identifier everywhere except one position, and a regex highlighter
" cannot see position. Painting `from` and `timeout` as keywords would be wrong
" far more often than right — a variable named `timeout` is not exotic.
"
" `self` is the one that tempts hardest, since it appears in every method
" signature. It is still an identifier, and colouring it would also colour
" `self` as a field name, a parameter, and a local.

" ------------------------------------------------------------------- types --
"
" The fixed-width scalar inventory is REAL at pin f9ee9aa: spec/10 (types,
" D54) writes `f32`/`f64`/`i32`/`u8` normatively, and the compiler's closed
" BUILTIN_TYPES set is the fifteen scalars plus the `wrapping` constructor
" (D56), painted here with `Self` beside them. (Through 70bdd35 this row was
" four names, because no spec then named a fixed-width scalar.) Not derivable
" from the EBNF — re-read wolf_sema/src/prelude.rs at each pin.
syn keyword wolfType bool byte f32 f64 i8 i16 i32 i64 int str u8 u16 u32 u64 uint wrapping Self

" `type` and `region` are also type-level (`type ::= … | 'type' | 'region'`)
" but they are reserved keywords, so they are already coloured above.

" ---------------------------------------------------------------- comments --
"
" `[gram.lex.comment]`. Wolf has NO block comment form — "nesting arguments
" lose to simplicity + lexer speed" — so there is no `/* */` region here and
" adding one would highlight a construct the lexer rejects.
"
" `///` (outer doc) and `//!` (inner doc) are prefixes of `//`, so they must be
" matched first; `syn match` takes the earliest-defined rule that matches at a
" position.
syn match wolfDocComment "//[/!].*$" contains=wolfTodo,@Spell
syn match wolfComment    "//.*$"     contains=wolfTodo,@Spell
syn keyword wolfTodo contained TODO FIXME XXX NOTE SAFETY

" ----------------------------------------------------------------- strings --
"
" `[gram.lex.str]`. Four literal forms and no character literal. ORDER MATTERS
" as much as it does in facsimile's table: `"""` must be defined before `"`, or
" the block form's opening fence is eaten as an empty string followed by a
" string start.
"
" EVERY string literal is an f-string (D26/X9): `{x}` re-enters token mode
" inside it, and `{{`/`}}` are the escapes. That is why `wolfInterp` exists —
" it is not a nicety, it is the difference between a wolf string and a C one.

" `{{` and `}}` first, so an escaped brace never opens an interpolation.
syn match wolfInterpEscape "{{\|}}" contained
" A single interpolation. Deliberately does not recurse into arbitrary
" expressions: `{x.method(1)}` colours as one unit. A regex highlighter cannot
" re-enter the expression grammar, and pretending otherwise produces confident
" wrong colours on nested braces. That gap is one of the arguments for semantic
" tokens whenever the compiler ships them (post-v1).
syn match wolfInterp "{[^{}]*}" contained contains=wolfFormatSpec
" Format specs, comptime-checked against argument types (D26): `{words:>3}`.
syn match wolfFormatSpec ":[^{}]*" contained

syn match wolfEscape "\\\%([nrt0\\'\"]\|x\x\x\|u{\x\{1,6}}\)" contained

" Block strings: dedented by the closing delimiter's column.
syn region wolfBlockString start=+"""+ end=+"""+ keepend
      \ contains=wolfInterp,wolfInterpEscape,wolfEscape,@Spell

" Raw strings: `r#"…"#`, fences balanced by count. Vim's regex has no counting
" construct, so the common depths are enumerated — longest first, because an
" `r##"` opener must not be matched by the `r#"` rule.
syn region wolfRawString start=+r###"+ end=+"###+ keepend contains=@Spell
syn region wolfRawString start=+r##"+  end=+"##+  keepend contains=@Spell
syn region wolfRawString start=+r#"+   end=+"#+   keepend contains=@Spell
syn region wolfRawString start=+r"+    end=+"+    keepend contains=@Spell

" Generalized literals: `re"[a-z]+"`, `path"/etc/hosts"` — an identifier fused
" to a string. The prefix is matched as part of the literal so it colours as
" one thing; `re` alone would otherwise read as a variable.
"
" No interpolation inside: the whole point of `re"…"` is that its body is
" handed to another grammar. `nextgroup` + `skipwhite` is not used because the
" fusion is lexical, with no space allowed between prefix and quote.
syn match wolfGeneralized +\<\h\w*"[^"]*"+ contains=wolfGeneralizedPrefix
syn match wolfGeneralizedPrefix +\<\h\w*\ze"+ contained

syn region wolfString start=+"+ skip=+\\"+ end=+"+ keepend
      \ contains=wolfInterp,wolfInterpEscape,wolfEscape,@Spell

" ----------------------------------------------------------------- numbers --
"
" INT ::= DEC_LIT | '0x'… | '0o'… | '0b'…, each with `_` separators.
" FLOAT ::= DEC_LIT '.' DEC_LIT EXPONENT? | DEC_LIT EXPONENT.
syn match wolfNumber "\<0x[0-9A-Fa-f_]\+\>"
syn match wolfNumber "\<0o[0-7_]\+\>"
syn match wolfNumber "\<0b[01_]\+\>"
syn match wolfFloat  "\<\d[0-9_]*\.\d[0-9_]*\%([eE][+-]\=\d[0-9_]*\)\=\>"
syn match wolfFloat  "\<\d[0-9_]*[eE][+-]\=\d[0-9_]*\>"
syn match wolfNumber "\<\d[0-9_]*\>"

" -------------------------------------------------------------- attributes --
"
" `#[…]`. Wolf has no preprocessor, so `#` is not a line-directive marker and
" must not swallow the rest of the line (facsimile's table records the same
" trap from the other direction).
syn region wolfAttribute start="#\[" end="\]" keepend contains=wolfString,wolfNumber

" --------------------------------------------------------------- operators --
"
" From the operator inventory. `grammar.ebnf` is generated by `wolf xtask
" spec-extract` and the extraction ELIDES the precedence climb — the file says
" so itself — so the comparison and logical operators below come from
" `spec/01-grammar.md` §3.2 rather than from the EBNF's quoted terminals. That
" extraction gap is recorded in `inventory.md` and is open upstream; a drift
" check reading `grammar.ebnf` alone cannot see an operator change, which is
" why the automated check covers the KEYWORD set only and this comment exists
" instead of a false claim of coverage.
"
" Longest-first within one alternation, so `<=>` is not matched as `<=` + `>`.
syn match wolfOperator "<=>\|<<=\|>>=\|\.\.="
syn match wolfOperator "==\|!=\|<=\|>=\|&&\|||\|<<\|>>\|\.\.\|->\|=>"
syn match wolfOperator "+=\|-=\|\*=\|/=\|%=\|&=\||=\|\^="
syn match wolfOperator "[-+*/%&|^!<>=?@]"

" `?` is postfix error propagation, `!` is prefix not AND the `!T` error-row
" marker AND part of `!=`, and `^` is bitwise xor AND D25's from-end index
" marker (`s[^1]`). A highlighter that only colours them does not have to tell
" them apart — which is exactly why they share one group.

" ------------------------------------------------------------------ region --
"
" Braces are matched only to give `%` and folding something to work with; no
" `syn region` for blocks, because an unbalanced brace in a file being edited
" would then desynchronise the whole buffer below the cursor.
syn match wolfDelimiter "[{}()\[\];,.]"

" A function name at its definition site, so `fn` lines stand out. This is the
" one structural inference in the file, and it is safe: `fn_item ::= fn_qual*
" 'fn' IDENT …` puts an identifier immediately after `fn`, always.
syn match wolfFunction "\%(\<fn\s\+\)\@<=\h\w*"

" Vim's syntax engine re-scans from a bounded distance above the window.
" Strings are the only multi-line construct here, and 200 lines comfortably
" covers a `"""` block in canonical style.
syn sync minlines=200

" ------------------------------------------------------------- highlighting --
hi def link wolfKeyword           Keyword
hi def link wolfConditional       Conditional
hi def link wolfRepeat            Repeat
hi def link wolfStatement         Statement
hi def link wolfStorageClass      StorageClass
hi def link wolfStructure         Structure
hi def link wolfBoolean           Boolean
hi def link wolfType              Type
hi def link wolfComment           Comment
hi def link wolfDocComment        SpecialComment
hi def link wolfTodo              Todo
hi def link wolfString            String
hi def link wolfBlockString       String
hi def link wolfRawString         String
hi def link wolfGeneralized       String
hi def link wolfGeneralizedPrefix PreProc
hi def link wolfInterp            Special
hi def link wolfInterpEscape      SpecialChar
hi def link wolfFormatSpec        Special
hi def link wolfEscape            SpecialChar
hi def link wolfNumber            Number
hi def link wolfFloat             Float
hi def link wolfAttribute         PreProc
hi def link wolfOperator          Operator
hi def link wolfDelimiter         Delimiter
hi def link wolfFunction          Function

let b:current_syntax = "wolf"

let &cpo = s:cpo_save
unlet s:cpo_save
