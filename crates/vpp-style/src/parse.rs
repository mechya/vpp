//! CSS text to a [`StyleSheet`]. `cssparser` does the tokenising and the rule
//! and declaration structure, following the CSS Syntax specification; this file
//! turns what it finds into VPP's supported subset.
//!
//! Anything unsupported is skipped, never fatal, as browsers do: at-rules,
//! unsupported selectors (dropped from their selector list; a rule with none
//! left is dropped), and invalid declarations.

use cssparser::{
    AtRuleParser, CowRcStr, DeclarationParser, Delimiter, ParseError, Parser, ParserState,
    QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token,
    parse_important,
};

use crate::selector::{Combinator, CompoundSelector, Selector};
use crate::stylesheet::{Declaration, Rule, StyleSheet};

/// Parses a stylesheet.
pub fn parse_stylesheet(css: &str) -> StyleSheet {
    let mut parser = Parser::new(css);
    let rules = StyleSheetParser::new(&mut parser, &mut TopLevel)
        .filter_map(Result::ok)
        .flatten()
        .collect();
    StyleSheet { rules }
}

/// Parses the declarations of a `style=""` attribute or a rule body.
pub fn parse_declarations(text: &str) -> Vec<Declaration> {
    let mut parser = Parser::new(text);
    RuleBodyParser::new(&mut parser, &mut Declarations)
        .filter_map(Result::ok)
        .collect()
}

/// Parses rules at the top level of a stylesheet. At-rules are skipped.
struct TopLevel;

impl<'i> QualifiedRuleParser<'i> for TopLevel {
    type Prelude = Vec<Selector>;
    type QualifiedRule = Option<Rule>;
    type Error = ();

    fn parse_prelude(&mut self, input: &mut Parser<'i>) -> Result<Vec<Selector>, ParseError<()>> {
        let selectors = input.parse_comma_separated_ignoring_errors(parse_selector);
        if selectors.is_empty() {
            return Err(ParseError::custom(()));
        }
        Ok(selectors)
    }

    fn parse_block(
        &mut self,
        selectors: Vec<Selector>,
        _start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<Option<Rule>, ParseError<()>> {
        let declarations: Vec<_> = RuleBodyParser::new(input, &mut Declarations)
            .filter_map(Result::ok)
            .collect();
        // A rule with nothing to apply is dropped, as in the C++ parser.
        Ok((!declarations.is_empty()).then_some(Rule {
            selectors,
            declarations,
        }))
    }
}

impl<'i> AtRuleParser<'i> for TopLevel {
    type Prelude = ();
    type AtRule = Option<Rule>;
    type Error = ();
}

/// Parses the declarations inside a rule body or a `style` attribute.
struct Declarations;

impl<'i> DeclarationParser<'i> for Declarations {
    type Declaration = Declaration;
    type Error = ();

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
        _start: &ParserState,
    ) -> Result<Declaration, ParseError<()>> {
        let value = input.parse_until_before(Delimiter::Bang, read_value)?;
        let important = input.try_parse(parse_important).is_ok();
        input.expect_exhausted()?;
        if value.is_empty() {
            return Err(ParseError::custom(()));
        }
        let property = if name.starts_with("--") {
            name.to_string()
        } else {
            name.to_ascii_lowercase()
        };
        Ok(Declaration {
            property,
            value,
            important,
        })
    }
}

impl<'i> QualifiedRuleParser<'i> for Declarations {
    type Prelude = ();
    type QualifiedRule = Declaration;
    type Error = ();
}

impl<'i> AtRuleParser<'i> for Declarations {
    type Prelude = ();
    type AtRule = Declaration;
    type Error = ();
}

impl<'i> RuleBodyItemParser<'i, Declaration, ()> for Declarations {
    fn parse_declarations(&self) -> bool {
        true
    }

    // Nested rules (CSS Nesting) are not supported yet.
    fn parse_qualified(&self) -> bool {
        false
    }
}

/// The value text as written, trimmed, with each comment replaced by a space.
fn read_value<'i>(input: &mut Parser<'i>) -> Result<String, ParseError<()>> {
    let mut value = String::new();
    loop {
        let start = input.position();
        let Ok(token) = input.next_including_whitespace_and_comments() else {
            break;
        };
        let has_block = matches!(
            token,
            Token::Function(_)
                | Token::ParenthesisBlock
                | Token::SquareBracketBlock
                | Token::CurlyBracketBlock
        );
        if matches!(token, Token::Comment(_)) {
            value.push(' ');
            continue;
        }
        if has_block {
            input.parse_nested_block(|block| {
                while block.next_including_whitespace_and_comments().is_ok() {}
                Ok::<_, ParseError<()>>(())
            })?;
        }
        value.push_str(input.slice_from(start));
    }
    Ok(value.trim().to_owned())
}

