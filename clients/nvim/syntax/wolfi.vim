" Vim syntax file
" Language:  wolf interface (.wolfi)
"
" A `.wolfi` is generated wolf declarations, so it is lexically wolf and
" deserves exactly one syntax file's worth of maintenance. Sourcing rather than
" copying is the whole point: a keyword added to the derived set in
" `syntax/wolf.vim` must not need a second edit here to appear.

if exists("b:current_syntax")
  finish
endif

runtime! syntax/wolf.vim

let b:current_syntax = "wolfi"
