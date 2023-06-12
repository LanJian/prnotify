use std::fmt;

use crate::clients::github;

#[derive(Debug)]
pub struct Comment {
    author: String,
    body: String,
    pub pr_url: String,
    pub url: String,
}

impl Comment {
    pub fn new(author: String, body: String, pr_url: String, url: String) -> Self {
        Self {
            body,
            author,
            pr_url,
            url,
        }
    }
}

impl fmt::Display for Comment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "@{} commented:", self.author)?;
        writeln!(f, "")?;
        writeln!(f, "{}", self.body)
    }
}

#[derive(Debug)]
pub enum ReviewState {
    Commented,
    Approved,
    ChangesRequested,
}

impl From<github::ReviewState> for ReviewState {
    fn from(value: github::ReviewState) -> Self {
        match value {
            github::ReviewState::Approved => Self::Approved,
            github::ReviewState::ChangesRequested => Self::ChangesRequested,
            github::ReviewState::Commented => Self::Commented,
        }
    }
}

#[derive(Debug)]
pub struct Review {
    author: String,
    state: ReviewState,
    body: Option<String>,
    comments: Vec<String>,
    pub pr_url: String,
    pub url: String,
}

impl Review {
    pub fn new(
        author: String,
        state: ReviewState,
        body: String,
        pr_url: String,
        url: String,
    ) -> Self {
        Self {
            author,
            state,
            body: (!body.is_empty()).then_some(body),
            comments: Vec::default(),
            pr_url,
            url,
        }
    }

    pub fn add_comment(&mut self, comment: String) {
        self.comments.push(comment);
    }
}

impl fmt::Display for Review {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let review_state_msg = match self.state {
            ReviewState::Approved => format!("@{} approved:", self.author),
            ReviewState::ChangesRequested => format!("@{} requested changes:", self.author),
            ReviewState::Commented => format!("@{} commented:", self.author),
        };
        writeln!(f, "{}", review_state_msg)?;

        if let Some(msg) = &self.body {
            writeln!(f, "")?;
            writeln!(f, "{}", msg)?;
        } else if self.comments.len() == 1 {
            // display the only comment as the review body
            writeln!(f, "")?;
            writeln!(f, "{}", self.comments[0])?;
        }

        if self.comments.len() > 1 {
            writeln!(f, "")?;
            writeln!(f, "(+ {} comments)", self.comments.len())?;
        }

        Ok(())
    }
}
