# prnotify

*Poll-based notifier for Github Pull Requests*

A Github PR notifier that does not listen on webhooks and therefore does not
need to be installed into a Github org or repo. Only a Github personal access
token is required.

The notifier pulls PRs using the provided search queries. It then sends a
notification using [ntfy](https://ntfy.sh/) for any:
  * New PRs opened
  * New comments
  * New reviews

## Usage

Check out the repo, create a configuration file(see below), and run:
```sh
cargo run --release
```

To poll periodically, make a cron job. Example cron:
```crontab
# Run every 5 minutes and append output to a log file
*/5 * * * * /home/fakeuser/code/prnotify/target/release/prnotify >> /home/fakeuser/.local/log/prnotify.log 2>&1
```

## Authentication

### Github

A Github personal access token(PAT) with repo read permissions is required to
authenticate to Github. A token can be created by following the
[docs](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens).

In some cases, the PAT is not enough to authenticate with Github
programatically. For example, custom Github Enterprise or SSO setups.
In these cases, `prnotify` can extract cookies from Firefox cookies storage
and pass it along with requests to Github. See the Configuration section for
more details.

### ntfy

Requests to ntfy currently isn't authenticated. Options for using ntfy are:
1. (Recommended) Run a self-hosted internal ntfy server. 
2. Use the public server at https://ntfy.sh. Be sure to pick a topic name that
is not easily guessable per ntfy's [documentation](https://docs.ntfy.sh/publish/)

## Configuration

Configuration is loaded and merged from the following sources in order:

#### 1. `$XDG_CONFIG_HOME/prnotify/prnotify.toml`

On Linux, this defaults to `$HOME/.config/prnotify/prnotify.toml`

#### 2. Environment variables:

All configuration options can be set as env vars in `SCREAMING_SNAKE_CASE` with
the prefix `PRNOTIFY`. Examples:
* `export PRNOTIFY_GITHUB_HOSTNAME=github.examplecompany.com`
* `export PRNOTIFY_NTFY_TOPIC=example-topic`

### Configuration Options

```toml
# (Required) Settings for connecting to Github
[github]
# (Required) The personal access token to authenticate with Github
personal_access_token = "ghp_faketoken"

# (Required) The username of the authenticated user. Comments made by this user
# are filtered out and will not trigger notifications.
username = "fake-user"

# (Optional) The hostname of the Github API to connect to. Specify this if you
# are connecting to a Github Enterprise server.
#
# Default: github.com
hostname = "github.examplecompany.com"

# (Optional) The list of queries to search. Any issue that appears in at least
# one of the query results will be processed. The default query searches for
# open PRs that involves the current authenticated user.
#
# Default: ["is:open is:pr involves:@me"]
queries = [
  "is:open is:pr involves:@me",
  "is:open is:pr label:example-label"
]

# (Required) Settings for connecting to ntfy
[ntfy]
# (Required) The base url of the ntfy server
base_url = "https://ntfy.exampledomain.com"

# (Required) The ntfy topic to send notifications to
topic = "example-topic"

# (Required) Settings for the local cache
[cache]
# (Required) The path of the cache file to read from and write to
path = "~/.cache/prnotify.json"

# (Optional) Settings for extracting cookies from Firefox. Specify this if you
# need to provide cookies to authenticate with Github.
#
# Note: this is normally not needed. This is only needed if you cannot
# authenticate to Github programatically with just the personal access token.
[firefox]
# (Required) The path of the Firefox cookies file. The default path on Linux
# is `~/.mozilla/firefox/{profile}/cookies.sqlite`.
cookies_file_path = "~/.mozilla/firefox/example-profile/cookies.sqlite"
```
