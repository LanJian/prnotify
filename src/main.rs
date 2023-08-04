use std::collections::HashMap;

use crate::clients::cache::{CacheClient, PullRequest};
use crate::clients::github::{GithubClient, Issue, ReviewState};
use crate::clients::ntfy::NtfyClient;
use crate::feedback::{Comment, Review};
use anyhow::Result;
use log::{debug, info};
use regex::Regex;
use settings::Settings;

mod clients;
mod feedback;
mod settings;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting prnotify");

    // parse settings
    let settings = Settings::try_new()?;

    // if settings are specified, extract cookies from firefox local storage
    let cookies = match &settings.firefox {
        Some(firefox_settings) => Some(util::extract_cookies(
            &firefox_settings.cookies_file_path,
            &settings.github.hostname,
        )?),
        None => None,
    };

    // parse regexes
    let exclude_comment_patterns = settings
        .github
        .exclude_comment_patterns
        .iter()
        .map(|x| Regex::new(x))
        .collect::<Result<Vec<Regex>, _>>()?;

    // initialize clients
    let cache_client = CacheClient::new(settings.cache.path);
    let ntfy_client = NtfyClient::try_new(settings.ntfy.base_url, settings.ntfy.topic)?;
    let github_client = GithubClient::try_new(
        &settings.github.personal_access_token,
        format!("https://{}/api/v3", settings.github.hostname),
        cookies,
        settings.github.proxy_url,
    )?;

    // read data stored in cache
    let current_data = cache_client.read().unwrap_or_default();
    let mut new_data = HashMap::default();

    // get relevant pull requests from github
    let mut prs_by_ids: HashMap<usize, Issue> = HashMap::default();
    for query in settings.github.queries {
        let prs_response = github_client.pull_requests(&query).await?;
        for pr in prs_response.items {
            prs_by_ids.entry(pr.id).or_insert(pr);
        }
    }

    for (_, pr) in prs_by_ids {
        // get comments, reviews, and review comments from github for the current PR
        let comments_by_ids = comments_by_ids(
            &github_client,
            &pr,
            settings.github.username.as_str(),
            &exclude_comment_patterns,
        )
        .await?;
        let reviews_by_ids = reviews_by_ids(
            &github_client,
            &pr,
            settings.github.username.as_str(),
            &exclude_comment_patterns,
        )
        .await?;

        // do notifications
        send_notifications(
            &ntfy_client,
            &current_data,
            &reviews_by_ids,
            &comments_by_ids,
            &pr,
        )
        .await?;

        // add to cache data, to be saved to file later
        new_data.insert(
            pr.id,
            PullRequest {
                reviews: reviews_by_ids.keys().copied().collect(),
                comments: comments_by_ids.keys().copied().collect(),
            },
        );
    }

    cache_client.write(&new_data)?;

    info!("Done");
    Ok(())
}

async fn reviews_by_ids(
    github_client: &GithubClient,
    pr: &Issue,
    username: &str,
    exclude_comment_patterns: &Vec<Regex>,
) -> Result<HashMap<usize, Review>> {
    let repo_owner = pr.repo_owner()?;
    let repo_name = pr.repo_name()?;

    let reviews_response = github_client
        .reviews(&repo_owner, &repo_name, pr.number)
        .await?;
    let review_comments_response = github_client
        .review_comments(&repo_owner, &repo_name, pr.number)
        .await?;

    let mut ret: HashMap<usize, Review> = reviews_response
        .into_iter()
        .filter(|x| x.state != ReviewState::Pending && x.user.login != username)
        .filter(|x| !is_comment_filtered(&x.body, exclude_comment_patterns))
        .map(|x| {
            (
                x.id,
                Review::new(
                    x.user.login,
                    x.state.into(),
                    x.body,
                    pr.html_url.clone(),
                    x.html_url,
                ),
            )
        })
        .collect();

    // attach review comments to their reviews
    for review_comment in review_comments_response {
        ret.entry(review_comment.pull_request_review_id)
            .and_modify(|e| e.add_comment(review_comment.body));
    }

    Ok(ret)
}

async fn comments_by_ids(
    github_client: &GithubClient,
    pr: &Issue,
    username: &str,
    exclude_comment_patterns: &Vec<Regex>,
) -> Result<HashMap<usize, Comment>> {
    let repo_owner = pr.repo_owner()?;
    let repo_name = pr.repo_name()?;

    let comments_response = github_client
        .issue_comments(&repo_owner, &repo_name, pr.number)
        .await?;

    let ret: HashMap<usize, Comment> = comments_response
        .into_iter()
        .filter(|x| x.user.login != username)
        .filter(|x| !is_comment_filtered(&x.body, exclude_comment_patterns))
        .map(|x| {
            (
                x.id,
                Comment::new(x.user.login, x.body, pr.html_url.clone(), x.html_url),
            )
        })
        .collect();

    Ok(ret)
}

async fn send_notifications(
    ntfy_client: &NtfyClient,
    current_data: &HashMap<usize, PullRequest>,
    reviews_by_ids: &HashMap<usize, Review>,
    comments_by_ids: &HashMap<usize, Comment>,
    pr: &Issue,
) -> Result<()> {
    if !current_data.contains_key(&pr.id) {
        debug!("Sending notification for new pr: {:?}", pr);
        ntfy_client
            .notify(
                "New Pull Request",
                format!("@{} opened {}", pr.user.login, pr.title),
                &[("Open PR", &pr.html_url)],
            )
            .await?;

        // this is a new PR, no need to check comments or reviews
        return Ok(());
    }

    let current = &current_data[&pr.id];
    for (k, v) in comments_by_ids {
        if !current.comments.contains(&k) {
            debug!("Sending notification for comment: {:?}", v);
            ntfy_client
                .notify(
                    &pr.title,
                    v.to_string(),
                    &[("Open PR", &v.pr_url), ("Open Comment", &v.url)],
                )
                .await?;
        }
    }

    for (k, v) in reviews_by_ids {
        if !current.reviews.contains(&k) {
            debug!("Sending notification for review: {:?}", v);
            ntfy_client
                .notify(
                    &pr.title,
                    v.to_string(),
                    &[("Open PR", &v.pr_url), ("Open Comment", &v.url)],
                )
                .await?;
        }
    }

    Ok(())
}

fn is_comment_filtered(body: &str, exclude_comment_patterns: &Vec<Regex>) -> bool {
    exclude_comment_patterns.iter().any(|x| x.is_match(body))
}
