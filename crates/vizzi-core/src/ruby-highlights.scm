(comment) @comment
(constant) @type
[(integer) (float)] @number
[(true) (false) (nil)] @constant
[(string) (bare_string) (subshell) (heredoc_body) (simple_symbol)
 (delimited_symbol) (string_array) (symbol_array)] @string
(regex) @string
(method name: (_) @function)
(singleton_method name: (_) @function)
(call method: (_) @function)
[
  "alias" "and" "begin" "break" "case" "class" "def" "do" "else"
  "elsif" "end" "ensure" "for" "if" "in" "module" "next" "not" "or"
  "redo" "rescue" "retry" "return" "then" "undef" "unless" "until"
  "when" "while" "yield"
] @keyword
