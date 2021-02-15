// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::Safe;
use anyhow::{Context, Result};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::{collections::HashSet, env::var, net::SocketAddr};

// Environment variable which can be set with the auth credentials
// to be used for all sn_api tests
const TEST_AUTH_CREDENTIALS: &str = "TEST_AUTH_CREDENTIALS";

// Environment variable which can be set with the bootstraping contacts
// to be used for all sn_api tests
const TEST_BOOTSTRAPPING_PEERS: &str = "TEST_BOOTSTRAPPING_PEERS";

// Instantiate a Safe instance
pub async fn new_safe_instance() -> Result<Safe> {
    let mut safe = Safe::default();
    let credentials = match var(TEST_AUTH_CREDENTIALS) {
        Ok(val) => {
            let keypair = serde_json::from_str(&val).with_context(|| {
                format!(
                    "Failed to parse credentials read from {} env var",
                    TEST_AUTH_CREDENTIALS
                )
            })?;
            Some(keypair)
        }
        Err(_) => None,
    };

    let bootstrap_contacts = get_bootstrap_contacts()?;
    safe.connect(credentials, None, Some(bootstrap_contacts))
        .await?;
    Ok(safe)
}

pub async fn new_read_only_safe_instance() -> Result<Safe> {
    let mut safe = Safe::default();
    let bootstrap_contacts = get_bootstrap_contacts()?;
    safe.connect(None, None, Some(bootstrap_contacts)).await?;
    Ok(safe)
}

// Create a random NRS name
pub fn random_nrs_name() -> String {
    thread_rng().sample_iter(&Alphanumeric).take(15).collect()
}

fn get_bootstrap_contacts() -> Result<HashSet<SocketAddr>> {
    let contacts = match var(TEST_BOOTSTRAPPING_PEERS) {
        Ok(val) => serde_json::from_str(&val).with_context(|| {
            format!(
                "Failed to parse bootstraping contacts list from {} env var",
                TEST_BOOTSTRAPPING_PEERS
            )
        })?,
        Err(_) => {
            // we default to this address as that's what we
            // normally use in local test network as the genesis node address
            vec!["127.0.0.1:12000".parse()?].into_iter().collect()
        }
    };

    Ok(contacts)
}
