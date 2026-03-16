use crate::k8s::K8sConfig;
use anyhow::Result;
use rustls_native_certs::load_native_certs;
use std::fs::File;
use std::io::BufReader;
use tracing::warn;
use ureq::tls::Certificate;

pub fn load(config: &K8sConfig) -> Result<Vec<Certificate<'static>>> {
    let mut certs: Vec<Certificate<'static>> = Vec::new();

    // Add certs from native store.
    let cert_result = load_native_certs();

    cert_result
        .errors
        .iter()
        .for_each(|error| warn!(ex=?error, "Encountered an error while loading a certificate from the native store."));

    for native_cert in cert_result.certs {
        certs.push(Certificate::from_der(native_cert.as_ref()).to_owned());
    }

    // Load k8s cert.
    let path = &config.cacrt_path;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    for cert in rustls_pemfile::certs(&mut reader) {
        let cert = cert?;
        certs.push(Certificate::from_der(cert.as_ref()).to_owned());
    }

    Ok(certs)
}