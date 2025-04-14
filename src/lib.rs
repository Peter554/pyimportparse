use pest::Parser;
use pest::iterators::Pair;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct GrammarParser;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Import {
    pub imported_object: String,
    pub line_number: u32,
    pub typechecking_only: bool,
}

impl Import {
    pub fn new(imported_object: &str, line_number: u32, typechecking_only: bool) -> Self {
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
                    line_number: line_number as u32,
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
                    line_number: line_number as u32,
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
        line_number: line_number as u32,
        typechecking_only: context.typechecking_only,
    }]
}

#[cfg(test)]
mod tests {
    use super::parse_imports;
    use parameterized::parameterized;

    #[test]
    fn test_parse_empty_string() {
        let imports = parse_imports("").unwrap();
        assert!(imports.is_empty());
    }

    fn parse_and_check(case: (&str, &[&str])) {
        let (code, expected_imports) = case;
        let imports = parse_imports(code).unwrap();
        assert_eq!(
            expected_imports,
            imports
                .into_iter()
                .map(|i| i.imported_object)
                .collect::<Vec<_>>()
        );
    }

    fn parse_and_check_with_typechecking_only(case: (&str, &[(&str, bool)])) {
        let (code, expected_imports) = case;
        let imports = parse_imports(code).unwrap();
        assert_eq!(
            expected_imports
                .iter()
                .map(|i| (i.0.to_owned(), i.1))
                .collect::<Vec<_>>(),
            imports
                .into_iter()
                .map(|i| (i.imported_object, i.typechecking_only))
                .collect::<Vec<_>>()
        );
    }

