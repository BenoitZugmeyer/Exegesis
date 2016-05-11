use super::website;

use regex;
use std::fmt::Debug;
use std::error::Error;

pub trait Matcher: Debug {
    fn matches(&self, &website::Website) -> bool;
}

#[derive(Debug)]
pub struct URLMatcher {
    re: regex::Regex,
}

impl URLMatcher {
    pub fn new(pattern: &str) -> Result<Self, Box<Error>> {
        Ok(URLMatcher { re: URLMatcher::compile_pattern(pattern)? })
    }

    fn compile_pattern(pattern: &str) -> Result<regex::Regex, Box<Error>> {
        let re_pattern = format!("^{}$",
                                 &regex::quote(pattern)
                                     .replace(r"/\*\*", "(?:/.*)?")
                                     .replace(r"\*\*", ".*")
                                     .replace(r"/\*", "(?:/[^/]*)?")
                                     .replace(r"\*", "[^/]*"));

        Ok(regex::Regex::new(&re_pattern)?)
    }
}

impl Matcher for URLMatcher {
    fn matches(&self, website: &website::Website) -> bool {
        self.re.is_match(&website.request_url)
    }
}

#[cfg(test)]
mod urlmatcher_compile_pattern {
    use super::URLMatcher;

    #[test]
    fn generated_re_glob() {
        let re = URLMatcher::compile_pattern("*//coucou.com").unwrap();
        assert_eq!(re.as_str(), r"^[^/]*//coucou\.com$");
    }

    #[test]
    fn generated_re_glob_after_slash() {
        let re = URLMatcher::compile_pattern("http://coucou.com/*").unwrap();
        assert_eq!(re.as_str(), r"^http://coucou\.com(?:/[^/]*)?$");
    }

    #[test]
    fn generated_re_superglob() {
        let re = URLMatcher::compile_pattern("http://coucou.com/foo**").unwrap();
        assert_eq!(re.as_str(), r"^http://coucou\.com/foo.*$");
    }

    #[test]
    fn generated_re_superglob_after_slash() {
        let re = URLMatcher::compile_pattern("http://coucou.com/**").unwrap();
        assert_eq!(re.as_str(), r"^http://coucou\.com(?:/.*)?$");
    }

    #[test]
    fn match_plain() {
        let re = URLMatcher::compile_pattern("http://foo.com").unwrap();
        assert!(re.is_match("http://foo.com"));
        assert!(!re.is_match("https://foo.com"));
        assert!(!re.is_match("http://foo.coma"));
        assert!(!re.is_match("blah/http://foo.com"));
    }

    #[test]
    fn match_glob_protocol() {
        let re = URLMatcher::compile_pattern("*//foo.com").unwrap();
        assert!(re.is_match("http://foo.com"));
        assert!(re.is_match("https://foo.com"));
        assert!(!re.is_match("http://foo.coma"));
        assert!(!re.is_match("blah/http://foo.com"));
    }

    #[test]
    fn match_glob_path() {
        let re = URLMatcher::compile_pattern("http://foo.com/*").unwrap();
        assert!(re.is_match("http://foo.com/"));
        assert!(re.is_match("http://foo.com"));
        assert!(re.is_match("http://foo.com/a"));
        assert!(!re.is_match("http://foo.coma"));
        assert!(!re.is_match("http://foo.com/a/b"));
    }

    #[test]
    fn match_superglob_path() {
        let re = URLMatcher::compile_pattern("http://foo.com/**").unwrap();
        assert!(re.is_match("http://foo.com/"));
        assert!(re.is_match("http://foo.com"));
        assert!(re.is_match("http://foo.com/a"));
        assert!(re.is_match("http://foo.com/a/b/c"));
        assert!(!re.is_match("http://foo.coma"));
    }
}