/// Parses one selector of a selector list, or fails if it uses anything unsupported.
fn parse_selector<'i>(input: &mut Parser<'i>) -> Result<Selector, ParseError<()>> {
    let mut compounds = Vec::new();
    let mut combinators = Vec::new();
    let mut current: Option<CompoundSelector> = None;
    let mut pending: Option<Combinator> = None;

    while let Ok(token) = input.next_including_whitespace() {
        let token = token.clone();
        match token {
            Token::WhiteSpace(_) => {
                if current.is_some() && pending.is_none() {
                    pending = Some(Combinator::Descendant);
                }
                continue;
            }
            Token::Delim('>') => {
                if current.is_none() {
                    return Err(ParseError::custom(()));
                }
                pending = Some(Combinator::Child);
                continue;
            }
            _ => {}
        }

        if let Some(combinator) = pending.take() {
            compounds.push(current.take().expect("a combinator follows a compound"));
            combinators.push(combinator);
        }
        let compound = current.get_or_insert_with(CompoundSelector::default);
        match token {
            Token::Delim('*') => {}
            Token::Ident(tag) if compound.tag.is_none() => {
                compound.tag = Some(tag.to_ascii_lowercase())
            }
            Token::IDHash(id) if compound.id.is_none() => compound.id = Some(id.to_string()),
            Token::Delim('.') => match input.next_including_whitespace()? {
                Token::Ident(class) => compound.classes.push(class.to_string()),
                _ => return Err(ParseError::custom(())),
            },
            // Pseudo-classes, attribute selectors, sibling combinators, and anything else.
            _ => return Err(ParseError::custom(())),
        }
    }

    // A selector cannot end in a combinator other than trailing whitespace.
    let (Some(last), None | Some(Combinator::Descendant)) = (current, pending) else {
        return Err(ParseError::custom(()));
    };
    compounds.push(last);
    Ok(Selector {
        compounds,
        combinators,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration(property: &str, value: &str, important: bool) -> Declaration {
        Declaration {
            property: property.into(),
            value: value.into(),
            important,
        }
    }

    #[test]
    fn parses_rules_selectors_and_declarations() {
        let sheet = parse_stylesheet(".card > p, h1 { color: red; margin: 0 auto !important }");
        assert_eq!(sheet.rules.len(), 1);
        let rule = &sheet.rules[0];
        assert_eq!(rule.selectors.len(), 2);
        assert_eq!(rule.selectors[0].combinators, [Combinator::Child]);
        assert_eq!(rule.selectors[0].compounds[0].classes, ["card"]);
        assert_eq!(rule.selectors[0].compounds[1].tag.as_deref(), Some("p"));
        assert_eq!(
            rule.declarations,
            [
                declaration("color", "red", false),
                declaration("margin", "0 auto", true)
            ]
        );
    }

    #[test]
    fn keeps_values_as_written_without_comments() {
        let decls = parse_declarations(
            "font-family: \"Segoe UI\", sans-serif; background: url(a.png) /* note */ no-repeat; COLOR: rgb(1, 2, 3)",
        );
        assert_eq!(
            decls,
            [
                declaration("font-family", "\"Segoe UI\", sans-serif", false),
                declaration("background", "url(a.png)   no-repeat", false),
                declaration("color", "rgb(1, 2, 3)", false),
            ]
        );
    }

    #[test]
    fn skips_at_rules_comments_and_invalid_declarations() {
        let sheet = parse_stylesheet(
            "@import url(x.css); /* comment */ @media (min-width: 1px) { p { color: blue } }\n\
             p { color: ; margin: 1px ! bogus; padding: 2px; : 3px } div { }",
        );
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(
            sheet.rules[0].declarations,
            [declaration("padding", "2px", false)]
        );
    }

    #[test]
    fn drops_unsupported_selectors_but_keeps_the_rest_of_the_list() {
        let sheet =
            parse_stylesheet("a:hover, [x], p + p, p ~ p, b { color: red } i:hover { color: red }");
        assert_eq!(sheet.rules.len(), 1);
        let selectors = &sheet.rules[0].selectors;
        assert_eq!(selectors.len(), 1);
        assert_eq!(selectors[0].compounds[0].tag.as_deref(), Some("b"));
    }

    #[test]
    fn tag_names_are_lower_cased_classes_and_ids_are_not() {
        let sheet = parse_stylesheet("DIV.Big#Go { color: red }");
        let compound = &sheet.rules[0].selectors[0].compounds[0];
        assert_eq!(compound.tag.as_deref(), Some("div"));
        assert_eq!(compound.classes, ["Big"]);
        assert_eq!(compound.id.as_deref(), Some("Go"));
    }

    #[test]
    fn combinators_with_and_without_spaces() {
        for text in ["ul>li", "ul > li", "ul >li"] {
            let sheet = parse_stylesheet(&format!("{text} {{ color: red }}"));
            assert_eq!(
                sheet.rules[0].selectors[0].combinators,
                [Combinator::Child],
                "{text}"
            );
        }
        let sheet = parse_stylesheet("  a   b  { color: red }");
        assert_eq!(
            sheet.rules[0].selectors[0].combinators,
            [Combinator::Descendant]
        );
    }

    #[test]
    fn malformed_selectors_are_dropped() {
        for text in ["> p", "p >", ".", "#1", "p..a", "a b >"] {
            assert!(
                parse_stylesheet(&format!("{text} {{ color: red }}")).is_empty(),
                "{text}"
            );
        }
    }

    #[test]
    fn important_is_case_insensitive_and_custom_properties_keep_case() {
        assert_eq!(
            parse_declarations("--Brand: #2563eb; Color: red !IMPORTANT"),
            [
                declaration("--Brand", "#2563eb", false),
                declaration("color", "red", true)
            ]
        );
    }
}
