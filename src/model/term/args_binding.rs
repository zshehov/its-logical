use nom::{
    bytes::complete::{tag, take_till1},
    error::VerboseError,
    multi::separated_list1,
    IResult,
};

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct ArgsBinding {
    // TODO: reconsider this being optional, just a string with _ value is probably better
    pub(crate) binding: Vec<Option<String>>,
}
impl ArgsBinding {
    pub(crate) fn encode(&self) -> String {
        let normalised: Vec<String> = self.binding.iter().map(|f| match f {
            Some(s) => s.to_owned(),
            None => "_".to_string(),
        }).collect();

        normalised.join(",")
    }
}

pub(crate) fn parse_args_binding<'a>(
    i: &'a str,
) -> IResult<&'a str, ArgsBinding, VerboseError<&str>> {
    separated_list1(tag(","), take_till1(is_end_of_args))(i).map(|(leftover, args)| {
        (
            leftover,
            ArgsBinding {
                binding: args
                    .iter()
                    .map(|&s| {
                        if s == "_" {
                            return None;
                        }
                        return Some(s.to_string());
                    })
                    .collect(),
            },
        )
    })
}

fn is_end_of_args(c: char) -> bool {
    c == ',' || c == ')'
}
