use crate::k8s::K8sConfig;
use anyhow::Result;
use rustls::RootCertStore;
use rustls_native_certs::load_native_certs;
use std::fs::File;
use std::io::BufReader;
use tracing::warn;

pub fn install(config: &K8sConfig) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();

    // Add certs from native store.
    let cert_result = load_native_certs();
    
    cert_result
        .errors
        .iter()
        .for_each(|error| warn!(ex=?error, "Encountered an error while loading a certificate from the native store."));
    
    for native_cert in cert_result.certs {
        roots.add(native_cert)?;
    }

    // Load k8s cert.
    let path = &config.cacrt_path;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader);
    for cert in certs {
        roots.add(cert?)?;
    }

    Ok(roots)
}
