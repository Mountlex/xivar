#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    terms: Vec<QueryTerm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QueryTerm {
    Prefix(String),
    Exact(String),
}

impl Query {
    pub fn empty() -> Query {
        Query { terms: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }
}

impl IntoIterator for Query {
    type Item = QueryTerm;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.terms.into_iter()
    }
}

impl<'a> IntoIterator for &'a Query {
    type Item = &'a QueryTerm;
    type IntoIter = std::slice::Iter<'a, QueryTerm>;

    fn into_iter(self) -> Self::IntoIter {
        self.terms.iter()
    }
}

impl From<String> for Query {
    fn from(text: String) -> Self {
        let terms = text
            .split_whitespace()
            .map(|t| {
                let prep = t.trim().to_lowercase();
                if prep.ends_with('$') {
                    let mut chars = prep.chars();
                    chars.next_back();
                    QueryTerm::Exact(chars.as_str().to_string())
                } else {
                    QueryTerm::Prefix(prep)
                }
            })
            .collect();
        Self { terms }
    }
}
