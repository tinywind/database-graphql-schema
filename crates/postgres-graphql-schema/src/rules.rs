use regex::RegexBuilder;

use crate::{Error, Result, config::TypeOverrideConfig};

#[derive(Debug)]
pub(crate) struct Rule {
    setting: &'static str,
    pattern: String,
    regex: regex::Regex,
    matched: bool,
}

impl Rule {
    fn new(setting: &'static str, pattern: &str) -> Result<Self> {
        let anchored = format!(r"\A(?:{pattern})\z");
        let regex = RegexBuilder::new(&anchored)
            .case_insensitive(true)
            .build()
            .map_err(|error| {
                Error::Generation(format!(
                    "{setting} pattern '{pattern}' is not a valid regular expression: {error}"
                ))
            })?;
        Ok(Self {
            setting,
            pattern: pattern.to_owned(),
            regex,
            matched: false,
        })
    }

    pub fn matches(&mut self, name: &str) -> bool {
        let matched = self.regex.is_match(name);
        self.matched |= matched;
        matched
    }

    fn assert_matched(&self) -> Result<()> {
        if self.matched {
            Ok(())
        } else {
            Err(Error::Generation(format!(
                "{} pattern '{}' matches nothing in the selected schema; remove it or fix the name",
                self.setting, self.pattern
            )))
        }
    }
}

#[derive(Debug)]
pub(crate) struct Rules(Vec<Rule>);

impl Rules {
    pub fn new(setting: &'static str, patterns: &[String]) -> Result<Self> {
        patterns
            .iter()
            .map(|pattern| Rule::new(setting, pattern))
            .collect::<Result<Vec<_>>>()
            .map(Self)
    }

    pub fn any_matches(&mut self, name: &str) -> bool {
        self.0
            .iter_mut()
            .map(|rule| rule.matches(name))
            .fold(false, |any, matched| any | matched)
    }

    pub fn assert_matched(&self) -> Result<()> {
        self.0.iter().try_for_each(Rule::assert_matched)
    }
}

#[derive(Debug)]
pub(crate) struct OverrideRules(Vec<(Rule, String)>);

impl OverrideRules {
    pub fn new(config: &[TypeOverrideConfig]) -> Result<Self> {
        config
            .iter()
            .map(|entry| {
                Ok((
                    Rule::new("types.override", &entry.pattern)?,
                    entry.graphql.trim().to_owned(),
                ))
            })
            .collect::<Result<Vec<_>>>()
            .map(Self)
    }

    pub fn first_match(&mut self, name: &str) -> Option<String> {
        let mut first = None;
        for (rule, graphql) in &mut self.0 {
            if rule.matches(name) && first.is_none() {
                first = Some(graphql.clone());
            }
        }
        first
    }

    pub fn assert_matched(&self) -> Result<()> {
        self.0
            .iter()
            .try_for_each(|(rule, _)| rule.assert_matched())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_are_case_insensitive_full_match_and_fail_when_stale() {
        let mut rules = Rules::new("x", &[r"User\..*".to_owned()]).unwrap();
        assert!(rules.any_matches("user.password"));
        assert!(!rules.any_matches("prefix.user.password"));
        assert!(rules.assert_matched().is_ok());

        let stale = Rules::new("x", &["missing".to_owned()]).unwrap();
        assert!(
            stale
                .assert_matched()
                .unwrap_err()
                .to_string()
                .contains("matches nothing")
        );
    }
}
