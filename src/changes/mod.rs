pub(crate) mod propagation;
pub(crate) mod two_phase_commit;

#[derive(Clone)]
pub(crate) enum ArgsChange {
    Pushed(String),
    Moved(Vec<usize>),
    Removed(usize),
}
