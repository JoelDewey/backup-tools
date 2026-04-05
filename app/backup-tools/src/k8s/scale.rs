use crate::k8s::{DefaultK8sClient, K8sClient, K8sConfig};
use anyhow::{anyhow, Context, Result};
use envy::prefixed;
use std::fs::read_to_string;
use std::thread::sleep;
use std::time::Duration;
use tracing::{error, info, trace_span};

const K8S_PREFIX: &str = "KUBERNETES_";

pub fn scale_deployment(inner: impl FnOnce() -> Result<()>) -> Result<()> {
    let span = trace_span!("k8s");
    let _entered = span.enter();

    let k8s_config = prefixed(K8S_PREFIX).from_env::<K8sConfig>()?;
    let k8s_client = DefaultK8sClient::new(&k8s_config)?;
    let service_namespace = k8s_config
        .service_namespace
        .clone()
        .or_else(|| get_namespace(&k8s_config))
        .ok_or_else(|| anyhow!("Failed to determine namespace."))?;

    run_with_scaling(
        &k8s_client,
        &service_namespace,
        &k8s_config.service_deployment_name,
        inner,
    )
}

fn run_with_scaling(
    client: &impl K8sClient,
    namespace: &str,
    deployment_name: &str,
    inner: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let replica_count = client
        .get_available_replicas(namespace, deployment_name)
        .context("Retrieving original replica count.")?;

    if replica_count == 0 {
        info!("Deployment replicas already at 0, no scale down needed.")
    } else {
        info!("Scaling down deployment...");
        scale_down(namespace, deployment_name, client)
            .inspect(|_| info!("Finished scaling down deployment."))
            .context("Failed to scale down deployment.")?;
    }

    let inner_result = inner();
    if inner_result.is_err() {
        error!("Executing inner backup process failed! Attempting to scale deployment up anyway.");
    }

    if replica_count == 0 {
        info!("Deployment replicas were set to 0 initially so no scale up is required.")
    } else {
        match scale_up(namespace, deployment_name, client, replica_count) {
            Ok(c) => {
                info!(replica_count=%c, "Scaled back up to the original replica count.")
            }
            Err(e) => {
                error!(ex=?e, "Failed to scale Deployment back to original replica count.")
            }
        }
    }

    inner_result
}

fn scale_down(namespace: &str, deployment_name: &str, client: &impl K8sClient) -> Result<i32> {
    scale(namespace, deployment_name, client, 0)
}

fn scale_up(
    namespace: &str,
    deployment_name: &str,
    client: &impl K8sClient,
    target_replicas: i32,
) -> Result<i32> {
    scale(namespace, deployment_name, client, target_replicas)
}

fn scale(
    namespace: &str,
    deployment_name: &str,
    client: &impl K8sClient,
    target_replicas: i32,
) -> Result<i32> {
    let prev_replicas = client.get_available_replicas(namespace, deployment_name)?;
    if prev_replicas == target_replicas {
        return Ok(prev_replicas);
    }

    info!("Replica count prior to scale operation: {}", prev_replicas);

    info!(
        "Beginning scale to target replica count of {}; waiting 120 seconds for the scaling to complete.",
        &target_replicas
    );
    client.scale(namespace, deployment_name, target_replicas)?;

    let delay = Duration::from_secs(1);
    let mut replica_count = -1;
    for i in 0..120 {
        replica_count = client.get_available_replicas(namespace, deployment_name)?;
        if replica_count == target_replicas {
            return Ok(replica_count);
        } else {
            if i % 5 == 0 {
                info!(
                    "Still waiting for replica count to reach target count of {} replica(s); replica count is at {} replica(s).",
                    &target_replicas,
                    &replica_count,
                );
            }

            sleep(delay);
        }
    }

    Err(anyhow!(format!(
        "Failed to scale down application after 120 seconds; replica count: {}",
        replica_count
    )))
}

