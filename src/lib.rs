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
    pub code: String,
    pub typechecking_only: bool,
}

impl Import {
    pub fn new(
        imported_object: &str,
        line_number: usize,
        import_statement: &str,
        typechecking_only: bool,
    ) -> Self {
        Self {
            imported_object: imported_object.to_string(),
            line_number,
            code: import_statement.to_string(),
            typechecking_only,
        }
    }
}

pub fn parse_imports(code: &str) -> Result<Vec<Import>, String> {
    let mut imports = vec![];

    let parsed_code = GrammarParser::parse(Rule::CODE, code)
        .map_err(|e| format!("failed to parse: {}", e))?
        .next()
        .unwrap();

    for pair in parsed_code.into_inner() {
        match pair.as_rule() {
            Rule::IMPORT_STATEMENT
            | Rule::FROM_IMPORT_STATEMENT
            | Rule::MULTILINE_FROM_IMPORT_STATEMENT
            | Rule::WILDCARD_FROM_IMPORT_STATEMENT => {
                imports.extend(parse_import_statement(pair, false));
            }
            Rule::IF_TYPECHECKING => {
                for inner_pair in pair.into_inner() {
                    for import_statement in inner_pair.into_inner() {
                        imports.extend(parse_import_statement(import_statement, true));
                    }
                }
            }
            Rule::EOI => {}
            _ => unreachable!(),
        }
    }

    Ok(imports)
}

