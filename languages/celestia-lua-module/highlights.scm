"return" @keyword.return

[
  "goto"
  "in"
  "local"
] @keyword

(break_statement) @keyword

(do_statement ["do" "end"] @keyword)
(while_statement ["while" "do" "end"] @keyword)
(repeat_statement ["repeat" "until"] @keyword)
(if_statement ["if" "elseif" "else" "then" "end"] @keyword)
(for_statement ["for" "do" "end"] @keyword)
(function_declaration ["function" "end"] @keyword)
(function_definition ["function" "end"] @keyword)

[
  "and"
  "not"
  "or"
] @keyword.operator

[(false) (true)] @constant.language.boolean
(nil) @constant.language

(binary_expression operator: _ @operator)
(unary_expression operator: _ @operator)

(identifier) @variable
(parameters (identifier) @parameter)
(field name: (identifier) @property)
(dot_index_expression field: (identifier) @property)

(function_declaration
  name: (identifier) @function)
(function_call
  name: (identifier) @function.call)

(comment) @comment
(hash_bang_line) @preproc
(number) @constant.numeric
(string) @string.quoted.double
(escape_sequence) @string.escape
