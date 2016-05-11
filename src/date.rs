use ::chrono;
use ::regex;
use std::error;

pub fn parse_date(format: &str, input: &str) -> Result<chrono::NaiveDate, Box<error::Error>> {
    let compile_re = regex::Regex::new("%.").unwrap();

    let pattern = compile_re.replace_all(&regex::quote(format), |captures: &regex::Captures| {
        match captures.at(0) {
                Some("%Y") | Some("%m") | Some("%d") => r"\d+",
                Some("%b") | Some("%B") => r"\w+",
                _ => r"\S+",
            }
            .to_string()
    });
    let re_pattern = regex::Regex::new(&pattern)?;
    let date = re_pattern.captures(input).ok_or("No date found")?.at(0).unwrap();
    Ok(chrono::NaiveDate::parse_from_str(date, format)?)
}

#[cfg(test)]
mod tests {
    use super::parse_date;
    use ::chrono::NaiveDate;

    #[test]
    fn test_parse_date() {
        let date = parse_date("%Y-%m-%d", "2015-10-10").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(2015, 10, 10));
    }

    #[test]
    fn test_parse_date_locale() {
        let date = parse_date("%Y %b %d", "2015 Jul 10").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(2015, 7, 10));
    }

    #[test]
    fn test_parse_date_with_surrounding_content() {
        let date = parse_date("%Y-%m-%d", "blah 2015-10-10").unwrap();
        assert_eq!(date, NaiveDate::from_ymd(2015, 10, 10));
    }
}
