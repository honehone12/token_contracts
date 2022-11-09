#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod erc20 {

    use ink_storage::{ traits::SpreadAllocate, Mapping };

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        InsufficientBalance,
        AttemptingSelfTransfer,
        InsufficientAllowance,
        AttemptingSelfAllowance
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance
    }

    /// Defines the storage of your contract.
    /// Add new fields to the below struct in order
    /// to add new static storage fields to your contract.
    #[ink(storage)]
    #[derive(SpreadAllocate)]
    pub struct Erc20 {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowance: Mapping<(/*owner*/AccountId, /*spender*/AccountId), Balance>
    }

    // constructors
    impl Erc20 {
        #[ink(constructor)]
        pub fn new(init_supply: Balance) -> Self {
            ink_lang::utils::initialize_contract(|cnt: &mut Self| {
                cnt.new_init_impl(init_supply)
            })
        }
        
        fn new_init_impl(&mut self, init_supply: Balance) {
            let caller = Self::env().caller();
            self.balances.insert(&caller, &init_supply);
            self.total_supply = init_supply;
            self.env().emit_event(Transfer {
                from: None,
                to: Some(caller),
                value: init_supply
            });
        }
        
        /// Constructors can delegate to other constructors.
        #[ink(constructor)]
        pub fn default() -> Self {
            ink_lang::utils::initialize_contract(|cnt: &mut Self| {
                cnt.new_init_impl(u128::MAX)
            })
        }

        /// A message that can be called on instantiated contracts.
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_impl(&owner)
        }

        #[inline]
        fn balance_of_impl(&self, owner: &AccountId) -> Balance {
            self.balances.get(owner).unwrap_or_default()
        }

        /// Simply returns the current value
        #[ink(message)]
        pub fn total_supply(&self) -> Balance {
            self.total_supply
        }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let src = self.env().caller();
            // ensure src is not dest
            if src == to {
                return Err(Error::AttemptingSelfTransfer)
            }

            self.transfer_from_to(&src, &to, value)
        }

        // third party transfer
        #[ink(message)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance
        ) -> Result<()> {
            // ensure src is not dest
            if from == to {
                return Err(Error::AttemptingSelfTransfer)
            }

            let caller = self.env().caller();
            // !!caller has to be allowed, not to or from 
            let allowance = self.allowance_impl(&from, &caller);
            // ensure caller(third party) has enough allowance
            if allowance < value {
                return Err(Error::InsufficientAllowance)
            }

            self.transfer_from_to(&from, &to, value)?;
            self.allowance.insert((&from, &to), &(allowance - value));
            Ok(())
        }

        fn transfer_from_to(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance
        ) -> Result<()> {
            let src_balance = self.balance_of_impl(from);
            // ensure from has enough balance
            if src_balance < value {
                return Err(Error::InsufficientBalance)
            }

            self.balances.insert(from, &(src_balance - value));
            let dst_balance = self.balance_of_impl(to);
            self.balances.insert(to, &(dst_balance + value));
            self.env().emit_event(Transfer{
                from: Some(*from),
                to: Some(*to),
                value
            });
            Ok(())
        }

        #[ink(message)]
        pub fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            // ensure owner is not spender
            if owner == spender {
                return Err(Error::AttemptingSelfAllowance)
            }
            // ensure owner has enough balance
            if self.balance_of_impl(&owner) < value {
                return Err(Error::InsufficientBalance)
            }

            self.allowance.insert((&owner, &spender), &value);
            self.env().emit_event(Approval{
                owner,
                spender,
                value
            });
            Ok(())
        }

        #[ink(message)]
        pub fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowance_impl(&owner, &spender)
        }
        
        fn allowance_impl(&self, owner: &AccountId, spender: &AccountId) -> Balance {
            self.allowance.get((owner, spender)).unwrap_or_default()
        }
    }

    /// Unit tests in Rust are normally defined within such a `#[cfg(test)]`
    /// module and test functions are marked with a `#[test]` attribute.
    /// The below code is technically just normal Rust code.
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        //use super::*;

        /// Imports `ink_lang` so we can use `#[ink::test]`.
        use ink_lang as ink;

        /// We test if the default constructor does its job.
        #[ink::test]
        fn something_works() {
            
        }
    }
}