    #[parameterized(case = {
        ("import foo", &["foo"]),
        ("import foo_FOO_123", &["foo_FOO_123"]),
        ("import foo.bar", &["foo.bar"]),
        ("import foo.bar.baz", &["foo.bar.baz"]),
        ("import foo, bar, bax", &["foo", "bar", "bax"]),
        ("import foo as FOO", &["foo"]),
        ("import foo as FOO, bar as BAR", &["foo", "bar"]),
        ("import  foo  as  FOO ,  bar  as  BAR", &["foo", "bar"]),
        ("import foo # Comment", &["foo"]),
    })]
    fn test_parse_simple_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("from foo import bar", &["foo.bar"]),
        ("from foo import bar_BAR_123", &["foo.bar_BAR_123"]),
        ("from .foo import bar", &[".foo.bar"]),
        ("from ..foo import bar", &["..foo.bar"]),
        ("from . import foo", &[".foo"]),
        ("from .. import foo", &["..foo"]),
        ("from foo.bar import baz", &["foo.bar.baz"]),
        ("from .foo.bar import baz", &[".foo.bar.baz"]),
        ("from ..foo.bar import baz", &["..foo.bar.baz"]),
        ("from foo import bar, baz, bax", &["foo.bar", "foo.baz", "foo.bax"]),
        ("from foo import bar as BAR", &["foo.bar"]),
        ("from foo import bar as BAR, baz as BAZ", &["foo.bar", "foo.baz"]),
        ("from  foo  import  bar  as  BAR ,  baz  as  BAZ", &["foo.bar", "foo.baz"]),
        ("from foo import bar # Comment", &["foo.bar"]),
    })]
    fn test_parse_from_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("from foo import (bar)", &["foo.bar"]),
        ("from foo import (bar,)", &["foo.bar"]),
        ("from foo import (bar, baz)", &["foo.bar", "foo.baz"]),
        ("from foo import (bar, baz,)", &["foo.bar", "foo.baz"]),
        ("from foo import (bar as BAR, baz as BAZ,)", &["foo.bar", "foo.baz"]),
        ("from  foo  import  ( bar  as  BAR , baz  as  BAZ , )", &["foo.bar", "foo.baz"]),
        ("from foo import (bar, baz,) # Comment", &["foo.bar", "foo.baz"]),

        (r#"
from foo import (
    bar,
    baz
)
        "#, &["foo.bar", "foo.baz"]),

        (r#"
from foo import (
    bar,
    baz,
)
        "#, &["foo.bar", "foo.baz"]),

        (r#"
from foo import (
    a, b,
    c, d,
)
        "#, &["foo.a", "foo.b", "foo.c", "foo.d"]),

        // As name
        (r#"
from foo import (
    bar as BAR,
    baz as BAZ,
)
        "#, &["foo.bar", "foo.baz"]),

        // Whitespace
        (r#"
from  foo  import  (

    bar  as  BAR ,

       baz  as  BAZ ,

)
        "#, &["foo.bar", "foo.baz"]),

        // Comments
        (r#"
from foo import ( # C
    # C
    bar as BAR, # C
    # C
    baz as BAZ, # C
    # C
) # C
        "#, &["foo.bar", "foo.baz"]),
    })]
    fn test_parse_multiline_from_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("from foo import *", &["foo.*"]),
        ("from .foo import *", &[".foo.*"]),
        ("from ..foo import *", &["..foo.*"]),
        ("from . import *", &[".*"]),
        ("from .. import *", &["..*"]),
        ("from  foo  import  *", &["foo.*"]),
        ("from foo import * # Comment", &["foo.*"]),
    })]
    fn test_parse_wildcard_from_import_statement(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        ("import a; import b", &["a", "b"]),
        ("import a; import b;", &["a", "b"]),
        ("import  a ;  import  b ;", &["a", "b"]),
        ("import a; import b # Comment", &["a", "b"]),
        ("import a; from b import c; from d import (e); from f import *", &["a", "b.c", "d.e", "f.*"]),
    })]
    fn test_parse_import_statement_list(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import a, b, \
       c, d
        "#, &["a", "b", "c", "d"]),

        (r#"
from foo import a, b, \
                c, d
        "#, &["foo.a", "foo.b", "foo.c", "foo.d"]),

        (r#"
from foo \
    import *
        "#, &["foo.*"]),
    })]
    fn test_backslash_continuation(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import a
def foo():
    import b 
import c
        "#, &["a", "b", "c"]),

        (r#"
import a
class Foo:
    import b
import c
        "#, &["a", "b", "c"]),
    })]
    fn test_parse_nested_imports(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[parameterized(case = {
        (r#"
import foo
if typing.TYPE_CHECKING: import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING: import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if  TYPE_CHECKING :  import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING: import bar as BAR
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo # C
if TYPE_CHECKING: import bar # C
import baz # C
"#, &[("foo", false), ("bar", true), ("baz", false)]),
    })]
    fn test_singleline_if_typechecking(case: (&str, &[(&str, bool)])) {
        parse_and_check_with_typechecking_only(case);
    }

    #[parameterized(case = {
        (r#"
import foo
if typing.TYPE_CHECKING:
    import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING:
    import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import  foo

if  TYPE_CHECKING :

    import  bar

import  baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING:
    import bar as BAR
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo # C
if TYPE_CHECKING: # C
    # C
    import bar # C
    # C
import baz # C
"#, &[("foo", false), ("bar", true), ("baz", false)]),

        (r#"
import foo
if TYPE_CHECKING:
    """
    Comment
    """
    import bar
import baz
"#, &[("foo", false), ("bar", true), ("baz", false)]),
    })]
    fn test_multiline_if_typechecking(case: (&str, &[(&str, bool)])) {
        parse_and_check_with_typechecking_only(case);
    }

    #[parameterized(case = {
        (r#"
import foo
"""
import bar
"""
import baz
"#, &["foo", "baz"]),

        (r#"
import foo
'''
import bar
'''
import baz
"#, &["foo", "baz"]),
    })]
    fn test_multiline_strings(case: (&str, &[&str])) {
        parse_and_check(case);
    }

    #[test]
    fn test_parse_line_numbers() {
        let imports = parse_imports(
            "
import a
from b import c
from d import (e)
from f import *",
        )
        .unwrap();
        assert_eq!(
            vec![
                ("a".to_owned(), 2_u32),
                ("b.c".to_owned(), 3_u32),
                ("d.e".to_owned(), 4_u32),
                ("f.*".to_owned(), 5_u32),
            ],
            imports
                .into_iter()
                .map(|i| (i.imported_object, i.line_number))
                .collect::<Vec<_>>()
        );
    }
}
