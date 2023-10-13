mod client;

mod cert;
mod config;
mod model;
pub mod scale;

use client::{DefaultK8sClient, K8sClient};
use config::K8sConfig;
