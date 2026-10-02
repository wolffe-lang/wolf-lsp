; tree-sitter-wolf highlights.
; Standard capture names (the helix/nvim scope family — dotted names
; degrade by prefix in every consumer). Specific patterns first: the
; canonical tree-sitter-highlight engine (helix, zed) is first-match-wins.

; ------------------------------------------------------------- comments

(doc_comment) @comment.block.documentation
(line_comment) @comment.line

; ------------------------------------------------------------- strings

(escape_sequence) @constant.character.escape
(brace_escape) @constant.character.escape

; `STR_ESC` derives it or it is E0101 at the escape (v0.2.2) — the
; refusal is a node, not an ERROR, so the rest of the literal survives.
(invalid_escape) @error

; interpolation braces are code, not string content
(interpolation
  "{" @punctuation.special
  "}" @punctuation.special)

(format_spec) @string.special

; generalized literals: the prefix is a comptime call
(generalized_string_literal
  prefix: (identifier) @function.macro)

(string_literal) @string
(multiline_string_literal) @string
(raw_string_literal) @string
(generalized_string_literal) @string

; ------------------------------------------------------------- literals

(boolean_literal) @constant.builtin.boolean
(integer_literal) @constant.numeric.integer
(float_literal) @constant.numeric.float
(char_literal) @constant.character

; ------------------------------------------------------------ functions

(function_item
  name: (identifier) @function)

(call_expression
  function: (field_expression
    field: (identifier) @function.method))

