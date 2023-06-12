use anyhow::Result;
use reqwest::Client;

pub struct NtfyClient {
    client: Client,
    base_url: String,
    topic: String,
}

impl NtfyClient {
    pub fn try_new(base_url: String, topic: String) -> Result<Self> {
        let ntfy_client = Self {
            client: Client::builder().build()?,
            base_url,
            topic,
        };

        Ok(ntfy_client)
    }

    pub async fn notify(
        &self,
        title: &str,
        message: String,
        view_actions: &[(&str, &str)],
    ) -> Result<()> {
        let actions_header_value = view_actions
            .iter()
            .map(|&(a, b)| format!("view, {}, {};", a, b))
            .collect::<Vec<String>>()
            .join(" ");

        self.client
            .post(format!("{}/{}", self.base_url, self.topic))
            .header("Title", title)
            .header("Actions", actions_header_value)
            .body(message)
            .send()
            .await?;

        Ok(())
    }
}