fn parse_import_statement(pair: Pair<Rule>, typechecking_only: bool) -> Vec<Import> {
    let mut imports = vec![];

    match pair.as_rule() {
        Rule::IMPORT_STATEMENT => {
            let (line_number, _) = pair.line_col();
            let code = pair.as_str();
            let imported_object = pair.into_inner().next().unwrap().as_str().to_string();
            imports.push(Import {
                imported_object,
                line_number,
                code: code.to_string(),
                typechecking_only,
            });
        }
        Rule::FROM_IMPORT_STATEMENT | Rule::MULTILINE_FROM_IMPORT_STATEMENT => {
            let (line_number, _) = pair.line_col();
            let code = pair.as_str();
            let mut inner_pairs = pair.into_inner();
            let imported_base = {
                let mut imported_base = inner_pairs.next().unwrap().as_str();
                if imported_base.ends_with(".") {
                    imported_base = imported_base.strip_suffix(".").unwrap();
                }
                imported_base
            };
            for inner_pair in inner_pairs {
                match inner_pair.as_rule() {
                    Rule::IDENTIFIER => {
                        let imported_object = format!("{}.{}", imported_base, inner_pair.as_str());
                        imports.push(Import {
                            imported_object,
                            line_number,
                            code: code.to_string(),
                            typechecking_only,
                        });
                    }
                    Rule::AS_IDENTIFIER => {}
                    _ => unreachable!(),
                }
            }
        }
        Rule::WILDCARD_FROM_IMPORT_STATEMENT => {
            let import_statement = pair.as_str().to_string();
            let (line_number, _) = pair.line_col();
            let mut inner_pairs = pair.into_inner();
            let mut imported_l = inner_pairs.next().unwrap().as_str();
            if imported_l.ends_with(".") {
                imported_l = imported_l.strip_suffix(".").unwrap();
            }
            let imported = format!("{}.*", imported_l);
            imports.push(Import {
                imported_object: imported.to_string(),
                line_number,
                code: import_statement,
                typechecking_only,
            });
        }
        _ => unreachable!(),
    }

    imports
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
            expected_imports: &[Import::new("foo", 1, "import foo", false)],
        },
        ParseTestCase {
            code: "import foo_bar",
            expected_imports: &[Import::new("foo_bar", 1, "import foo_bar", false)],
        },
        ParseTestCase {
            code: "import foo.bar",
            expected_imports: &[Import::new("foo.bar", 1, "import foo.bar", false)],
        },
        ParseTestCase {
            code: "import foo as foofoo",
            expected_imports: &[Import::new("foo", 1, "import foo as foofoo", false)],
        },
        ParseTestCase {
            code: "import foo; import bar",
            expected_imports: &[
                Import::new("foo", 1, "import foo", false),
                Import::new("bar", 1, "import bar", false)
            ],
        },
        ParseTestCase {
            code: "import foo; import bar;",
            expected_imports: &[
                Import::new("foo", 1, "import foo", false),
                Import::new("bar", 1, "import bar", false)
            ],
        },
        ParseTestCase {
            code: "
import a
import b.c",
            expected_imports: &[
                Import::new("a", 2, "import a", false),
                Import::new("b.c", 3, "import b.c", false)
            ],
        },
        ParseTestCase {
            code: "from foo import bar",
            expected_imports: &[Import::new("foo.bar", 1, "from foo import bar", false)],
        },
        ParseTestCase {
            code: "from foo import bar as barbar",
            expected_imports: &[Import::new("foo.bar", 1, "from foo import bar as barbar", false)],
        },
        ParseTestCase {
            code: "from .foo import bar",
            expected_imports: &[Import::new(".foo.bar", 1, "from .foo import bar", false)],
        },
        ParseTestCase {
            code: "from ..foo import bar",
            expected_imports: &[Import::new("..foo.bar", 1, "from ..foo import bar", false)],
        },
        ParseTestCase {
            code: "from . import foo",
            expected_imports: &[Import::new(".foo", 1, "from . import foo", false)],
        },
        ParseTestCase {
            code: "from .. import foo",
            expected_imports: &[Import::new("..foo", 1, "from .. import foo", false)],
        },
        ParseTestCase {
            code: "import foo; from bar import baz",
            expected_imports: &[
                Import::new("foo", 1, "import foo", false),
                Import::new("bar.baz", 1, "from bar import baz", false)
            ],
        },
        ParseTestCase {
            code: "from foo import *",
            expected_imports: &[Import::new("foo.*", 1, "from foo import *", false)],
        },
        ParseTestCase {
            code: "from . import *",
            expected_imports: &[Import::new(".*", 1, "from . import *", false)],
        },
        ParseTestCase {
            code: "from .. import *",
            expected_imports: &[Import::new("..*", 1, "from .. import *", false)],
        },
        ParseTestCase {
            code: "from foo import bar, baz",
            expected_imports: &[
                Import::new("foo.bar", 1, "from foo import bar, baz", false),
                Import::new("foo.baz", 1, "from foo import bar, baz", false)
            ],
        },
        ParseTestCase {
            code: "from foo import (bar)",
            expected_imports: &[Import::new("foo.bar", 1, "from foo import (bar)", false)],
        },
        ParseTestCase {
            code: "from foo import (bar,)",
            expected_imports: &[Import::new("foo.bar", 1, "from foo import (bar,)", false)],
        },
        ParseTestCase {
            code: "from foo import (bar, baz)",
            expected_imports: &[
                Import::new("foo.bar", 1, "from foo import (bar, baz)", false),
                Import::new("foo.baz", 1, "from foo import (bar, baz)", false)
            ],
        },
        ParseTestCase {
            code: "from foo import (bar, baz,)",
            expected_imports: &[
                Import::new("foo.bar", 1, "from foo import (bar, baz,)", false),
                Import::new("foo.baz", 1, "from foo import (bar, baz,)", false)
            ],
        },
        ParseTestCase {
            code: "
from foo import (
    bar, baz
)",
            expected_imports: &[Import::new("foo.bar", 2, "from foo import (
    bar, baz
)", false), Import::new("foo.baz", 2, "from foo import (
    bar, baz
)", false)],
        },
        ParseTestCase {
            code: "
from foo import (
    bar
    ,
    baz
    ,
)",
            expected_imports: &[
Import::new("foo.bar", 2, "from foo import (
    bar
    ,
    baz
    ,
)", false),
Import::new("foo.baz", 2, "from foo import (
    bar
    ,
    baz
    ,
)", false)],
        },
        ParseTestCase {
            code: r"from \
    foo \
    import \
    bar",
            expected_imports: &[Import::new("foo.bar", 1, r"from \
    foo \
    import \
    bar", false)],
        },
        ParseTestCase {
            code: "
import typing
if typing.TYPE_CHECKING:
    import foo
import bar",
            expected_imports: &[
    Import::new("typing", 2, "import typing", false),
    Import::new("foo", 4, "import foo", true),
    Import::new("bar", 5, "import bar", false),
],
        },
        ParseTestCase {
            code: "
import typing

if typing.TYPE_CHECKING:

    import foo

import bar",
            expected_imports: &[
    Import::new("typing", 2, "import typing", false),
    Import::new("foo", 6, "import foo", true),
    Import::new("bar", 8, "import bar", false),
],
        },
    })]
    fn test_parse(case: ParseTestCase) {
        let result = parse_imports(case.code);
        pretty_assertions::assert_eq!(Ok(case.expected_imports.to_vec()), result);
    }
}
