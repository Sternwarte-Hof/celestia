(comment) @comment.line

(command_statement
  name: (command_name
    (identifier) @keyword.control))

(property_entry key: (identifier) @property)
(property_entry key: (string) @property)

(string) @string.quoted.double
(number) @constant.numeric
(boolean) @constant.language.boolean
