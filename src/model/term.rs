pub struct Term {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) arguments: Vec<String>,
    // gives values to the arguments Vec
    pub(crate) facts: Vec<Vec<Option<String>>>,
    pub(crate) rules: Vec<(Vec<Option<String>>, String)>,
}

impl Term {
    pub(crate) fn new(
        name: &str,
        description: &str,
        arguments: &[&str],
        facts: Vec<Vec<Option<String>>>,
        rules: Vec<(Vec<Option<String>>, String)>,
    ) -> Self {
        Self {
            name: name.to_owned(),
            description: description.to_owned(),
            arguments: arguments.iter().map(|&s| s.to_owned()).collect(),
            facts,
            rules,
        }
    }
}
