use crate::k8s::{DefaultK8sClient, K8sClient, K8sConfig};
use anyhow::{anyhow, Context, Result};
use envy::from_env;
use std::thread::sleep;
use std::time::Duration;
use tracing::{error, info};

pub fn scale_deployment(inner: impl FnOnce() -> Result<()>) -> Result<()> {
    let k8s_config = from_env::<K8sConfig>()?;
    let k8s_client = DefaultK8sClient::new(&k8s_config)?;

    let replica_count = k8s_client
        .get_replica_count(
            &k8s_config.service_namespace,
            &k8s_config.service_deployment_name,
        )
        .context("Retrieving original replica count.")?;

    if replica_count == 0 {
        info!("Deployment replicas already at 0, no scale down needed.")
    } else {
        info!("Scaling down deployment...");
        scale_down(&k8s_config, &k8s_client)
            .map(|_| {
                info!("Finished scaling down deployment.");
            })
            .context("Failed to scale down deployment.")?;
    }

    if let Err(e) = inner() {
        error!(ex=%e, "Executing inner backup process failed! Attempting to scale deployment up anyway.");
    }

    if replica_count == 0 {
        info!("Deployment replicas were set to 0 initially so no scale up is required.")
    } else {
        scale_up(&k8s_config, &k8s_client, replica_count)
            .context("Failed to scale up deployment after performing backups.")?;
    }

    Ok(())
}

fn scale_down(config: &K8sConfig, client: &impl K8sClient) -> Result<i32> {
    scale(config, client, 0)
}

fn scale_up(config: &K8sConfig, client: &impl K8sClient, target_replicas: i32) -> Result<i32> {
    scale(config, client, target_replicas)
}

fn scale(config: &K8sConfig, client: &impl K8sClient, target_replicas: i32) -> Result<i32> {
    let prev_replicas =
        client.get_replica_count(&config.service_namespace, &config.service_deployment_name)?;
    if prev_replicas == target_replicas {
        return Ok(prev_replicas);
    }

    info!("Replica count prior to scale operation: {}", prev_replicas);

    info!("Beginning scale to target replica count of {}; waiting 120 seconds for the scale down to complete.", &target_replicas);
    client.scale(
        &config.service_namespace,
        &config.service_deployment_name,
        target_replicas,
    )?;

    let delay = Duration::from_secs(1);
    let mut replica_count = -1;
    for _ in 0..120 {
        replica_count =
            client.get_replica_count(&config.service_namespace, &config.service_deployment_name)?;
        if replica_count == target_replicas {
            return Ok(replica_count);
        } else {
            info!(
                "Replica count is at {}, waiting a second before checking again.",
                &replica_count
            );
            sleep(delay);
        }
    }

    Err(anyhow!(format!(
        "Failed to scale down application after 120 seconds; replica count: {}",
        replica_count
    )))
}
