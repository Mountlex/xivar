#[derive(Clone)]
pub struct Query {
    pub terms: Option<Vec<String>>,
    pub max_hits: Option<u32>,
}

impl Query {
    pub fn builder() -> QueryBuilder {
        QueryBuilder::new()
    }

    pub fn empty() -> Query {
        Query {
            terms: None,
            max_hits: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        // TODO
        self.terms.is_none() || self.terms.as_ref().unwrap().is_empty()
    }
}

pub struct QueryBuilder {
    terms: Option<Vec<String>>,
    max_hits: Option<Option<u32>>,
}

impl QueryBuilder {
    pub fn new() -> QueryBuilder {
        QueryBuilder {
            terms: None,
            max_hits: None,
        }
    }
    pub fn terms(mut self, terms: Vec<String>) -> QueryBuilder {
        self.terms = Some(terms);
        self
    }
    pub fn max_hits(mut self, max_hits: Option<u32>) -> QueryBuilder {
        self.max_hits = Some(max_hits);
        self
    }
    pub fn build(self) -> Query {
        Query {
            terms: self.terms,
            max_hits: self.max_hits.unwrap_or_else(|| None),
        }
    }
}
