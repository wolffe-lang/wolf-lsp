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
(constructor_pattern type: (path (identifier) @constructor))
(struct_pattern type: (path (identifier) @type))
(field_pattern name: (identifier) @variable.other.member)
(rest_pattern) @operator
(row_entry (path (identifier) @type.enum.variant))
(struct_expression name: (type_path (path (identifier) @type)))

((identifier) @type.builtin
  (#any-of? @type.builtin
    "int" "uint" "i8" "i16" "i32" "i64" "u8" "u16" "u32" "u64"
    "f32" "f64" "bool" "str" "byte" "usize" "isize" "char"))

(type_path (path (identifier) @type))
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

[
  "if"
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

[
  "struct"
  "enum"
  "type"
  "trait"
  "impl"
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
