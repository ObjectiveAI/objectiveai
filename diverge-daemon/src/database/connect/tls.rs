//! TLS toward the database, by the mode.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use rustls_pki_types::pem::PemObject as _;
use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

use crate::database::{Ssl, Target};

/// The socket wrapped in TLS: with `verify-ca` and `verify-full` the
/// chain is verified against `sslrootcert` when the URL names one,
/// else the web's roots, and the host name with it — `verify-ca` is
/// held to the stricter of the two, since the daemon has the name;
/// with `require`, `prefer` and `allow` nothing is verified, as libpq
/// has it: the password crosses as a SCRAM exchange either way.
pub async fn secure(tcp: TcpStream, target: &Target) -> Result<TlsStream<TcpStream>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let builder = ClientConfig::builder_with_provider(Arc::clone(&provider))
        .with_safe_default_protocol_versions()
        .map_err(|error| error.to_string())?;
    let config = match target.ssl {
        Ssl::VerifyCa | Ssl::VerifyFull => {
            let mut roots = RootCertStore::empty();
            match &target.root_cert {
                Some(path) => {
                    for certificate in CertificateDer::pem_file_iter(path).map_err(|error| format!("{}: {error}", path.display()))? {
                        let certificate = certificate.map_err(|error| format!("{}: {error}", path.display()))?;
                        roots.add(certificate).map_err(|error| error.to_string())?;
                    }
                }
                None => roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned()),
            }
            builder.with_root_certificates(roots).with_no_client_auth()
        }
        Ssl::Disable | Ssl::Allow | Ssl::Prefer | Ssl::Require => builder
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(Unverified {
                schemes: provider.signature_verification_algorithms.supported_schemes(),
            }))
            .with_no_client_auth(),
    };
    let name = ServerName::try_from(target.host.clone()).map_err(|error| format!("{}: {error}", target.host))?;
    TlsConnector::from(Arc::new(config))
        .connect(name, tcp)
        .await
        .map_err(|error| format!("TLS to {} failed: {error}", target.host))
}

/// A verifier that accepts any certificate: what `require` means in
/// libpq, encryption without identity.
#[derive(Debug)]
struct Unverified {
    schemes: Vec<SignatureScheme>,
}

impl ServerCertVerifier for Unverified {
    fn verify_server_cert(
        &self,
        _: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(&self, _: &[u8], _: &CertificateDer<'_>, _: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes.clone()
    }
}
