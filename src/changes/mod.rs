pub(crate) mod propagation;

#[derive(Clone)]
pub(crate) enum ArgsChange {
    Pushed(String),
    Moved(Vec<usize>),
    Removed(usize),
}
