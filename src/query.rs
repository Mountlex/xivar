pub struct Query<'a> {
    pub terms: Option<&'a [String]>,
    pub max_hits: u32,
}

impl<'a> Query<'a> {
    pub fn builder() -> QueryBuilder<'a> {
        QueryBuilder::new()
    }
}

pub struct QueryBuilder<'a> {
    terms: Option<&'a [String]>,
    max_hits: Option<u32>,
}

impl<'a> QueryBuilder<'a> {
    pub fn new() -> QueryBuilder<'a> {
        QueryBuilder {
            terms: None,
            max_hits: None,
        }
    }
    pub fn terms(mut self, terms: &'a [String]) -> QueryBuilder<'a> {
        self.terms = Some(terms);
        self
    }
    pub fn max_hits(mut self, max_hits: u32) -> QueryBuilder<'a> {
        self.max_hits = Some(max_hits);
        self
    }
    pub fn build(self) -> Query<'a> {
        Query {
            terms: self.terms,
            max_hits: self.max_hits.unwrap_or_else(|| 50),
        }
    }
}
