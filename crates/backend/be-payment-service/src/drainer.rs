use std::sync::Arc;

use be_remote_db::DatabaseManager;
use chrono::Duration as ChronoDuration;
use tokio::sync::oneshot;
use tokio::time::{Duration, sleep};

use crate::analytics;
use crate::provision::{ProvisionError, StripeBillingProvisioner};

/// Maximum number of jobs claimed per tick. Sized so a backlog drains in a
/// few iterations without monopolising the Stripe API.
const BATCH_SIZE: i64 = 50;

/// How long a claimed job stays "leased" before another worker may retry it.
/// Sized to be longer than the worst-case Stripe + DB round-trip.
const LEASE: ChronoDuration = ChronoDuration::minutes(2);

/// Time between drainer ticks when the queue is empty.
const IDLE_POLL_INTERVAL: Duration = Duration::from_secs(15);

/// Time between drainer ticks when there might be more work to do (we hit the
/// batch size). Lets a backlog drain quickly without busy-looping.
const BUSY_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Permanent failures are kept for inspection; transient failures back off
/// exponentially up to `MAX_BACKOFF`.
const MAX_BACKOFF: ChronoDuration = ChronoDuration::hours(1);

/// Once a job hits this many attempts we cap its backoff at [`MAX_BACKOFF`]
/// (instead of growing exponentially) and rely on backlog alerting to surface
/// the row for manual intervention. The job continues to retry indefinitely.
const MAX_ATTEMPTS: i32 = 20;

pub struct DrainerHandle {
    shutdown: Option<oneshot::Sender<()>>,
    join: tokio::task::JoinHandle<()>,
}

impl DrainerHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        let _ = self.join.await;
    }
}

/// Spawn the background worker that drains the
/// `stripe.customer_provisioning_jobs` outbox.
///
/// Idempotent on a per-user basis: each job is leased via
/// `claim_due_provisioning_jobs`, then the provisioner is called and either
/// the row is deleted (success) or rescheduled with backoff (failure).
pub fn spawn_drainer(
    db: Arc<DatabaseManager>,
    provisioner: Arc<StripeBillingProvisioner>,
) -> DrainerHandle {
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let join = tokio::spawn(async move {
        tracing::info!("Stripe customer provisioning drainer started");
        loop {
            let processed = match tick(&db, &provisioner).await {
                Ok(n) => n,
                Err(e) => {
                    tracing::error!(error = %e, "Drainer tick failed");
                    0
                }
            };

            let next_delay = if processed >= BATCH_SIZE as usize {
                BUSY_POLL_INTERVAL
            } else {
                IDLE_POLL_INTERVAL
            };

            tokio::select! {
                _ = sleep(next_delay) => {}
                _ = &mut shutdown_rx => {
                    tracing::info!("Stripe customer provisioning drainer shutting down");
                    break;
                }
            }
        }
    });

    DrainerHandle {
        shutdown: Some(shutdown_tx),
        join,
    }
}

async fn tick(
    db: &DatabaseManager,
    provisioner: &StripeBillingProvisioner,
) -> anyhow::Result<usize> {
    let jobs = db
        .claim_due_provisioning_jobs()
        .limit(BATCH_SIZE)
        .lease(LEASE)
        .call()
        .await?;

    let count = jobs.len();
    if count > 0 {
        let backlog = db.count_pending_provisioning_jobs().call().await.ok();
        tracing::debug!(claimed = count, backlog = ?backlog, "Drainer claimed jobs");
    }

    for job in jobs {
        process_job(db, provisioner, job).await;
    }

    Ok(count)
}

async fn process_job(
    db: &DatabaseManager,
    provisioner: &StripeBillingProvisioner,
    job: be_remote_db::ProvisioningJob,
) {
    let user = match db.get_user().id(job.user_id).call().await {
        Ok(u) => u,
        Err(e) if e.is_not_found() => {
            tracing::info!(user_id = %job.user_id, "User vanished before provisioning — clearing job");
            if let Err(e) = db
                .complete_stripe_customer_provisioning()
                .user_id(job.user_id)
                .call()
                .await
            {
                tracing::warn!(user_id = %job.user_id, error = %e, "Failed to clear orphan job");
            }
            return;
        }
        Err(e) => {
            tracing::warn!(user_id = %job.user_id, error = %e, "Failed to load user for provisioning");
            return;
        }
    };

    match provisioner.ensure_customer(user.id, &user.email).await {
        Ok(_) => {
            analytics::track_customer_provisioning_attempt("succeeded");
            if let Err(e) = db
                .complete_stripe_customer_provisioning()
                .user_id(job.user_id)
                .call()
                .await
            {
                tracing::error!(user_id = %job.user_id, error = %e, "Provisioned but failed to clear job");
            }
        }
        Err(err) => {
            analytics::track_customer_provisioning_attempt(err.kind());
            let next_attempts = job.attempts.saturating_add(1);
            let permanent = matches!(err, ProvisionError::Permanent(_));
            let exhausted = next_attempts >= MAX_ATTEMPTS;

            if permanent || exhausted {
                tracing::error!(
                    user_id = %job.user_id,
                    attempts = next_attempts,
                    permanent,
                    error = %err,
                    "Stripe customer provisioning blocked — manual intervention required"
                );
            } else {
                tracing::warn!(
                    user_id = %job.user_id,
                    attempts = next_attempts,
                    error = %err,
                    "Stripe customer provisioning failed — will retry"
                );
            }

            let backoff = if permanent || exhausted {
                MAX_BACKOFF
            } else {
                exponential_backoff(next_attempts)
            };

            if let Err(e) = db
                .fail_stripe_customer_provisioning()
                .user_id(job.user_id)
                .error(&err.to_string())
                .retry_after(backoff)
                .call()
                .await
            {
                tracing::error!(user_id = %job.user_id, error = %e, "Failed to record provisioning failure");
            }
        }
    }
}

fn exponential_backoff(attempts: i32) -> ChronoDuration {
    let exp = attempts.clamp(1, 12) as u32;
    let secs = 2_i64.saturating_pow(exp);
    let candidate = ChronoDuration::seconds(secs);
    if candidate > MAX_BACKOFF {
        MAX_BACKOFF
    } else {
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_then_caps() {
        assert_eq!(exponential_backoff(1), ChronoDuration::seconds(2));
        assert_eq!(exponential_backoff(5), ChronoDuration::seconds(32));
        assert_eq!(exponential_backoff(11), ChronoDuration::seconds(2048));
        assert_eq!(exponential_backoff(12), MAX_BACKOFF);
        assert_eq!(exponential_backoff(20), MAX_BACKOFF);
    }
}
