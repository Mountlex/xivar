pub mod dblp;

pub trait RequestString {
    fn query_url(&self) -> String;
}
