use super::lexer::RedirectOp;

#[derive(Debug)]
pub enum Node {
    Command(Command),
    Pipeline(Vec<Command>),
}

#[derive(Debug, Clone, Default)]
pub struct Command {
    pub program: String,
    pub arguments: Vec<String>,
    pub redirections: Vec<Redirection>,
    pub background: bool,
}

#[derive(Debug, Clone)]
pub struct Redirection {
    pub operator: RedirectOp,
    pub filename: String,
}
