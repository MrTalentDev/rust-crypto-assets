#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

// subsa smart contract
#[ink::contract]
mod subsa {
    use ink_storage::{traits::SpreadAllocate, Mapping};

    use scale::{Decode, Encode};

    pub type AssetId = AccountId;

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    pub struct Subsa {
        // immutable asset params
        asset_id: AssetId,
        creator: AccountId,
        asset_name: String,
        unit_name: String,
        total: Balance,
        decimals: u32,
        default_frozen: bool,
        url: String,
        metadata_hash: [u8; 4],
        // mutable asset params
        managerId: AccountId,
        reserveId: AccountId,
        freezeId: AccountId,
        clawbackId: AccountId,
        balances: Mapping<AccountId, Balance>,
        accounts_opted_in: Mapping<AccountId, bool>,
        frozen_holders: Mapping<AccountId, bool>,
    }

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        NotManagerId,
        NotReserveId,
        NotFreezeId,
        NotClawbackId,
        NotOptedIn,
        AlreadyOptedIn,
        NotFrozen,
        NotFreezable,
        AlreadyFrozen,
        FrozenAccount,
        NotEnoughBalance,
        ZeroAmount,
    }

    /// Event emitted when a token transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        sender: AccountId,
        #[ink(topic)]
        receiver: AccountId,
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        amount: Option<Balance>,
    }

    /// Event emitted when an asset is created.
    #[ink(event)]
    pub struct Creation {
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        asset_name: String,
        #[ink(topic)]
        creator: AccountId,
        #[ink(topic)]
        total: Balance,
    }

    /// Event emitted when an asset is frozen.
    /// Note: only the freeze account can freeze an account.
    #[ink(event)]
    pub struct Freeze {
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        account: AccountId,
        #[ink(topic)]
        freeze_id: AccountId,
        #[ink(topic)]
        freeze: bool,
    }

    /// Event emitted when an asset is reconfigured.
    /// Note: only the manager can reconfigure an asset.
    /// Note: the manager can change the reserve, freeze, and clawback addresses.
    #[ink(event)]
    pub struct Modify {
        #[ink(topic)]
        manager_id: AccountId,
        #[ink(topic)]
        reserve_id: AccountId,
        #[ink(topic)]
        freeze_id: AccountId,
        #[ink(topic)]
        clawback_id: AccountId,
    }

    /// Event emitted when an account opts in to receive an asset.
    #[ink(event)]
    pub struct OptIn {
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an account opts out of receiving an asset.
    /// Note: only accounts that have opted in can opt out.
    #[ink(event)]
    pub struct OptOut {
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        account: AccountId,
    }

    /// Event emitted when an asset is revoked.
    /// Note: only the manager address can revoke an asset.
    #[ink(event)]
    pub struct Revoke {
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        clawback: AccountId,
        #[ink(topic)]
        amount: Option<Balance>,
    }

    /// Event emitted when an asset is destroyed.
    /// Note: this can only happen if there are no remaining asset holdings.
    /// Note: only the manager can destroy an asset.
    #[ink(event)]
    pub struct Destruction {
        #[ink(topic)]
        asset_id: AssetId,
        #[ink(topic)]
        destroyer: AccountId,
    }

    impl Subsa {
        #[ink(constructor)]
        pub fn new(
            asset_name: String,
            unit_name: String,
            total: Balance,
            decimals: u32,
            default_frozen: bool,
            url: String,
            metadata_hash: [u8; 4],
            manager: Option<AccountId>,
            reserve: Option<AccountId>,
            freeze: Option<AccountId>,
            clawback: Option<AccountId>,
        ) -> Self {
            Self {
                creator: Self::env().caller(),
                asset_name,
                unit_name,
                total,
                decimals,
                default_frozen,
                url,
                metadata_hash,
                asset_id: Self::env().account_id(),
                managerId: manager.unwrap_or_else(|| AccountId::from([0x0; 32])),
                reserveId: reserve.unwrap_or_else(|| AccountId::from([0x0; 32])),
                freezeId: freeze.unwrap_or_else(|| AccountId::from([0x0; 32])),
                clawbackId: clawback.unwrap_or_else(|| AccountId::from([0x0; 32])),
                balances: Mapping::default(),
                accounts_opted_in: Mapping::default(),
                frozen_holders: Mapping::default(),
            }
        }

        /// Transfer `amount` of tokens from `sender` to `receiver`.
        #[ink(message)]
        pub fn transfer(&mut self, receiver: AccountId, amount: Balance) -> Result<(), Error> {
            let sender = self.env().caller();

            // check if sender has enough balance
            let sender_balance = self.balances.get(&sender).unwrap_or(0);
            if sender_balance < amount {
                return Err(Error::NotEnoughBalance);
            }

            // check if receiver has opted in
            let receiver_opted_in = self.accounts_opted_in.get(&receiver).unwrap_or(false);
            if !receiver_opted_in {
                return Err(Error::NotOptedIn);
            }

            // update sender and receiver balances
            self.balances.insert(&sender, &(sender_balance - amount));
            self.balances.insert(
                &receiver,
                &(self.balances.get(&receiver).unwrap_or(0) + amount),
            );

            // emit transfer event
            self.env().emit_event(Transfer {
                sender,
                receiver,
                asset_id: self.asset_id,
                amount: Some(amount),
            });

            Ok(())
        }

        // OptIn to receive an asset
        #[ink(message)]
        pub fn opt_in(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();

            // check if caller has already opted in
            let caller_opted_in = self.accounts_opted_in.get(&caller).unwrap_or(false);
            if caller_opted_in {
                return Err(Error::AlreadyOptedIn);
            }

            // update caller's opt in status
            self.accounts_opted_in.insert(&caller, &true);

            // emit opt in event
            self.env().emit_event(OptIn {
                asset_id: self.asset_id,
                account: caller,
            });

            Ok(())
        }

        // OptOut of receiving an asset
        #[ink(message)]
        pub fn opt_out(&mut self) -> Result<(), Error> {
            let caller = self.env().caller();

            // check if caller has opted in
            let caller_opted_in = self.accounts_opted_in.get(&caller).unwrap_or(false);
            if !caller_opted_in {
                return Err(Error::NotOptedIn);
            }

            // update caller's opt in status
            self.accounts_opted_in.insert(&caller, &false);

            // emit opt out event
            self.env().emit_event(OptOut {
                asset_id: self.asset_id,
                account: caller,
            });

            Ok(())
        }

        // Freeze an account
        #[ink(message)]
        pub fn freeze(&mut self, account: AccountId, freeze: bool) -> Result<(), Error> {
            let caller = self.env().caller();

            // check if token can be frozen
            if !self.default_frozen {
                return Err(Error::NotFreezable);
            }

            // check if caller is the freeze address
            if caller != self.freezeId {
                return Err(Error::NotFreezeId);
            }

            // check if account is already frozen
            let account_frozen = self.frozen_holders.get(&account).unwrap_or(false);
            if account_frozen {
                return Err(Error::AlreadyFrozen);
            }

            // update account's frozen status
            self.frozen_holders.insert(&account, &freeze);

            // emit freeze event
            self.env().emit_event(Freeze {
                asset_id: self.asset_id,
                account,
                freeze,
                freeze_id: self.freezeId,
            });

            Ok(())
        }

        // Modify/Reconfigure an asset
        // Note: only the manager can modify an asset
        // Note: only mutable asset params can be modified
        // List of mutable asset params:
        // - managerId, reserveId, freezeId, clawbackId
        #[ink(message)]
        pub fn modify_asset(
            &mut self,
            manager: Option<AccountId>,
            reserve: Option<AccountId>,
            freeze: Option<AccountId>,
            clawback: Option<AccountId>,
        ) -> Result<(), Error> {
            let caller = self.env().caller();

            // check if caller is the manager
            if caller != self.managerId {
                return Err(Error::NotManagerId);
            }

            // update asset params
            self.managerId = manager.unwrap_or_else(|| AccountId::from([0x0; 32]));
            self.reserveId = reserve.unwrap_or_else(|| AccountId::from([0x0; 32]));
            self.freezeId = freeze.unwrap_or_else(|| AccountId::from([0x0; 32]));
            self.clawbackId = clawback.unwrap_or_else(|| AccountId::from([0x0; 32]));

            // emit modify asset event
            self.env().emit_event(Modify {
                manager_id: self.managerId,
                reserve_id: self.reserveId,
                freeze_id: self.freezeId,
                clawback_id: self.clawbackId,
            });

            Ok(())
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn default_works() {}
    }
}
