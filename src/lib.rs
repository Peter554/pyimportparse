use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct GrammarParser;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Import {
    pub imported_object: String,
    pub line_number: usize,
    pub typechecking_only: bool,
}

impl Import {
    pub fn new(imported_object: &str, line_number: usize, typechecking_only: bool) -> Self {
        Self {
            imported_object: imported_object.to_owned(),
            line_number,
            typechecking_only,
        }
    }
}

#[derive(Debug)]
struct ParseContext {
    typechecking_only: bool,
}

pub fn parse_imports(code: &str) -> Result<Vec<Import>, String> {
    let pair = GrammarParser::parse(Rule::CODE, code)
        .map_err(|e| format!("failed to parse: {}", e))?
        .next()
        .unwrap();

    let mut context = ParseContext {
        typechecking_only: false,
    };

    Ok(parse_pair(pair, &mut context))
}

fn parse_pair(pair: Pair<Rule>, context: &mut ParseContext) -> Vec<Import> {
    match pair.as_rule() {
        Rule::CODE
        | Rule::FRAGMENT
        | Rule::IMPORT_STATEMENT_LIST
        | Rule::IF_TYPECHECKING_FRAGMENT => parse_inner_pairs(pair, context),
        Rule::IF_TYPECHECKING | Rule::SINGLELINE_IF_TYPECHECKING => {
            context.typechecking_only = true;
            let imports = parse_inner_pairs(pair, context);
            context.typechecking_only = false;
            imports
        }
        Rule::SIMPLE_IMPORT_STATEMENT => parse_simple_import_statement(pair, context),
        Rule::FROM_IMPORT_STATEMENT | Rule::MULTILINE_FROM_IMPORT_STATEMENT => {
            parse_from_import_statement(pair, context)
        }
        Rule::WILDCARD_FROM_IMPORT_STATEMENT => parse_wildcard_from_import_statement(pair, context),
        Rule::MULTILINE_STRING => {
            vec![]
        }
        Rule::EOI => {
            vec![]
        }
        _ => unreachable!("{:?}", pair.as_rule()),
    }
}

fn parse_inner_pairs(pair: Pair<Rule>, context: &mut ParseContext) -> Vec<Import> {
    pair.into_inner().fold(vec![], |mut imports, inner_pair| {
        imports.extend(parse_pair(inner_pair, context));
        imports
    })
}

fn parse_simple_import_statement(pair: Pair<Rule>, context: &mut ParseContext) -> Vec<Import> {
    let (line_number, _) = pair.line_col();

    pair.into_inner()
        .flat_map(|inner_pair| match inner_pair.as_rule() {
            Rule::MODULE => {
                let imported_object = inner_pair.as_str().to_owned();
                Some(Import {
                    imported_object,
                    line_number,
                    typechecking_only: context.typechecking_only,
                })
            }
            Rule::AS_IDENTIFIER => None,
            _ => unreachable!("{:?}", inner_pair.as_rule()),
        })
        .collect()
}

fn parse_from_import_statement(pair: Pair<Rule>, context: &mut ParseContext) -> Vec<Import> {
    let (line_number, _) = pair.line_col();
    let mut inner_pairs = pair.into_inner();
    let imported_base = {
        let mut imported_base = inner_pairs.next().unwrap().as_str();
        if imported_base.ends_with(".") {
            imported_base = imported_base.strip_suffix(".").unwrap();
        }
        imported_base
    };

    inner_pairs
        .filter_map(|inner_pair| match inner_pair.as_rule() {
            Rule::IDENTIFIER => {
                let imported_object = format!("{}.{}", imported_base, inner_pair.as_str());
                Some(Import {
                    imported_object,
                    line_number,
                    typechecking_only: context.typechecking_only,
                })
            }
            Rule::AS_IDENTIFIER => None,
            _ => unreachable!("{:?}", inner_pair.as_rule()),
        })
        .collect()
}

fn parse_wildcard_from_import_statement(
    pair: Pair<Rule>,
    context: &mut ParseContext,
) -> Vec<Import> {
    let (line_number, _) = pair.line_col();
    let mut inner_pairs = pair.into_inner();
    let mut imported_l = inner_pairs.next().unwrap().as_str();
    if imported_l.ends_with(".") {
        imported_l = imported_l.strip_suffix(".").unwrap();
    }
    let imported = format!("{}.*", imported_l);
    vec![Import {
        imported_object: imported.to_string(),
        line_number,
        typechecking_only: context.typechecking_only,
    }]
}

#[cfg(test)]
mod tests {
    use super::{Import, parse_imports};
    use parameterized::parameterized;

