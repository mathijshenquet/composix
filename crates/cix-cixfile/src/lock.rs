use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockFile {
    pub nixpkgs: NixpkgsLock,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NixpkgsLock {
    pub url: String,
    pub rev: String,
    #[serde(rename = "narHash")]
    pub nar_hash: String,
}
