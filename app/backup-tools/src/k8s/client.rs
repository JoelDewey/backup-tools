use crate::k8s::model::workload::Deployment;
use crate::k8s::{cert, K8sConfig};
use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::sync::Arc;
use tracing::debug;
use ureq::{Error, MiddlewareNext, Request, Response};
use url::Url;

pub trait K8sClient {
    fn get_available_replicas(&self, namespace: &str, name: &str) -> Result<i32>;

    fn scale(&self, namespace: &str, name: &str, count: i32) -> Result<()>;
}

fn logging_middleware(req: Request, next: MiddlewareNext) -> Result<Response, ureq::Error> {
    let method = String::from(req.method());
    let url = String::from(req.url());
    tracing::debug!("K8s Client Begin: {} {}", &method, &url);

    let result = next.handle(req);
    result
        .map(|r| {
            tracing::debug!(
                "K8s Client End: {} {} - {} {}",
                method,
                r.get_url(),
                r.status(),
                r.status_text()
            );

            r
        })
        .map_err(|e: ureq::Error| {
            match &e {
                Error::Status(code, response) => {
                    tracing::error!(
                        "K8s Client HTTP Error: {} {} - {} {}",
                        method,
                        url,
                        code,
                        response.status_text()
                    );
                }
                _ => {
                    tracing::error!(
                        kind=%e.kind(),
                        "K8s Client Error: {} {}",
                        method,
                        url
                    );
                }
            }

            e
        })
}

pub struct DefaultK8sClient {
    kube_base_url: Url,
    token: String,
    agent: ureq::Agent,
}

impl DefaultK8sClient {
    pub fn new(config: &K8sConfig) -> Result<DefaultK8sClient> {
        let kube_base_url = DefaultK8sClient::get_url(config)?;
        let token = DefaultK8sClient::get_token(config)?;
        debug!("Token Byte Length: {}", &token.len());

        let root_store = cert::install(config)?;
        let tls_config = rustls::ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let agent = ureq::AgentBuilder::new()
            .tls_config(Arc::new(tls_config))
            .middleware(logging_middleware)
            .build();

        Ok(DefaultK8sClient {
            kube_base_url,
            token,
            agent,
        })
    }

    fn get_url(config: &K8sConfig) -> Result<Url> {
        let host_with_scheme = format!("https://{}", &config.service_host);
        let mut result = Url::parse(&host_with_scheme)?;
        result
            .set_port(Some(config.service_port_https))
            .map_err(|_| {
                anyhow!(format!(
                    "Failed to add port to Kubernetes service URL: {}",
                    host_with_scheme
                ))
            })?;

        Ok(result)
    }

    fn get_token(config: &K8sConfig) -> Result<String> {
        std::fs::read_to_string(&config.token_path).context("Failed to retrieve Kube token.")
    }
}

impl K8sClient for DefaultK8sClient {
    fn get_available_replicas(&self, namespace: &str, name: &str) -> Result<i32> {
        let path = format!(
            "/apis/apps/v1/namespaces/{}/deployments/{}",
            namespace, name
        );
        let url = self.kube_base_url.join(&path)?;

        let response = self
            .agent
            .get(url.as_str())
            .set("Accept", "application/json")
            .set("Authorization", &format!("Bearer {}", &self.token))
            .call()?
            .into_json::<Deployment>()?;

        response
            .status
            .ok_or_else(|| anyhow!("Failed to retrieve Deployment status."))
            .map(|r| r.available_replicas.unwrap_or(0))
    }

    fn scale(&self, namespace: &str, name: &str, count: i32) -> Result<()> {
        let path = format!(
            "/apis/apps/v1/namespaces/{}/deployments/{}",
            namespace, name
        );
        let url = self.kube_base_url.join(&path)?;
        let body = ScalePatch::new(count);

        self.agent
            .patch(url.as_str())
            .set("Accept", "application/json")
            .set("Authorization", &format!("Bearer {}", &self.token))
            .set("Content-Type", "application/strategic-merge-patch+json")
            .send_json(body)?;

        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct ScalePatchSpec {
    pub replicas: i32,
}

#[derive(Debug, Serialize)]
struct ScalePatch {
    spec: ScalePatchSpec,
}

impl ScalePatch {
    pub fn new(replicas: i32) -> ScalePatch {
        ScalePatch {
            spec: ScalePatchSpec { replicas },
        }
    }
}