fn get_namespace(config: &K8sConfig) -> Option<String> {
    if let Some(path) = &config.namespace_file_path {
        read_to_string(path).map_or_else(
            |e| {
                error!(ex=?e, "Failed to load namespace from namespace file.");
                None
            },
            Some,
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{run_with_scaling, scale};
    use crate::k8s::K8sClient;
    use anyhow::{anyhow, Result};
    use std::cell::RefCell;
    use std::collections::VecDeque;

    struct MockK8sClient {
        replica_responses: RefCell<VecDeque<i32>>,
        scale_calls: RefCell<Vec<(String, String, i32)>>,
    }

    impl MockK8sClient {
        fn new(replica_responses: Vec<i32>) -> Self {
            MockK8sClient {
                replica_responses: RefCell::new(replica_responses.into_iter().collect()),
                scale_calls: RefCell::new(Vec::new()),
            }
        }

        fn scale_call_count(&self) -> usize {
            self.scale_calls.borrow().len()
        }

        fn scale_calls(&self) -> Vec<(String, String, i32)> {
            self.scale_calls.borrow().clone()
        }
    }

    impl K8sClient for MockK8sClient {
        fn get_available_replicas(&self, _namespace: &str, _name: &str) -> Result<i32> {
            Ok(self
                .replica_responses
                .borrow_mut()
                .pop_front()
                .expect("No more replica responses queued in MockK8sClient"))
        }

        fn scale(&self, namespace: &str, name: &str, count: i32) -> Result<()> {
            self.scale_calls
                .borrow_mut()
                .push((namespace.to_string(), name.to_string(), count));
            Ok(())
        }
    }

    // --- scale() ---

    #[test]
    fn scale_given_already_at_target_returns_count_without_calling_scale() {
        let client = MockK8sClient::new(vec![0]);

        let result = scale("ns", "deploy", &client, 0);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(client.scale_call_count(), 0);
    }

    #[test]
    fn scale_given_target_reached_on_first_poll_returns_target() {
        // First get_available_replicas → 2 (current); after scale() call → 0 (target reached)
        let client = MockK8sClient::new(vec![2, 0]);

        let result = scale("ns", "deploy", &client, 0);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
        assert_eq!(client.scale_call_count(), 1);
    }

    #[test]
    fn scale_passes_correct_namespace_and_name_to_client() {
        let client = MockK8sClient::new(vec![3, 0]);
        scale("my-namespace", "my-deployment", &client, 0).unwrap();
        let calls = client.scale_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0],
            ("my-namespace".to_string(), "my-deployment".to_string(), 0)
        );
    }

    // --- run_with_scaling() ---

    #[test]
    fn run_with_scaling_given_zero_replicas_skips_scale_and_runs_inner() {
        let client = MockK8sClient::new(vec![0]);
        let inner_called = RefCell::new(false);

        let result = run_with_scaling(&client, "ns", "deploy", || {
            *inner_called.borrow_mut() = true;
            Ok(())
        });

        assert!(result.is_ok());
        assert!(*inner_called.borrow(), "Expected inner to be called");
        assert_eq!(
            client.scale_call_count(),
            0,
            "Expected no scale calls when replicas already at 0"
        );
    }

    #[test]
    fn run_with_scaling_nonzero_replicas_scales_down_runs_inner_scales_up() {
        // Replica response sequence:
        // 1. Initial count query                         → 2
        // 2. scale_down → scale(): prev check            → 2 (not yet at target 0)
        // 3. scale_down → scale(): first poll            → 0 (target reached)
        // 4. scale_up   → scale(): prev check            → 0 (not yet at target 2)
        // 5. scale_up   → scale(): first poll            → 2 (target reached)
        let client = MockK8sClient::new(vec![2, 2, 0, 0, 2]);
        let inner_called = RefCell::new(false);

        let result = run_with_scaling(&client, "ns", "deploy", || {
            *inner_called.borrow_mut() = true;
            Ok(())
        });

        assert!(result.is_ok());
        assert!(*inner_called.borrow(), "Expected inner to be called");
        let calls = client.scale_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].2, 0, "First scale call should scale down to 0");
        assert_eq!(calls[1].2, 2, "Second scale call should scale back up to 2");
    }

    #[test]
    fn run_with_scaling_inner_failure_still_scales_up() {
        // Even when inner fails, scale-up must still be attempted
        let client = MockK8sClient::new(vec![2, 2, 0, 0, 2]);

        let result = run_with_scaling(&client, "ns", "deploy", || {
            Err(anyhow!("backup failed"))
        });

        assert!(result.is_err(), "Expected the inner error to be propagated");
        let calls = client.scale_calls();
        assert_eq!(
            calls.len(),
            2,
            "Expected both scale-down and scale-up even when inner fails"
        );
        assert_eq!(calls[0].2, 0);
        assert_eq!(calls[1].2, 2);
    }

    #[test]
    fn run_with_scaling_propagates_inner_error_message() {
        let client = MockK8sClient::new(vec![0]);

        let result = run_with_scaling(&client, "ns", "deploy", || {
            Err(anyhow!("specific inner error"))
        });

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("specific inner error"));
    }
}
