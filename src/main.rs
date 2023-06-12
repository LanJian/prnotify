use std::collections::HashMap;

use crate::clients::cache::{CacheClient, PullRequest};
use crate::clients::github::GithubClient;
use crate::clients::ntfy::NtfyClient;
use crate::feedback::{Comment, Review};
use anyhow::Result;
use log::{debug, info};
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
    let cookies = match settings.firefox {
        Some(firefox_settings) => Some(util::extract_cookies(
            &firefox_settings.cookies_file_path,
            &settings.github.hostname,
        )?),
        None => None,
    };

    let cache_client = CacheClient::new(settings.cache.path);
    let ntfy_client = NtfyClient::try_new(settings.ntfy.base_url, settings.ntfy.topic)?;
    let github_client = GithubClient::try_new(
        &settings.github.personal_access_token,
        format!("https://{}/api/v3", settings.github.hostname),
        cookies,
    )?;

    // read data stored in cache
    let current_data = cache_client.read().unwrap_or_default();

    // get relevant requests from github
    let prs_response = github_client.pull_requests().await?;

    let mut new_data = HashMap::default();

    for pr in prs_response.items {
        let repo_owner = pr.repo_owner()?;
        let repo_name = pr.repo_name()?;

        // get comments, reviews, and review comments from github for the current PR
        let comments_response = github_client
            .issue_comments(&repo_owner, &repo_name, pr.number)
            .await?;
        let reviews_response = github_client
            .reviews(&repo_owner, &repo_name, pr.number)
            .await?;
        let review_comments_response = github_client
            .review_comments(&repo_owner, &repo_name, pr.number)
            .await?;

        let comments_by_ids: HashMap<usize, Comment> = comments_response
            .into_iter()
            .map(|x| {
                (
                    x.id,
                    Comment::new(x.user.login, x.body, pr.html_url.clone(), x.html_url),
                )
            })
            .collect();

        // attach review comments to their reviews
        let mut reviews_by_ids: HashMap<usize, Review> = reviews_response
            .into_iter()
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

        for review_comment in review_comments_response {
            reviews_by_ids
                .entry(review_comment.pull_request_review_id)
                .and_modify(|e| e.add_comment(review_comment.body));
        }

        // add to cache data, to be saved to file later
        new_data.insert(
            pr.id,
            PullRequest {
                reviews: reviews_by_ids.keys().copied().collect(),
                comments: comments_by_ids.keys().copied().collect(),
            },
        );

        // do notifications
        if !current_data.contains_key(&pr.id) {
            debug!("Sending notification for pr: {:?}", pr);
            ntfy_client
                .notify(
                    "New Pull Request",
                    format!("@{} opened {}", pr.user.login, pr.title),
                    &[("Open PR", &pr.html_url)],
                )
                .await?;
            continue;
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
    }

    cache_client.write(&new_data)?;

    info!("Done");
    Ok(())
}
