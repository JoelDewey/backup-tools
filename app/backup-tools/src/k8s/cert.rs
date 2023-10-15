use crate::k8s::K8sConfig;
use anyhow::Result;
use rustls::{Certificate, RootCertStore};
use rustls_native_certs::load_native_certs;
use std::fs::File;
use std::io::BufReader;

pub fn install(config: &K8sConfig) -> Result<RootCertStore> {
    let mut roots = RootCertStore::empty();

    // Add certs from native store.
    for native_cert in load_native_certs()? {
        roots.add(&Certificate(native_cert.0))?;
    }

    // Load k8s cert.
    let path = &config.cacrt_path;
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)?;
    for cert in certs {
        roots.add(&Certificate(cert))?;
    }

    Ok(roots)
}
