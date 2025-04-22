use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{alphanumeric1, newline, not_line_ending, space0, space1};
use nom::combinator::{all_consuming, opt, recognize};
use nom::multi::{many0, many1, separated_list1};
use nom::sequence::{delimited, terminated};
use nom::{IResult, Parser};
use nom_locate::{LocatedSpan, position};

type Span<'a> = LocatedSpan<&'a str>;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Import<'a> {
    pub imported_object: &'a str,
    pub line_number: u32,
    pub typechecking_only: bool,
}

impl<'a> Import<'a> {
    pub fn new(imported_object: &'a str, line_number: u32, typechecking_only: bool) -> Self {
        Self {
            imported_object,
            line_number,
            typechecking_only,
        }
    }
}

pub fn parse_imports(s: &str) -> Result<Vec<Import>, String> {
    let s = Span::new(s);
    let (_, result) = all_consuming(many0(parse_import_statement))
        .parse(s)
        .map_err(|e| e.to_string())?;
    Ok(result.into_iter().flatten().collect())
}

fn parse_import_statement(s: Span) -> IResult<Span, Vec<Import>> {
    let (rest, result) = delimited(
        (tag("import"), space1),
        separated_list1(
            delimited(space0, tag(","), space0),
            terminated(
                (position, parse_module),
                opt((space1, tag("as"), space1, parse_identifier)),
            ),
        ),
        (opt(space0), opt(parse_comment), opt(newline)),
    )
    .parse(s)?;
    Ok((
        rest,
        result
            .into_iter()
            .map(|(span, module_name)| Import::new(module_name, span.location_line(), false))
            .collect(),
    ))
}

fn parse_module(s: Span) -> IResult<Span, &str> {
    let (rest, result) = recognize(separated_list1(tag("."), parse_identifier)).parse(s)?;
    Ok((rest, result.fragment()))
}

fn parse_identifier(s: Span) -> IResult<Span, &str> {
    let (rest, result) = recognize(many1(alt((alphanumeric1, tag("_"))))).parse(s)?;
    Ok((rest, result.fragment()))
}

fn parse_comment(s: Span) -> IResult<Span, ()> {
    let (rest, _) = (tag("#"), not_line_ending).parse(s)?;
    Ok((rest, ()))
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
}
