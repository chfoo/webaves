//! Retry operations.

use std::{future::Future, time::Duration};

use backoff::{backoff::Backoff, ExponentialBackoff};

/// Performs an operation with reattempts.
pub struct Retry {
    backoff: ExponentialBackoff,
}

impl Retry {
    /// Creates a new `Retry` with the default backoff configuration.
    pub fn new() -> Self {
        Self {
            backoff: ExponentialBackoff {
                initial_interval: Duration::from_secs(2),
                max_interval: Duration::from_secs(3600),
                ..Default::default()
            },
        }
    }

    /// Returns a reference to the backoff algorithm object.
    pub fn backoff(&self) -> &ExponentialBackoff {
        &self.backoff
    }

    /// Returns a mutable reference to the backoff algorithm object.
    pub fn backoff_mut(&mut self) -> &mut ExponentialBackoff {
        &mut self.backoff
    }

    /// Sets the backoff algorithm object.
    pub fn set_backoff(&mut self, backoff: ExponentialBackoff) {
        self.backoff = backoff;
    }

    /// Runs a function until it is successful.
    ///
    /// The function `operation` will be called repeatedly until it is
    /// successful as determined by `success_condition`. Between each call
    /// of `operation`, this function will sleep based on `backoff`.
    ///
    /// The function `success_condition` accepts a reference to the output of
    /// `operation`. If `success_condition` returns `true`, the output is
    /// considered successful. If `false`, the output is discarded and the retry
    /// loop will continue.
    pub async fn async_run<O, OFut, R, C>(&mut self, operation: O, success_condition: C) -> R
    where
        O: Fn() -> OFut,
        OFut: Future<Output = R>,
        C: Fn(&R) -> bool,
    {
        self.backoff.reset();

        loop {
            let result = operation().await;
            let success = success_condition(&result);

            if success {
                return result;
            } else {
                match self.backoff.next_backoff() {
                    Some(duration) => tokio::time::sleep(duration).await,
                    None => return result,
                }
            }
        }
    }
}

impl Default for Retry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[tokio::test]
    async fn test_retry_async() {
        let input = Arc::new(Mutex::new(vec![1, 2]));
        let result = Retry::default()
            .async_run(
                || async {
                    let mut g = input.lock().unwrap();
                    g.remove(0)
                },
                |&item| item == 2,
            )
            .await;

        assert_eq!(result, 2);
    }
}
