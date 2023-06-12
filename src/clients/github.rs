use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::{
    header::{HeaderMap, ACCEPT, AUTHORIZATION, COOKIE},
    Client,
};
use serde::{de::DeserializeOwned, Deserialize};

#[derive(Debug, Deserialize)]
pub struct User {
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    pub id: usize,
    pub number: usize,
    pub title: String,
    pub user: User,
    pub html_url: String,
}

impl Issue {
    // Extract the repo owner from the issue url
    pub fn repo_owner(&self) -> Result<String> {
        let re = Regex::new(r"https://[^/]+/([^/]+)/[^/]+/pull/\d+")?;
        let cap = re
            .captures_iter(&self.html_url)
            .next()
            .and_then(|x| x.get(1))
            .ok_or_else(|| anyhow!("Invalid pull request url"))?;

        Ok(cap.as_str().to_owned())
    }

    // Extract the repo name from the issue url
    pub fn repo_name(&self) -> Result<String> {
        let re = Regex::new(r"https://[^/]+/[^/]+/([^/]+)/pull/\d+")?;
        let cap = re
            .captures_iter(&self.html_url)
            .next()
            .and_then(|x| x.get(1))
            .ok_or_else(|| anyhow!("Invalid pull request url"))?;

        Ok(cap.as_str().to_owned())
    }
}

#[derive(Debug, Deserialize)]
pub struct SearchIssuesResponse {
    pub items: Vec<Issue>,
}

#[derive(Debug, Deserialize)]
pub struct Comment {
    pub id: usize,
    pub body: String,
    pub user: User,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReviewState {
    Commented,
    Approved,
    ChangesRequested,
}

#[derive(Debug, Deserialize)]
pub struct Review {
    pub id: usize,
    pub body: String,
    pub state: ReviewState,
    pub user: User,
    pub html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ReviewComment {
    pub id: usize,
    pub pull_request_review_id: usize,
    pub body: String,
    pub user: User,
}

pub struct GithubClient {
    client: Client,
    base_url: String,
}

impl GithubClient {
    pub fn try_new(access_token: &str, base_url: String, cookie: Option<String>) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, "application/vnd.github+json".parse()?);
        headers.insert(AUTHORIZATION, format!("Bearer {}", access_token).parse()?);

        if let Some(unwrapped) = cookie {
            headers.insert(COOKIE, unwrapped.parse()?);
        }

        let github_client = Self {
            client: Client::builder().default_headers(headers).build()?,
            base_url,
        };

        Ok(github_client)
    }

    /// Returns a list of pull requests that involves the authenticated user
    pub async fn pull_requests(&self) -> Result<SearchIssuesResponse> {
        let response = self
            .client
            .get(format!("{}/search/issues", self.base_url))
            .query(&[("q", "is:open is:pr involves:@me"), ("per_page", "100")])
            .send()
            .await?;

        Ok(response.json().await?)
    }

    /// Returns a list of issue comments for the given pull request
    pub async fn issue_comments(
        &self,
        repo_owner: &str,
        repo_name: &str,
        pull_request_id: usize,
    ) -> Result<Vec<Comment>> {
        self.get_all(&format!(
            "/repos/{}/{}/issues/{}/comments",
            repo_owner, repo_name, pull_request_id
        ))
        .await
    }

    /// Returns a list of reviews for the given pull request
    pub async fn reviews(
        &self,
        repo_owner: &str,
        repo_name: &str,
        pull_request_id: usize,
    ) -> Result<Vec<Review>> {
        self.get_all(&format!(
            "/repos/{}/{}/pulls/{}/reviews",
            repo_owner, repo_name, pull_request_id
        ))
        .await
    }

    /// Returns a list of review comments for the given pull request
    pub async fn review_comments(
        &self,
        repo_owner: &str,
        repo_name: &str,
        pull_request_id: usize,
    ) -> Result<Vec<ReviewComment>> {
        self.get_all(&format!(
            "/repos/{}/{}/pulls/{}/comments",
            repo_owner, repo_name, pull_request_id
        ))
        .await
    }

    async fn get_all<T>(&self, path: &str) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut ret = Vec::default();
        let mut page = 1_usize;

        loop {
            let response = self
                .client
                .get(format!("{}{}", self.base_url, path))
                .query(&[("per_page", 100), ("page", page)])
                .send()
                .await?;

            let mut parsed: Vec<T> = response.json().await?;
            if parsed.is_empty() {
                break;
            }

            ret.append(&mut parsed);
            page += 1;
        }

        Ok(ret)
    }
}
