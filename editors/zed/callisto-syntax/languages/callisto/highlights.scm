(comment) @comment

(string_literal) @string
(escape_sequence) @string.escape
(interpolation_start) @punctuation.special
(interpolation_end) @punctuation.special

(integer_literal) @number
(float_literal) @number
(boolean_literal) @boolean

(type_builtin) @type.builtin
(type_identifier) @type

(type_declaration name: (type_identifier) @type)
(newtype_declaration name: (type_identifier) @type)
(impl_declaration target: (type_identifier) @type)
(sum_variant name: (type_identifier) @constructor)
(record_literal name: (type_identifier) @constructor)
(pattern_constructor name: (type_identifier) @constructor)
(pattern_record name: (type_identifier) @constructor)

(function_declaration name: (identifier) @function)
(extern_function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(method_call_expression method: (identifier) @function.method)
(member_expression property: (identifier) @property)
(call_expression function: (member_expression property: (identifier) @function.method))

(parameter name: (identifier) @variable.parameter)
(let_binding name: (identifier) @variable)
(var_binding name: (identifier) @variable)
(assignment name: (identifier) @variable)

(record_type_field name: (identifier) @property)
(record_init_field name: (identifier) @property)
(record_update_field name: (identifier) @property)
(pattern_record_field (identifier) @property)

(module_path (identifier) @namespace)

[
  "module"
  "import"
  "pub"
  "extern"
  "fn"
  "type"
  "newtype"
  "impl"
  "let"
  "var"
  "if"
  "else"
  "match"
  "case"
  "while"
  "for"
  "in"
  "return"
  "with"
  "and"
  "or"
  "not"
] @keyword

[
  "=>"
  "->"
  ".."
  "=="
  "!="
  "<="
  ">="
  "="
  "<"
  ">"
  "+"
  "-"
  "*"
  "/"
  "%"
  "|"
  "."
] @operator

[
  ","
  ":"
] @punctuation.delimiter

[
  "("
  ")"
  "["
  "]"
  "{"
  "}"
] @punctuation.bracket

"_" @variable.special