    struct ParseTestCase<'a> {
        code: &'a str,
        expected_imports: &'a [Import],
    }

    #[parameterized(case = {
        ParseTestCase {
            code: "",
            expected_imports: &[],
        },
        ParseTestCase {
            code: "import foo",
            expected_imports: &[Import::new("foo", 1, false)],
        },
        ParseTestCase {
            code: "import foo_bar",
            expected_imports: &[Import::new("foo_bar", 1, false)],
        },
        ParseTestCase {
            code: "import foo.bar",
            expected_imports: &[Import::new("foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "import foo as foofoo",
            expected_imports: &[Import::new("foo", 1, false)],
        },
        ParseTestCase {
            code: "import foo, bar",
            expected_imports: &[
                Import::new("foo", 1, false),
                Import::new("bar", 1, false)
            ],
        },
        ParseTestCase {
            code: "import foo; import bar",
            expected_imports: &[
                Import::new("foo", 1, false),
                Import::new("bar", 1, false)
            ],
        },
        ParseTestCase {
            code: "import foo; import bar;",
            expected_imports: &[
                Import::new("foo", 1, false),
                Import::new("bar", 1, false)
            ],
        },
        ParseTestCase {
            code: "
import a
import b.c",
            expected_imports: &[
                Import::new("a", 2, false),
                Import::new("b.c", 3, false)
            ],
        },
        ParseTestCase {
            code: "from foo import bar",
            expected_imports: &[Import::new("foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "from foo import bar as barbar",
            expected_imports: &[Import::new("foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "from .foo import bar",
            expected_imports: &[Import::new(".foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "from ..foo import bar",
            expected_imports: &[Import::new("..foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "from . import foo",
            expected_imports: &[Import::new(".foo", 1, false)],
        },
        ParseTestCase {
            code: "from .. import foo",
            expected_imports: &[Import::new("..foo", 1, false)],
        },
        ParseTestCase {
            code: "import foo; from bar import baz",
            expected_imports: &[
                Import::new("foo", 1, false),
                Import::new("bar.baz", 1, false)
            ],
        },
        ParseTestCase {
            code: "from foo import *",
            expected_imports: &[Import::new("foo.*", 1, false)],
        },
        ParseTestCase {
            code: "from . import *",
            expected_imports: &[Import::new(".*", 1, false)],
        },
        ParseTestCase {
            code: "from .. import *",
            expected_imports: &[Import::new("..*", 1, false)],
        },
        ParseTestCase {
            code: "from foo import bar, baz",
            expected_imports: &[
                Import::new("foo.bar", 1, false),
                Import::new("foo.baz", 1, false)
            ],
        },
        ParseTestCase {
            code: "from foo import (bar)",
            expected_imports: &[Import::new("foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "from foo import (bar,)",
            expected_imports: &[Import::new("foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "from foo import (bar, baz)",
            expected_imports: &[
                Import::new("foo.bar", 1, false),
                Import::new("foo.baz", 1, false)
            ],
        },
        ParseTestCase {
            code: "from foo import (bar, baz,)",
            expected_imports: &[
                Import::new("foo.bar", 1, false),
                Import::new("foo.baz", 1, false)
            ],
        },
        ParseTestCase {
            code: "
from foo import (
    bar, baz
)",
            expected_imports: &[Import::new("foo.bar", 2, false), Import::new("foo.baz", 2, false)],
        },
        ParseTestCase {
            code: r"from \
    foo \
    import \
    bar",
            expected_imports: &[Import::new("foo.bar", 1, false)],
        },
        ParseTestCase {
            code: "
import typing
if typing.TYPE_CHECKING:
    import foo
import bar",
            expected_imports: &[
    Import::new("typing", 2, false),
    Import::new("foo", 4, true),
    Import::new("bar", 5, false),
],
        },
        ParseTestCase {
            code: "
import typing
if typing.TYPE_CHECKING:
    import foo",
            expected_imports: &[
    Import::new("typing", 2, false),
    Import::new("foo", 4, true),
],
        },
        ParseTestCase {
            code: "
import typing
if typing.TYPE_CHECKING:
    print(\"hello\")
    import foo",
            expected_imports: &[
    Import::new("typing", 2, false),
    Import::new("foo", 5, true),
],
        },
        ParseTestCase {
            code: "
import typing

if typing.TYPE_CHECKING:

    import foo

import bar",
            expected_imports: &[
    Import::new("typing", 2, false),
    Import::new("foo", 6, true),
    Import::new("bar", 8, false),
],
        },
        ParseTestCase {
            code: "import foo  # hello",
            expected_imports: &[Import::new("foo", 1, false)],
        },
        ParseTestCase {
            code: r#"
"""
import foo
"""
import bar"#,
            expected_imports: &[Import::new("bar", 5, false)],
        },
        ParseTestCase {
            code: r#"
if TYPE_CHECKING: # Only for typechecking
    import foo"#,
            expected_imports: &[Import::new("foo", 3, true)],
        },
        ParseTestCase {
            code: r#"
if TYPE_CHECKING: import foo  # comment
import bar"#,
            expected_imports: &[
                Import::new("foo", 2, true),
                Import::new("bar", 3, false)
            ],
        },
        ParseTestCase {
            code: r#"
def f():
    import foo"#,
            expected_imports: &[
                Import::new("foo", 3, false),
            ],
        },
    })]
    fn test_parse(case: ParseTestCase) {
        let result = parse_imports(case.code);
        pretty_assertions::assert_eq!(Ok(case.expected_imports.to_vec()), result);
    }
}
