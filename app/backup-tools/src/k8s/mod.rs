mod client;

mod cert;
mod config;
mod model;
pub mod scale;
mod workload_type;

use client::{DefaultK8sClient, K8sClient};
use config::K8sConfig;
