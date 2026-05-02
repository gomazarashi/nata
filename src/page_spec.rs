use std::str::FromStr;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageSpec {
    items: Vec<PageExpr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PageExpr {
    Number(u32),
    Range(PageBound, PageBound),
    All,
    Last,
    Odd,
    Even,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PageBound {
    Number(u32),
    Last,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PageSpecRules {
    pub allow_odd_even: bool,
    pub require_single_page: bool,
    pub allow_empty_result: bool,
}

impl PageSpec {
    pub fn parse(input: &str) -> Result<Self, AppError> {
        input.parse()
    }

    pub fn resolve(&self, total_pages: u32) -> Result<Vec<u32>, AppError> {
        let mut resolved = Vec::new();

        for item in &self.items {
            match item {
                PageExpr::Number(page) => resolved.push(validate_page(*page, total_pages)?),
                PageExpr::Range(start, end) => {
                    let start = resolve_bound(start, total_pages)?;
                    let end = resolve_bound(end, total_pages)?;
                    if start > end {
                        return Err(AppError::InvalidPageSpec(format!("{start}-{end}")));
                    }
                    resolved.extend(start..=end);
                }
                PageExpr::All => resolved.extend(1..=total_pages),
                PageExpr::Last => resolved.push(validate_page(total_pages, total_pages)?),
                PageExpr::Odd => resolved.extend((1..=total_pages).filter(|page| page % 2 == 1)),
                PageExpr::Even => resolved.extend((1..=total_pages).filter(|page| page % 2 == 0)),
            }
        }

        Ok(resolved)
    }

    pub fn validate_rules(&self, total_pages: u32, rules: PageSpecRules) -> Result<Vec<u32>, AppError> {
        if !rules.allow_odd_even
            && self
                .items
                .iter()
                .any(|item| matches!(item, PageExpr::Odd | PageExpr::Even))
        {
            return Err(AppError::OddEvenNotAllowed);
        }

        let resolved = self.resolve(total_pages)?;

        if !rules.allow_empty_result && resolved.is_empty() {
            return Err(AppError::EmptyPageSelection);
        }

        if rules.require_single_page && resolved.len() != 1 {
            return Err(AppError::SinglePageRequired);
        }

        Ok(resolved)
    }

    pub fn to_qpdf_range(&self) -> String {
        self.items
            .iter()
            .map(PageExpr::to_qpdf_range)
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl PageExpr {
    fn to_qpdf_range(&self) -> String {
        match self {
            Self::Number(page) => page.to_string(),
            Self::Range(start, end) => format!("{}-{}", start.to_qpdf_range(), end.to_qpdf_range()),
            Self::All => "1-z".into(),
            Self::Last => "z".into(),
            Self::Odd => "1-z:odd".into(),
            Self::Even => "1-z:even".into(),
        }
    }
}

impl PageBound {
    fn to_qpdf_range(&self) -> String {
        match self {
            Self::Number(page) => page.to_string(),
            Self::Last => "z".into(),
        }
    }
}

impl FromStr for PageSpec {
    type Err = AppError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(AppError::InvalidPageSpec("page specification is empty".into()));
        }

        let items = trimmed
            .split(',')
            .map(parse_item)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { items })
    }
}

fn parse_item(input: &str) -> Result<PageExpr, AppError> {
    let token = input.trim();
    if token.is_empty() {
        return Err(AppError::InvalidPageSpec("empty page token".into()));
    }

    match token {
        "all" => return Ok(PageExpr::All),
        "last" => return Ok(PageExpr::Last),
        "odd" => return Ok(PageExpr::Odd),
        "even" => return Ok(PageExpr::Even),
        _ => {}
    }

    if let Some((start, end)) = token.split_once('-') {
        let start = parse_bound(start.trim())?;
        let end = parse_bound(end.trim())?;

        if matches!(start, PageBound::Last) {
            return Err(AppError::InvalidPageSpec(token.into()));
        }

        if let (PageBound::Number(start), PageBound::Number(end)) = (&start, &end)
            && start > end
        {
            return Err(AppError::InvalidPageSpec(token.into()));
        }

        return Ok(PageExpr::Range(start, end));
    }

    Ok(PageExpr::Number(parse_page_number(token)?))
}

fn parse_bound(input: &str) -> Result<PageBound, AppError> {
    if input == "last" {
        Ok(PageBound::Last)
    } else {
        Ok(PageBound::Number(parse_page_number(input)?))
    }
}

fn parse_page_number(input: &str) -> Result<u32, AppError> {
    let page = input
        .parse::<u32>()
        .map_err(|_| AppError::InvalidPageSpec(input.into()))?;

    if page == 0 {
        return Err(AppError::InvalidPageSpec(input.into()));
    }

    Ok(page)
}

fn resolve_bound(bound: &PageBound, total_pages: u32) -> Result<u32, AppError> {
    match bound {
        PageBound::Number(page) => validate_page(*page, total_pages),
        PageBound::Last => validate_page(total_pages, total_pages),
    }
}

fn validate_page(page: u32, total_pages: u32) -> Result<u32, AppError> {
    if total_pages == 0 || page == 0 || page > total_pages {
        return Err(AppError::PageOutOfRange { page, total_pages });
    }

    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::{PageSpec, PageSpecRules};
    use crate::error::AppError;

    #[test]
    fn accepts_supported_tokens() {
        for input in ["1", "1-5", "1,3,5-8", "all", "last", "odd", "even", "4-last"] {
            PageSpec::parse(input).expect("page spec should parse");
        }
    }

    #[test]
    fn rejects_invalid_tokens() {
        for input in ["", "0", "-1", "5-1", "last-4", "last-1", "abc", "1,,2"] {
            assert!(matches!(PageSpec::parse(input), Err(AppError::InvalidPageSpec(_))));
        }
    }

    #[test]
    fn resolves_ranges_in_order_and_keeps_duplicates() {
        let spec = PageSpec::parse("3,1,2,2,4-last").expect("spec should parse");
        let pages = spec.resolve(5).expect("spec should resolve");
        assert_eq!(pages, vec![3, 1, 2, 2, 4, 5]);
    }

    #[test]
    fn resolves_special_tokens() {
        let all = PageSpec::parse("all").expect("spec should parse");
        assert_eq!(all.resolve(4).expect("all should resolve"), vec![1, 2, 3, 4]);

        let odd = PageSpec::parse("odd").expect("spec should parse");
        assert_eq!(odd.resolve(6).expect("odd should resolve"), vec![1, 3, 5]);

        let even = PageSpec::parse("even").expect("spec should parse");
        assert_eq!(even.resolve(6).expect("even should resolve"), vec![2, 4, 6]);

        let last = PageSpec::parse("last").expect("spec should parse");
        assert_eq!(last.resolve(6).expect("last should resolve"), vec![6]);
    }

    #[test]
    fn rejects_out_of_range_pages_on_resolution() {
        let spec = PageSpec::parse("4-last").expect("spec should parse");
        assert!(matches!(
            spec.resolve(3),
            Err(AppError::PageOutOfRange {
                page: 4,
                total_pages: 3
            })
        ));
    }

    #[test]
    fn rules_can_reject_odd_even() {
        let spec = PageSpec::parse("odd").expect("spec should parse");
        assert!(matches!(
            spec.validate_rules(5, PageSpecRules::default()),
            Err(AppError::OddEvenNotAllowed)
        ));
    }

    #[test]
    fn rules_can_require_single_page() {
        let spec = PageSpec::parse("1-2").expect("spec should parse");
        assert!(matches!(
            spec.validate_rules(
                5,
                PageSpecRules {
                    allow_odd_even: true,
                    require_single_page: true,
                    allow_empty_result: false,
                }
            ),
            Err(AppError::SinglePageRequired)
        ));
    }

    #[test]
    fn converts_to_compact_qpdf_ranges() {
        let spec = PageSpec::parse("1-3,last,odd,even,4-last").expect("spec should parse");
        assert_eq!(spec.to_qpdf_range(), "1-3,z,1-z:odd,1-z:even,4-z");
    }
}
