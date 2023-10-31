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
    let _ = span.enter();

    let k8s_config = prefixed(K8S_PREFIX).from_env::<K8sConfig>()?;
    let k8s_client = DefaultK8sClient::new(&k8s_config)?;
    let service_namespace = k8s_config
        .service_namespace
        .clone()
        .or_else(|| get_namespace(&k8s_config))
        .ok_or_else(|| anyhow!("Failed to determine namespace."))?;

    let replica_count = k8s_client
        .get_available_replicas(&service_namespace, &k8s_config.service_deployment_name)
        .context("Retrieving original replica count.")?;

    if replica_count == 0 {
        info!("Deployment replicas already at 0, no scale down needed.")
    } else {
        info!("Scaling down deployment...");
        scale_down(&service_namespace, &k8s_config, &k8s_client)
            .map(|_| {
                info!("Finished scaling down deployment.");
            })
            .context("Failed to scale down deployment.")?;
    }

    let inner_result = inner();
    if inner_result.is_err() {
        error!("Executing inner backup process failed! Attempting to scale deployment up anyway.");
    }

    if replica_count == 0 {
        info!("Deployment replicas were set to 0 initially so no scale up is required.")
    } else {
        match scale_up(&service_namespace, &k8s_config, &k8s_client, replica_count) {
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

fn scale_down(namespace: &str, config: &K8sConfig, client: &impl K8sClient) -> Result<i32> {
    scale(namespace, config, client, 0)
}

fn scale_up(
    namespace: &str,
    config: &K8sConfig,
    client: &impl K8sClient,
    target_replicas: i32,
) -> Result<i32> {
    scale(namespace, config, client, target_replicas)
}

fn scale(
    namespace: &str,
    config: &K8sConfig,
    client: &impl K8sClient,
    target_replicas: i32,
) -> Result<i32> {
    let prev_replicas =
        client.get_available_replicas(namespace, &config.service_deployment_name)?;
    if prev_replicas == target_replicas {
        return Ok(prev_replicas);
    }

    info!("Replica count prior to scale operation: {}", prev_replicas);

    info!("Beginning scale to target replica count of {}; waiting 120 seconds for the scaling to complete.", &target_replicas);
    client.scale(namespace, &config.service_deployment_name, target_replicas)?;

    let delay = Duration::from_secs(1);
    let mut replica_count = -1;
    for i in 0..120 {
        replica_count =
            client.get_available_replicas(namespace, &config.service_deployment_name)?;
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
