//! One native clipboard owner keeps platform operations serialized, outside the UI thread.
use std::sync::mpsc::{self, SyncSender};
use tokio::sync::oneshot;
struct Job {
    text: Option<String>,
    reply: oneshot::Sender<Result<String, String>>,
}
pub struct Clipboard {
    sender: SyncSender<Job>,
}
impl Clipboard {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Job>(8);
        std::thread::Builder::new()
            .name("masterdata-clipboard".into())
            .spawn(move || {
                let mut clipboard = arboard::Clipboard::new();
                for job in receiver {
                    let result = match &mut clipboard {
                        Ok(c) => match job.text {
                            Some(text) => c.set_text(text).map(|_| String::new()),
                            None => c.get_text(),
                        }
                        .map_err(|e| e.to_string()),
                        Err(e) => Err(e.to_string()),
                    };
                    let _ = job.reply.send(result);
                }
            })
            .expect("native clipboard worker");
        Self { sender }
    }
    pub async fn text(&self, text: Option<String>) -> Result<String, String> {
        let (reply, result) = oneshot::channel();
        self.sender
            .try_send(Job { text, reply })
            .map_err(|_| "native clipboard is busy or closed".to_owned())?;
        result
            .await
            .map_err(|_| "native clipboard worker closed".to_owned())?
    }
}
