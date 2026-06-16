use crate::app::{App, TextFieldExt};
use crate::status;

impl App {
    pub async fn publish_message(&mut self) {
        if self.messaging.topic.get_value().is_empty() {
            self.status = status::topic_empty();
            return;
        }

        let topic = self.messaging.topic.get_value().to_string();
        let message = self.messaging.message.get_value().to_string();

        self.status = status::publishing();
        match self
            .client
            .publish_message(self.universe_id, &topic, &message)
            .await
        {
            Ok(()) => {
                self.status = status::published(&topic);
            }
            Err(err) => {
                self.status = status::api_error(err);
            }
        }
    }
}
