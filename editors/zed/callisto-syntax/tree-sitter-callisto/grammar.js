const PREC = {
  OR: 1,
  AND: 2,
  EQUALITY: 3,
  RELATIONAL: 4,
  ADD: 5,
  MULTIPLY: 6,
  WITH: 7,
  UNARY: 8,
  POSTFIX: 9,
  TYPE_ARROW: 1,
};

module.exports = grammar({
  name: "callisto",

  extras: ($) => [/[\s\uFEFF\u2060\u200B\n\r\t]/, $.comment],

  word: ($) => $.identifier,

  supertypes: ($) => [$._statement, $._expression, $._type_expression],

  conflicts: ($) => [
    [$.module_path],
  ],

  rules: {
    source_file: ($) => repeat($._top_level_item),

    _top_level_item: ($) =>
      choice(
        $.module_declaration,
        $.import_declaration,
        $.type_declaration,
        $.function_declaration,
        $.impl_declaration,
        $.extern_type_declaration,
        $.extern_function_declaration,
        $.extern_module_declaration,
        $._statement,
      ),

    comment: (_) => token(seq("//", /.*/)),

    module_declaration: ($) => seq("module", $.module_path),

    import_declaration: ($) =>
      seq("import", $.module_path, optional($.import_list)),

    module_path: ($) => seq($.identifier, repeat(seq(".", $.identifier))),

    import_list: ($) => seq(".", "{", commaSep1($.identifier), optional(","), "}"),

    type_declaration: ($) =>
      seq(
        optional("pub"),
        "type",
        field("name", $.type_identifier),
        optional($.type_parameters),
        choice(
          $.record_type_definition,
          seq("=", $.sum_type_definition),
          seq("=", $._type_expression),
        ),
      ),

    type_parameters: ($) => seq("[", commaSep1($.type_identifier), optional(","), "]"),

    record_type_definition: ($) =>
      seq("{", commaSep1($.record_type_field), optional(","), "}"),

    record_type_field: ($) =>
      seq(field("name", $.identifier), ":", field("type", $._type_expression)),

    sum_type_definition: ($) => repeat1($.sum_variant),

    sum_variant: ($) =>
      prec.right(
        seq(
          "|",
          field("name", $.type_identifier),
          optional(choice($.variant_positional_payload, $.variant_record_payload)),
        ),
      ),

    variant_positional_payload: ($) =>
      seq("(", commaSep1($._type_expression), optional(","), ")"),

    variant_record_payload: ($) => $.record_type_definition,

    impl_declaration: ($) =>
      seq(
        "impl",
        field("target", $.type_identifier),
        "do",
        repeat(choice($.function_declaration, $._statement)),
        "end",
      ),

    function_declaration: ($) =>
      seq(
        optional("pub"),
        "fn",
        field("name", $.identifier),
        $.parameter_list,
        optional(seq("->", $._type_expression)),
        optional(seq("do", repeat($._statement), "end")),
      ),

    extern_type_declaration: ($) => seq("extern", "type", field("name", $.type_identifier)),

    extern_function_declaration: ($) =>
      seq(
        "extern",
        "fn",
        field("name", $.identifier),
        $.parameter_list,
        optional(seq("->", $._type_expression)),
      ),

    extern_module_declaration: ($) =>
      seq("extern", "module", $.module_path, "do", repeat($.extern_function_declaration), "end"),

    parameter_list: ($) => seq("(", optional(commaSep1($.parameter)), optional(","), ")"),

    parameter: ($) => seq(field("name", $.identifier), ":", field("type", $._type_expression)),

    _statement: ($) =>
      choice(
        $.let_binding,
        $.var_binding,
        $.assignment,
        $.return_statement,
        $.while_statement,
        $.for_statement,
        $.expression_statement,
      ),

    let_binding: ($) =>
      seq(
        "let",
        field("name", $.identifier),
        optional(seq(":", $._type_expression)),
        "=",
        field("value", $._expression),
      ),

    var_binding: ($) =>
      seq(
        "var",
        field("name", $.identifier),
        optional(seq(":", $._type_expression)),
        "=",
        field("value", $._expression),
      ),

    assignment: ($) => seq(field("name", $.identifier), "=", field("value", $._expression)),

    return_statement: ($) =>
      prec.right(choice(seq("return", $._expression), "return")),

    while_statement: ($) =>
      seq(
        "while",
        field("condition", $._expression),
        "do",
        repeat($._statement),
        "end",
      ),

    for_statement: ($) =>
      seq(
        "for",
        field("name", $.identifier),
        "in",
        field("start", $._expression),
        "..",
        field("end", $._expression),
        "do",
        repeat($._statement),
        "end",
      ),

    expression_statement: ($) => $._expression,

    _expression: ($) =>
      choice(
        $.if_expression,
        $.match_expression,
        $.lambda_expression,
        $.record_update_expression,
        $.binary_expression,
        $.unary_expression,
        $.method_call_expression,
        $.call_expression,
        $.member_expression,
        $.record_literal,
        $.parenthesized_expression,
        $.unit_literal,
        $.string_literal,
        $.float_literal,
        $.integer_literal,
        $.boolean_literal,
        $.type_identifier,
        $.identifier,
      ),

    if_expression: ($) =>
      seq(
        "if",
        field("condition", $._expression),
        "then",
        repeat($._statement),
        repeat($.elseif_clause),
        "else",
        repeat($._statement),
        "end",
      ),

    elseif_clause: ($) =>
      seq(
        "elseif",
        field("condition", $._expression),
        "then",
        repeat($._statement),
      ),

    match_expression: ($) =>
      seq(
        "match",
        field("value", $._expression),
        optional("do"),
        repeat1($.match_case),
        "end",
      ),

    match_case: ($) => seq("case", field("pattern", $.pattern), "=>", field("value", $._expression)),

    pattern: ($) =>
      choice(
        "_",
        $.identifier,
        $.type_identifier,
        $.integer_literal,
        $.float_literal,
        $.string_literal,
        $.boolean_literal,
        $.pattern_constructor,
        $.pattern_record,
      ),

    pattern_constructor: ($) =>
      seq(field("name", $.type_identifier), "(", commaSep1($.pattern), optional(","), ")"),

    pattern_record: ($) =>
      seq(field("name", $.type_identifier), "{", commaSep1($.pattern_record_field), optional(","), "}"),

    pattern_record_field: ($) => choice($.identifier, seq($.identifier, "=", $.identifier)),

    lambda_expression: ($) =>
      seq(
        "fn",
        "(",
        optional(commaSep1($.parameter)),
        optional(","),
        ")",
        optional(seq("->", $._type_expression)),
        "=>",
        $._expression,
      ),

    record_literal: ($) =>
      seq(field("name", $.type_identifier), field("fields", $.record_init_fields)),

    record_init_fields: ($) =>
      seq("{", commaSep1($.record_init_field), optional(","), "}"),

    record_init_field: ($) =>
      seq(field("name", $.identifier), "=", field("value", $._expression)),

    record_update_expression: ($) =>
      prec.left(PREC.WITH, seq(field("record", $._expression), "with", $.record_update_fields)),

    record_update_fields: ($) =>
      seq("{", commaSep1($.record_update_field), optional(","), "}"),

    record_update_field: ($) =>
      seq(field("name", $.identifier), "=", field("value", $._expression)),

    binary_expression: ($) =>
      choice(
        prec.left(PREC.OR, seq($._expression, "or", $._expression)),
        prec.left(PREC.AND, seq($._expression, "and", $._expression)),
        prec.left(PREC.EQUALITY, seq($._expression, choice("==", "!="), $._expression)),
        prec.left(PREC.RELATIONAL, seq($._expression, choice("<", "<=", ">", ">="), $._expression)),
        prec.left(PREC.ADD, seq($._expression, choice("+", "-"), $._expression)),
        prec.left(PREC.MULTIPLY, seq($._expression, choice("*", "/", "%"), $._expression)),
      ),

    unary_expression: ($) => prec(PREC.UNARY, seq(choice("-", "not"), $._expression)),

    call_expression: ($) =>
      prec.left(PREC.POSTFIX, seq(field("function", $._expression), field("arguments", $.argument_list))),

    method_call_expression: ($) =>
      prec.left(
        PREC.POSTFIX,
        seq(
          field("object", $._expression),
          ".",
          field("method", $.identifier),
          field("arguments", $.argument_list),
        ),
      ),

    member_expression: ($) =>
      prec.left(
        PREC.POSTFIX,
        seq(field("object", $._expression), ".", field("property", $.identifier)),
      ),

    argument_list: ($) => seq("(", optional(commaSep1($._expression)), optional(","), ")"),

    parenthesized_expression: ($) => seq("(", $._expression, ")"),

    unit_literal: (_) => seq("(", ")"),

    string_literal: ($) =>
      seq(
        '"',
        repeat(choice($.string_text, $.escape_sequence, $.interpolation, "$")),
        '"',
      ),

    string_text: (_) => token(prec(1, /[^"\\$]+/)),

    escape_sequence: (_) => token(seq("\\", /./)),

    interpolation: ($) =>
      seq(alias("${", $.interpolation_start), field("expression", $._expression), alias("}", $.interpolation_end)),

    _type_expression: ($) => choice($.function_type, $.nullable_type, $._atomic_type),

    function_type: ($) =>
      prec.right(PREC.TYPE_ARROW, seq($._atomic_type, "->", $._type_expression)),

    nullable_type: ($) => prec.right(PREC.UNARY, seq($._atomic_type, "not")),

    _atomic_type: ($) =>
      choice(
        $.generic_type,
        $.parenthesized_type,
        $.unit_type,
        $.type_builtin,
        $.type_identifier,
        $.identifier,
      ),

    generic_type: ($) =>
      seq(field("name", choice($.type_identifier, $.identifier)), "[", commaSep1($._type_expression), optional(","), "]"),

    parenthesized_type: ($) => seq("(", $._type_expression, ")"),

    unit_type: (_) => seq("(", ")"),

    type_builtin: (_) => choice("Int", "Float", "Bool", "String", "Unit", "Nil", "nil"),

    float_literal: (_) => token(/\d+\.\d+/),

    integer_literal: (_) => token(/\d+/),

    boolean_literal: (_) => choice("true", "false"),

    identifier: (_) => /[a-z_][a-zA-Z0-9_]*/,

    type_identifier: (_) => /[A-Z][a-zA-Z0-9_]*/,
  },
});

function commaSep1(rule) {
  return seq(rule, repeat(seq(",", rule)));
}
