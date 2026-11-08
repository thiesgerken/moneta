use ipnet::IpNet;
use serde::{Deserialize, Deserializer, Serialize};
use std::net::IpAddr;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(default)]
pub struct Config {
    pub verbosity: i8,
    pub database: DatabaseConfig,
    pub api: ApiConfig,
    pub renditier: RenditierConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://moneta:moneta@localhost:5433/moneta".into(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ApiConfig {
    pub secret_key: String,
    pub port: u16,
    pub address: String,

    /// Take the user name in the `Remote-User` header as who is calling, the way a reverse
    /// proxy doing the login hands it over. Only from `trusted_proxies`: anybody else who can
    /// reach the backend could otherwise name any user they like.
    pub trusted_header_authentication: bool,

    /// The peers `Remote-User` is believed from, as CIDRs separated by commas
    /// (`10.42.0.0/16,127.0.0.1/32`). Matched against the TCP peer, never against
    /// `X-Forwarded-For` or `X-Real-IP`, which the caller writes itself.
    #[serde(deserialize_with = "comma_separated")]
    pub trusted_proxies: Vec<IpNet>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            secret_key: String::new(),
            port: 8181,
            address: "127.0.0.1".into(),
            trusted_header_authentication: false,
            trusted_proxies: Vec::new(),
        }
    }
}

impl ApiConfig {
    /// Header authentication without an allowlist would trust nobody and log every request as
    /// an attempt from outside it. That is a deployment that thinks it has SSO and has none, so
    /// it does not start.
    pub fn validate(&self) -> Result<(), String> {
        if self.trusted_header_authentication && self.trusted_proxies.is_empty() {
            return Err("MONETA__API__TRUSTED_HEADER_AUTHENTICATION needs \
                        MONETA__API__TRUSTED_PROXIES, the proxies Remote-User comes from"
                .into());
        }
        Ok(())
    }

    /// Whether `Remote-User` from this peer is to be believed.
    pub fn trusts(&self, peer: IpAddr) -> bool {
        // a dual-stack socket reports an IPv4 peer as ::ffff:a.b.c.d, which no IPv4 network
        // in the allowlist contains
        let peer = peer.to_canonical();
        self.trusted_header_authentication && self.trusted_proxies.iter().any(|n| n.contains(&peer))
    }
}

/// The environment hands over one string; the allowlist is a list of networks.
fn comma_separated<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<IpNet>, D::Error> {
    String::deserialize(deserializer)?
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse()
                .map_err(|e| serde::de::Error::custom(format!("trusted proxy '{entry}': {e}")))
        })
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RenditierConfig {
    /// Where renditier answers, without a trailing slash, e.g. `https://renditier.gerken.haus`.
    /// Empty switches the whole integration off: `investments` then reports nothing, which is
    /// what every deployment that has no renditier wants.
    pub url: String,

    /// Where a *browser* reaches renditier, for the link on a linked depot. In the cluster `url`
    /// is a Service that resolves nowhere outside it, which would make that link dead; where
    /// the two are the same, leaving this empty falls back to `url`.
    pub public_url: String,

    /// How long to wait for it. This sits on a page load, so a renditier that hangs has to
    /// become a missing number quickly rather than a spinner that never resolves.
    pub timeout_seconds: u64,
}

impl Default for RenditierConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            public_url: String::new(),
            timeout_seconds: 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strings, as the environment hands them over.
    fn api(enabled: bool, proxies: &str) -> ApiConfig {
        let config: Config = config::Config::builder()
            .set_override("api.trusted_header_authentication", enabled.to_string())
            .unwrap()
            .set_override("api.trusted_proxies", proxies)
            .unwrap()
            .build()
            .unwrap()
            .try_deserialize()
            .unwrap();
        config.api
    }

    #[test]
    fn header_authentication_is_off_by_default() {
        let api = ApiConfig::default();
        assert!(!api.trusted_header_authentication);
        assert!(!api.trusts("127.0.0.1".parse().unwrap()));
        assert_eq!(api.validate(), Ok(()));
    }

    #[test]
    fn header_authentication_without_an_allowlist_does_not_start() {
        assert!(api(true, "").validate().is_err());
        assert!(api(true, " , ").validate().is_err());
        assert_eq!(api(true, "10.42.0.0/16").validate(), Ok(()));
    }

    #[test]
    fn only_peers_inside_the_allowlist_are_trusted() {
        let api = api(true, "10.42.0.0/16, 127.0.0.1/32,fd00::/8");
        for trusted in ["10.42.3.4", "127.0.0.1", "::ffff:10.42.0.1", "fd12::1"] {
            assert!(api.trusts(trusted.parse().unwrap()), "{trusted}");
        }
        for outside in ["10.43.0.1", "127.0.0.2", "192.0.2.1", "::1"] {
            assert!(!api.trusts(outside.parse().unwrap()), "{outside}");
        }
    }

    #[test]
    fn an_allowlist_is_not_trusted_while_the_header_is_off() {
        assert!(!api(false, "10.42.0.0/16").trusts("10.42.0.1".parse().unwrap()));
    }

    #[test]
    fn a_malformed_allowlist_fails_to_load() {
        // a bare address too: a single host is written as /32, and guessing would hide a typo
        for entry in ["10.42.0.0/33", "10.42.0.5"] {
            let error = config::Config::builder()
                .set_override("api.trusted_proxies", entry)
                .unwrap()
                .build()
                .unwrap()
                .try_deserialize::<Config>()
                .unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains(&format!("trusted proxy '{entry}'")),
                "{error}"
            );
        }
    }
}
