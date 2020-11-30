
#[derive(Clone)]
pub enum Query<'a> {
    Full(&'a[String]),
    Title(&'a[String]),
    Author(&'a[String]),
}