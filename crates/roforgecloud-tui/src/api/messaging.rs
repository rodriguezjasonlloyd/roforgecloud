use crate::app::{App, TextFieldExt};

impl App {
    pub async fn publish_message(&mut self) {
        if self.messaging.topic.get_value().is_empty() {
            self.status = "topic cannot be empty".to_string();
            return;
        }

        let topic = self.messaging.topic.get_value().to_string();
        let message = self.messaging.message.get_value().to_string();

        self.status = "publishing...".to_string();
        match self
            .client
            .publish_message(
                self.universe_id,
                &topic,
                &message,
            )
            .await
        {
            Ok(()) => {
                self.status = format!("published to '{topic}'");
            }
            Err(err) => {
                self.status = format!("error: {err}");
            }
        }
    }
}
