use crate::*;
use requestty::question::Choice;
use requestty::Question;

pub fn get_search_term() -> Result<String> {
    let question = Question::input("search_term")
        .message("Please enter the name of the movie/show you're looking for")
        .build();
    Ok(requestty::prompt_one(question)?.try_into_string().unwrap())
}

pub fn choose_from_results(results: &Vec<SearchResult>) -> Result<&SearchResult> {
    let choices = {
        let mut choices = results
            .iter()
            .map(|sr| sr.to_string().into())
            .collect::<Vec<Choice<String>>>();
        choices.push(requestty::DefaultSeparator);
        choices.push(Choice::Choice("I can't see what I'm looking for".into()));
        choices
    };

    let question = Question::select("result")
        .message("Pick the correct search result")
        .choices(choices)
        .build();

    let answer = requestty::prompt_one(question)?;
    let item = answer.as_list_item().unwrap();
    if item.index < results.len() {
        Ok(results.get(item.index).unwrap())
    } else {
        // User selected "I can't see what I'm looking for"
        Err(RunError::NoDesiredSearchResults)
    }
}
