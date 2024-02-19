use std::{str::FromStr, sync::OnceLock};

use drift_program::state::{perp_market::PerpMarket, spot_market::SpotMarket};
pub use drift_program::{
    math::constants::{
        BASE_PRECISION_U64 as BASE_PRECISION, PRICE_PRECISION,
        QUOTE_PRECISION_U64 as QUOTE_PRECISION, SPOT_BALANCE_PRECISION,
    },
    ID as PROGRAM_ID,
};
use regex::Captures;
use solana_sdk::{address_lookup_table_account::AddressLookupTableAccount, pubkey::Pubkey};
use substreams_solana_macro::b58;

use crate::types::Context;

static STATE_ACCOUNT: OnceLock<Pubkey> = OnceLock::new();

lazy_static::lazy_static! {
    pub static ref TOKEN_PROGRAM_ID: Pubkey = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
}

/// Return the market lookup table
pub(crate) const fn market_lookup_table(context: Context) -> Pubkey {
    match context {
        Context::DevNet => {
            Pubkey::new_from_array(b58!("FaMS3U4uBojvGn5FSDEPimddcXsCfwkKsFgMVVnDdxGb"))
        }
        Context::MainNet => {
            Pubkey::new_from_array(b58!("D9cnvzswDikQDf53k4HpQ3KJ9y1Fv3HGGDFYMXnK5T6c"))
        }
    }
}

/// Drift state account
pub fn state_account() -> &'static Pubkey {
    STATE_ACCOUNT.get_or_init(|| {
        let (state_account, _seed) =
            Pubkey::find_program_address(&[&b"drift_state"[..]], &PROGRAM_ID);
        state_account
    })
}

/// calculate the PDA of a drift spot market given index
pub fn derive_spot_market_account(market_index: u16) -> Pubkey {
    let (account, _seed) = Pubkey::find_program_address(
        &[&b"spot_market"[..], &market_index.to_le_bytes()],
        &PROGRAM_ID,
    );
    account
}

/// calculate the PDA for a drift spot market vault given index
pub fn derive_spot_market_vault(market_index: u16) -> Pubkey {
    let (account, _seed) = Pubkey::find_program_address(
        &[&b"spot_market_vault"[..], &market_index.to_le_bytes()],
        &PROGRAM_ID,
    );
    account
}

/// calculate the PDA for the drift signer
pub fn derive_drift_signer() -> Pubkey {
    let (account, _seed) = Pubkey::find_program_address(&[&b"drift_signer"[..]], &PROGRAM_ID);
    account
}

/// Helper methods for market data structs
pub trait MarketExt {
    fn market_type(&self) -> &'static str;
    fn symbol(&self) -> &str;
}

impl MarketExt for PerpMarket {
    fn market_type(&self) -> &'static str {
        "perp"
    }
    fn symbol(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.name) }.trim_end()
    }
}

impl MarketExt for SpotMarket {
    fn market_type(&self) -> &'static str {
        "spot"
    }
    fn symbol(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.name) }.trim_end()
    }
}
const MAINNET_SPOT_MARKETS: &str = include_str!("mainnet_spot_markets.json");
const MAINNET_PERP_MARKETS: &str = include_str!("mainnet_perp_markets.json");

const DEVNET_SPOT_MARKETS: &str = include_str!("devnet_spot_markets.json");
const DEVNET_PERP_MARKETS: &str = include_str!("devnet_perp_markets.json");

fn fixup_item(s: &str) -> String {
    {
        let s = s.trim_start_matches("\"").trim_end_matches("\"");
        if s.starts_with("-") || hex::decode(s).is_ok() {
            if let Ok(v) = i128::from_str_radix(s, 16) {
                return serde_json::to_string(&v).unwrap();
            }
        }
        if let Ok(key) = Pubkey::from_str(s) {
            return serde_json::to_string(&key.to_bytes()).unwrap();
        }
    }
    s.replace("5Min", "5min").replace("24H", "24h").to_string()
}
fn replace_fixup_input(s: &str) -> String {
    let re = regex::Regex::new(r#"\{\s*(".+?")\s*:\s*\{\s*}\s*}"#).unwrap();
    let s = re.replace_all(&s, |c: &Captures| c.get(1).unwrap().as_str().to_string());
    let re = regex::Regex::new(r#"".+?""#).unwrap();
    let s = re.replace_all(&s, |c: &Captures| fixup_item(c.get(0).unwrap().as_str()));
    s.to_string()
}

/// Static-ish metadata from onchain drift program
pub struct ProgramData {
    spot_markets: Vec<SpotMarket>,
    perp_markets: Vec<PerpMarket>,
    pub lookup_table: AddressLookupTableAccount,
}

impl ProgramData {
    /// Return an uninitialized instance of `ProgramData` (useful for bootstrapping)
    pub const fn uninitialized() -> Self {
        Self {
            spot_markets: vec![],
            perp_markets: vec![],
            lookup_table: AddressLookupTableAccount {
                key: Pubkey::new_from_array([0; 32]),
                addresses: vec![],
            },
        }
    }
    /// Initialize `ProgramData`
    pub fn new(context: Context, lookup_table: AddressLookupTableAccount) -> Self {
        #[derive(serde::Deserialize)]
        struct Wrapper<T> {
            account: T,
        }

        let spot_json;
        let perp_json;
        match context {
            Context::MainNet => {
                spot_json = replace_fixup_input(MAINNET_SPOT_MARKETS);
                perp_json = replace_fixup_input(MAINNET_PERP_MARKETS);
            }
            Context::DevNet => {
                spot_json = replace_fixup_input(DEVNET_SPOT_MARKETS);
                perp_json = replace_fixup_input(DEVNET_PERP_MARKETS);
            }
        }
        // for (i, line) in spot_json.lines().enumerate() {
        //     println!("{}: {}", i + 1, line);
        // }
        let spot_markets: Vec<Wrapper<_>> = serde_json::from_str(&spot_json).unwrap();
        // for (i, line) in perp_json.lines().enumerate() {
        //     println!("{}: {}", i + 1, line);
        // }
        let perp_markets: Vec<Wrapper<_>> = serde_json::from_str(&perp_json).unwrap();

        Self {
            spot_markets: spot_markets.into_iter().map(|x| x.account).collect(),
            perp_markets: perp_markets.into_iter().map(|x| x.account).collect(),
            lookup_table,
        }
    }

    /// Return known spot markets
    pub fn spot_market_configs(&self) -> &[SpotMarket] {
        &self.spot_markets
    }

    /// Return known perp markets
    pub fn perp_market_configs(&self) -> &[PerpMarket] {
        &self.perp_markets
    }

    /// Return the spot market config given a market index
    pub fn spot_market_config_by_index(&self, market_index: u16) -> Option<&SpotMarket> {
        self.spot_markets.get(market_index as usize)
    }

    /// Return the perp market config given a market index
    pub fn perp_market_config_by_index(&self, market_index: u16) -> Option<&PerpMarket> {
        self.perp_markets.get(market_index as usize)
    }
}
