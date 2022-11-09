#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;
use ink_env::AccountId;

pub type TokenId64 = u64;
pub type Balance64 = u64;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    StorageInconsistency,
    OperationInconsistency,
    Overflow,
    Underflow,

    NotEnough,
    TooMuch
}

#[derive(
    Debug, Eq, PartialEq, scale::Encode, scale::Decode, Clone, Copy,
    ink_storage::traits::PackedLayout, ink_storage::traits::SpreadLayout
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum TokenBalanceOption {
    NonFungible,
    Some(Balance64)
}

#[derive(
    Debug, Eq, PartialEq, scale::Encode, scale::Decode, Clone,
    ink_storage::traits::PackedLayout, ink_storage::traits::SpreadLayout
)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum ApprovalOption {
    Some(Vec<TokenId64>),
    All
}

pub type Result<T> = core::result::Result<T, Error>;
pub type Vec<T> = ink_prelude::vec::Vec<T>;
pub type String = ink_prelude::string::String;

mod game_contract_interfaces {
    use super::*; 

    #[ink::trait_definition]
    pub trait Erc1155 {
        #[ink(message)]
        fn balance_of(&self, owner: AccountId, token_id: TokenId64) -> Balance64;

        #[ink(message)]
        fn balance_of_batch(&self, owners: Vec<AccountId>, token_ids: Vec<TokenId64>
        ) -> Vec<Balance64>;

        #[ink(message)]
        fn set_approval_for_all(&mut self, operator: AccountId, approved: bool
        ) -> Result<()>;

        #[ink(message)]
        fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool;

        #[ink(message)]
        fn safe_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId64,
            value: Balance64,
            data: Vec<u8>
        ) -> Result<()>;

        #[ink(message)]
        fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_ids: Vec<TokenId64>,
            values: Vec<Balance64>,
            data: Vec<u8>
        ) -> Result<()>;
    }

    #[ink::trait_definition]
    pub trait Erc1155TokenReceiver {
        #[ink(message, selector = 0xf23a6e61)]
        fn on_received(
            &mut self,
            opertor: AccountId,
            from: AccountId,
            token_ids: TokenId64,
            values: Balance64,
            data: Vec<u8>
        ) -> Vec<u8>;

        #[ink(message, selector = 0xbc197c81)]
        fn on_batch_received(
            &mut self,
            opertor: AccountId,
            from: AccountId,
            token_ids: Vec<TokenId64>,
            values: Vec<Balance64>,
            data: Vec<u8>
        ) -> Vec<u8>;
    }
}

#[ink::contract]
mod game_contract {
    use super::*;
    use game_contract_interfaces;

    pub type OwnershipPair = (AccountId, TokenId64);
    pub type ApprovalPair = (AccountId, AccountId);

    const GAME_CURRENCY_ID: TokenId64 = 0;
    const GAME_CURRENCY_INITIAL_AMOUNT: Balance64 = 10_000_000_000_000_000_000;
    const MINT_FEE: Balance64 = 1_000;

    #[ink(storage)]
    #[derive(Default, ink_storage::traits::SpreadAllocate)]
    pub struct GameContract {
        balances: ink_storage::Mapping<OwnershipPair, TokenBalanceOption>,
        approvals: ink_storage::Mapping<ApprovalPair, ApprovalOption>,

        token_variety: TokenId64,
        founder: AccountId
    }

    impl GameContract {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                let contract_addr = contract.env().account_id();
                contract.founder = contract.env().caller();
                contract.token_variety = 1;
                contract.balances.insert(
                    (contract_addr, GAME_CURRENCY_ID),
                    &TokenBalanceOption::Some(GAME_CURRENCY_INITIAL_AMOUNT)
                );
            })
        } 

        #[ink(message)]
        pub fn remained_currency_pool(&self) -> Result<Balance64> {
            self.get_remained_currency_pool()
        }

        #[ink(message, payable)]
        pub fn buy_game_currency(&mut self, amount: Balance64) -> Result<()> {
            if self.env().transferred_value() < amount as u128 {
                return Err(Error::NotEnough)
            }

            let max = self.get_remained_currency_pool()?;
            if amount > max {
                return Err(Error::TooMuch)
            }
            
            let contract_addr = self.env().account_id();
            let caller = self.env().caller();
            self.do_transfer_token(
                GAME_CURRENCY_ID,
                &contract_addr,
                &caller,
                TokenBalanceOption::Some(amount)
            )?;
            Ok(())
        }

        #[ink(message)]
        pub fn mint(&mut self) -> Result<TokenId64> {
            //pay with game currency

            self.do_mint()
        }

        #[inline]
        fn get_remained_currency_pool(&self) -> Result<Balance64> {
            let contract_addr = self.env().account_id();
            let op = self.balances.get((contract_addr, GAME_CURRENCY_ID))
            .ok_or(Error::StorageInconsistency)?;
            match op {
                TokenBalanceOption::NonFungible => Err(Error::StorageInconsistency),
                TokenBalanceOption::Some(b) => Ok(b)
            }
        }

        #[inline]
        fn do_mint(&mut self) -> Result<Balance64> {
            let caller = self.env().caller();
            let next_token_id = self.token_variety.checked_add(1).ok_or(Error::Overflow)?;
            self.balances.insert(
                (caller, next_token_id), &TokenBalanceOption::NonFungible
            );
            self.token_variety = next_token_id;
            Ok(next_token_id)
        }

        fn do_transfer_token(
            &mut self,
            token_id: TokenId64,
            from: &AccountId,
            to: &AccountId,
            amount: TokenBalanceOption
        ) -> Result<()> {
            if token_id == GAME_CURRENCY_ID {
                let balance_move;
                match amount {
                    TokenBalanceOption::NonFungible => {
                        return Err(Error::OperationInconsistency)
                    },
                    TokenBalanceOption::Some(b) => {
                        balance_move = b;
                    }
                }

                match self.balances.get((from, token_id)) {
                    None => {
                        return Err(Error::OperationInconsistency)
                    },
                    Some(TokenBalanceOption::NonFungible) => {
                        return Err(Error::StorageInconsistency)
                    },
                    Some(TokenBalanceOption::Some(b)) => {
                        let new_balance = b.checked_sub(balance_move).ok_or(Error::Underflow)?;
                        self.balances.insert(
                            (from, token_id), &TokenBalanceOption::Some(new_balance)
                        );
                    } 
                }

                match self.balances.get((to, token_id)) {
                    None => {
                        self.balances.insert(
                            (to, token_id),
                            &TokenBalanceOption::Some(balance_move)
                        );
                    },
                    Some(TokenBalanceOption::NonFungible) => {
                        return Err(Error::StorageInconsistency)
                    },
                    Some(TokenBalanceOption::Some(b)) => {
                        let new_balance = b.checked_add(balance_move).ok_or(Error::Overflow)?;
                        self.balances.insert(
                            (to, token_id), &TokenBalanceOption::Some(new_balance)
                        );
                    }
                }
            } else {
                self.balances.remove((from, token_id));
                self.balances.insert((to, token_id), &TokenBalanceOption::NonFungible);
            }
            Ok(())
        }
    }

    #[cfg(test)]
    mod tests {
        use ink_lang as ink;

        #[ink::test]
        fn it_works() {

        }
    }
}
