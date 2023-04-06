#[derive(Debug, PartialEq)]
pub enum Request<'i> {
    Get(GetRequestParams<'i>),
}

#[derive(Debug, PartialEq)]
pub struct GetRequestParams<'i> {
    pub url: &'i str,
    pub headers: Option<Vec<Header<'i>>>,
}

#[derive(Debug, PartialEq)]
pub struct Header<'i> {
    pub name: &'i str,
    pub value: &'i str,
}

#[derive(Debug, PartialEq)]
pub struct Program<'i> {
    pub requests: Vec<Request<'i>>,
}

impl<'i> Program<'i> {
    pub fn new() -> Self {
        Self { requests: vec![] }
    }
}
