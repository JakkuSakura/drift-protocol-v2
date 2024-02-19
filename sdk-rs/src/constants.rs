use std::{str::FromStr, sync::OnceLock};

use drift_program::state::{perp_market::PerpMarket, spot_market::SpotMarket};
pub use drift_program::{
    math::constants::{
        BASE_PRECISION_U64 as BASE_PRECISION, PRICE_PRECISION,
        QUOTE_PRECISION_U64 as QUOTE_PRECISION, SPOT_BALANCE_PRECISION,
    },
    ID as PROGRAM_ID,
};
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

/// Static-ish metadata from onchain drift program
#[derive(Clone)]
pub struct ProgramData {
    pub lookup_table: AddressLookupTableAccount,
}

impl ProgramData {
    /// Return an uninitialized instance of `ProgramData` (useful for bootstrapping)
    pub const fn uninitialized() -> Self {
        Self {
            lookup_table: AddressLookupTableAccount {
                key: Pubkey::new_from_array([0; 32]),
                addresses: vec![],
            },
        }
    }
    /// Initialize `ProgramData`
    pub fn new(lookup_table: AddressLookupTableAccount) -> Self {
        Self { lookup_table }
    }
}
