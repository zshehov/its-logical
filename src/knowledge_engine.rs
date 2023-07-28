use std::{cell::RefCell, rc::Rc};

pub(crate) trait ConsultResult {
    fn more(&mut self) -> Vec<String>;
}

pub(crate) trait Engine {
    type ConsultResult: ConsultResult;
    fn ask(&mut self, name: &str, args: &[String]) -> Rc<RefCell<Self::ConsultResult>>;
}
