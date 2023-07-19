pub(crate) mod propagation;

#[derive(Clone, Debug)]
pub(crate) enum ArgsChange {
    Pushed(String),
    Moved(Vec<usize>),
    Removed(usize),
}