(call_expression
  function: (identifier) @constructor
  (#match? @constructor "^[A-Z]"))

(call_expression
  function: (identifier) @function)

(spawn_expression
  function: (path (identifier) @function))

; -------------------------------------------------------------- types

(struct_item name: (identifier) @type)
(enum_item name: (identifier) @type)
(type_item name: (identifier) @type)
(trait_item name: (identifier) @type)
(generic_parameter name: (identifier) @type.parameter)
(enum_variant name: (identifier) @type.enum.variant)
(error_item name: (identifier) @type)
(constructor_pattern type: (path (identifier) @constructor))
;; s157 ([gram.pat.nullary]) â a payload-less variant paints as the
;; constructor it is; only the parens are missing.
(path_pattern type: (path (identifier) @constructor))
(struct_pattern type: (path (identifier) @type))
(field_pattern name: (identifier) @variable.other.member)
(rest_pattern) @operator
(row_entry (path (identifier) @type.enum.variant))
(struct_expression name: (type_path (path (identifier) @type)))

;; The builtin scalar names, and this list is CLOSED because the language's
;; is. `usize` and `isize` rode in with the first queries commit (le02) as
;; unexamined Rust-isms: no `spec/*.md` names either one, and neither is in
;; the compiler's closed builtin set at the v0.2.3 tag. Painting them
;; @type.builtin taught a reader two types wolf does not have — the same
;; wrong as painting a raw literal's braces, filed the other way round. Gone
;; at le06.
;;
;; `wrapping` joins them instead: D56's wrapping-arithmetic constructor is
;; a builtin type name (`wrapping[u32]`, `[type.numlit.cast.wrap]`), and it
;; is the one name in that set this list had never carried.
;;
;; `byte` was already here and it is correct — le06 checked, since the
;; sprint contract expected it might still be owed. It is a builtin at this
;; tag already; whatever wolf-lang s135 does to the byte tier, this line does
;; not wait on it.
;;
;; `Self` stays out deliberately: it is not a builtin type NAME but a
;; context-bound alias, and it already paints through `(type_path (path
;; (identifier)) @type)` wherever it can appear.
((identifier) @type.builtin
  (#any-of? @type.builtin
    "int" "uint" "i8" "i16" "i32" "i64" "u8" "u16" "u32" "u64"
    "f32" "f64" "bool" "str" "byte" "char" "wrapping"))

;; `range[int]` / `range[char]` â s158's prelude type ([type.range]).
;;
;; Deliberately NOT added to the closed builtin-scalar list above, and
;; the distinction is upstream's own: wolfc leaves `BUILTIN_TYPES` at
;; the same seventeen prims and puts `range` in `PRELUDE`, beside
;; `List` and `channel`, because it takes an argument. It also adds
;; `PRELUDE_TYPE_ONLY`, whose entire contents is `range`, because the
;; name resolves in TYPE position only â there is no `range(â¦)`
;; constructor, a range value being spelled `a..b`.
;;
;; So this is scoped to type position, and that scoping is load-bearing
;; rather than tidy. The `#any-of?` list above is matched against a
;; bare `(identifier)` in ANY position, which is why `var int = 5`
;; paints its own binding `@type.builtin` today. `int` is a name almost
;; nobody binds; `range` is one wolf-lang's corpus binds twice, in
;; `corpus/os/random_differs.lu` and `corpus/os/random_edges.lu`, both
;; `var range = true`. Putting it in that list would have coloured
;; them.
;;
;; The path is anchored to a single segment: the prelude name is
;; one segment, so a user's `foo.range` is untouched.
((type_path (path . (identifier) @type.builtin .))
  (#eq? @type.builtin "range"))

(type_path (path (identifier) @type))
;; `T ! IoErrors` (s158) â the tail names a declared error SET, so the
;; path is a type reference in type position, not one of the tags a
;; braced `! {â¦}` row spells with @type.enum.variant.
(aliased_error_type (path (identifier) @type))
(dyn_type (path (identifier) @type))
(trait_bound (path (identifier) @type))
(region_type) @type.builtin
(type_keyword) @type.builtin

; ------------------------------------------------------------- members

(field_expression
  field: (identifier) @variable.other.member)
(field_declaration
  name: (identifier) @variable.other.member)
(field_initializer
  name: (identifier) @variable.other.member)
(shorthand_field_initializer
  (identifier) @variable.other.member)

; ---------------------------------------------------------- parameters

(parameter name: (identifier) @variable.parameter)
(closure_parameter name: (identifier) @variable.parameter)
(self) @variable.builtin
(wildcard_pattern) @variable.builtin

; ----------------------------------------------------------- constants

(const_declaration name: (identifier) @constant)

((identifier) @constant
  (#match? @constant "^[A-Z][A-Z0-9_]+$"))

; ---------------------------------------------------------- attributes

(attribute) @attribute
(inner_attribute) @attribute
(shebang) @comment

; ------------------------------------------------------------ keywords

[
  "fn"
] @keyword.function

; `then` is contextual (s151, wolf-lang#307): the anonymous node exists
; only where the grammar admits the keyword, i.e. after a complete `if`
; condition. `let then = true`, `if then { … }` and `less.then(greater)`
; are `identifier` nodes and no pattern here can reach them.
[
  "if"
  "then"
  "else"
  "match"
  "select"
  "when"
] @keyword.control.conditional

[
  "for"
  "while"
  "loop"
  "in"
] @keyword.control.repeat

[
  "return"
  "break"
] @keyword.control.return

(continue_expression) @keyword.control.return

[
  "use"
  "import"
] @keyword.control.import

[
  "defer"
  "errdefer"
] @keyword.control

[
  "let"
  "var"
  "const"
  "pub"
  "mut"
  "take"
  "extern"
  "export"
  "comptime"
  "distinct"
] @keyword.storage.modifier

;; `error` is contextual (s158, wolf-lang#36) and joins the group that
;; declares a named thing, beside `type` and `trait` â the issue asked
;; only for `@keyword`, and this is that, refined the way every other
;; item keyword in this file already is.
;;
;; Safer here than `then` is: `then` relies on LR state alone, while
;; this anonymous node comes from an EXTERNAL token that fires only on
;; `error` IDENT `=`. `error = 4`, `error(x)`, `r.error` and a field
;; named `error` are `identifier` nodes and no pattern here reaches
;; them.
[
  "struct"
  "enum"
  "type"
  "trait"
  "impl"
  "error"
] @keyword.storage.type

[
  "as"
  "move"
  "copy"
  "shared"
  "freeze"
  "dyn"
  "handle"
  "weak"
] @keyword.operator

[
  "region"
  "scope"
  "spawn"
  "proc"
  "unsafe"
  "asm"
  "assume"
  "borrow"
  "noalias"
  "from"
  "timeout"
  "rc"
  "pool"
  "cap"
  "pkg"
  "c"
] @keyword

; ------------------------------------------------------------ operators

[
  "="  "+=" "-=" "*=" "/=" "%=" "&=" "|=" "^=" "<<=" ">>="
  "+" "-" "*" "/" "%"
  "<<" ">>" "&" "^" "|"
  "==" "!=" "<" ">" "<=" ">=" "<=>"
  "&&" "||" "!"
  ".." "..="
  "->" "=>" "?" "@"
] @operator

; --------------------------------------------------------- punctuation

["(" ")" "[" "]" "{" "}" "#["] @punctuation.bracket
["," ";" ":" "."] @punctuation.delimiter

; ------------------------------------------------------------ fallback

(identifier) @variable
