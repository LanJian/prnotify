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

## Setup and Usage

### Docker

A docker image is published at
[`jackhxs/prnotify`](https://hub.docker.com/r/jackhxs/prnotify).

Create a configuration file(see [below](#configuration)) and a cache directory on the host
machine and mount them into the container. You can optionally provide config
values via environment variables, this may be desirable for specifying secrets.

Example docker command:
```sh
docker run \
  --volume /path/to/prnotify/config.toml:/etc/prnotify/prnotify.toml \
  --volume /path/to/prnotify/cache-dir:/var/cache/prnotify \
  --env PRNOTIFY__GITHUB__PERSONAL_ACCESS_TOKEN=ghp_faketoken \
  jackhxs/prnotify:latest
```

To poll periodically, make a cron job. Example cron:
```crontab
# Run every 5 minutes and append output to a log file
*/5 * * * * docker run [args...] >> /home/fakeuser/.local/log/prnotify.log 2>&1
```

### Kubernetes

To poll periodically, set up a `CronJob` in Kubernetes. You'll also need to
create a `ConfigMap` and a `PersistentVolumeClaim` for the config file and the
cache directory respectively. You can optionally provide config values via
secrets and environment variables.

Example Kubernetes manifests:

#### `pvc.yaml`
```yaml
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: prnotify-cache-pvc
  namespace: prnotify
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: local-path
  resources:
    requests:
      storage: 10Mi
```

#### `secret.yaml`
This example is only for demonstration purpose. Don't check secrets into source
control. Instead, you can use `kubectl create secret` to create a secret, or
refer to the docs for [Kubernetes
secrets](https://kubernetes.io/docs/concepts/configuration/secret/) for other
options.

```yaml
---
apiVersion: v1
data:
  PRNOTIFY__GITHUB__PERSONAL_ACCESS_TOKEN: base64-encoded-github-pat
kind: Secret
metadata:
  name: prnotify-raw-secrets
  namespace: prnotify
```

#### `config-map.yaml`
See [below](#configuration) for all the config options.
```yaml
---
apiVersion: v1
data:
  settings.toml: |
    [github]
    username = "fake-user"

    [ntfy]
    base_url = "https://ntfy.exampledomain.com"
    topic = "example-topic"

    [cache]
    path = "/var/cache/prnotify/prnotify.json"
kind: ConfigMap
metadata:
  name: prnotify-config-map
  namespace: prnotify
```

#### `cron-job.yaml`
```yaml
---
apiVersion: batch/v1
kind: CronJob
metadata:
  name: prnotify-cronjob
  namespace: prnotify
spec:
  schedule: "*/5 * * * *"
  concurrencyPolicy: Forbid
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: Never
          containers:
            - name: prnotify
              image: jackhxs/prnotify:latest
              imagePullPolicy: Always
              envFrom:
                - secretRef:
                    name: prnotify-raw-secrets
              volumeMounts:
                - name: settings
                  mountPath: /etc/prnotify
                - name: cache
                  mountPath: /var/cache/prnotify
          volumes:
            - name: settings
              configMap:
                name: prnotify-config-map
                items:
                  - key: settings.toml
                    path: prnotify.toml
            - name: cache
              persistentVolumeClaim:
                claimName: prnotify-cache-pvc
```

### Source

Check out the repo, create a configuration file(see [below](#configuration)), and run:
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
In these cases, `prnotify` provides some options:

* Extract cookies from Firefox cookies storage and pass it along with requests
to Github
* Pass requests to Github through a HTTP proxy

See the [Configuration](#configuration) section for more details.

### ntfy

Requests to ntfy currently isn't authenticated. Options for using ntfy are:
1. (Recommended) Run a self-hosted internal ntfy server. 
2. Use the public server at https://ntfy.sh. Be sure to pick a topic name that
is not easily guessable per ntfy's [documentation](https://docs.ntfy.sh/publish/)

## Configuration

Configuration is loaded and merged from the following sources in order:

#### 1. System config file path:

On Linux and Mac: `/etc/prnotify/prnotify.toml`

#### 2. Default user config file path:

* On Linux: `$XDG_CONFIG_HOME/prnotify/prnotify.toml`
or `$HOME/.config/prnotify/prnotify.toml`
* On Mac: `$HOME/Library/Application Support/prnotify/prnotify.toml`

#### 3. Environment variables:

All configuration options can be set as env vars in `SCREAMING_SNAKE_CASE` with
the prefix `PRNOTIFY`. Nested attributes are separated by `__`. Examples:
* `export PRNOTIFY__GITHUB__HOSTNAME=github.examplecompany.com`
* `export PRNOTIFY__NTFY__BASE_URL=ntfy.exampledomain.com`
* `export PRNOTIFY__NTFY__TOPIC=example-topic`

---

Additionally, the log level(default `INFO`) can be set via the `RUST_LOG`
environment variables. To get more verbose logs:
```
export RUST_LOG=debug
```

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

# (Optional) The URL of the proxy server to send Github API requests through.
proxy_url = "http://fake-ip-or-hostname:5678"

# (Optional) The list of queries to search. Any issue that appears in at least
# one of the query results will be processed. The default query searches for
# open PRs that involves the current authenticated user.
#
# Default: ["is:open is:pr involves:@me"]
queries = [
  "is:open is:pr involves:@me",
  "is:open is:pr label:example-label"
]

# (Optional) List of comment patterns to exclude. Each pattern is parsed as a
# regular expression.If a comment matches any of the patterns, it will be
# ignored and not trigger a notification.
#
# Default: []
exclude_comment_patterns = [
  "^.*filtered pattern.*%",
  "^.*another filtered pattern.*%",
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
