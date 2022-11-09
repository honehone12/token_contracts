#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod erc721 {

    use ink_storage::{
        traits::{ SpreadAllocate, PackedLayout, SpreadLayout },
        Mapping
    };
    use scale::{ Encode, Decode };

    #[derive(Encode, Decode, Debug, PartialEq, Eq, Copy, Clone)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        ReachingMax,
        ReachingMin,
        StorageDataInconsistency,

        TokenAlreadyExists,
        TokenNotFound,
        NotTokenOwner,
        NotApproved,
        ApproveForSelf,
        TrasnferToSelf,
        AccountToBurn,

        NotEnoughFee,
        FounderOnly,
        NotEnoughBalance
    }

    // https://ink.substrate.io/faq#how-do-i-hash-a-value
    type TokenId = Hash;
    type TokenSecret = Hash;
    type TokenGen = ink_prelude::string::String;
    type TokenPhrase = ink_prelude::string::String;
    type TokenGenPhrasePairs = ink_prelude::vec::Vec<(TokenGen, TokenPhrase)>;
    type GenHashing = ink_env::hash::Blake2x256;
    type Erc721Result = Result<(), Error>;
    type InternalResult = Result<(), ()>;
    type ApprovalPair = (/*owner*/AccountId, /*operator*/AccountId);
    type ApprovedAccounts = ink_prelude::vec::Vec<AccountId>;

    const MINT_FEE: Balance = 100;
    const BURN_FEE: Balance = 100;
    const TRANSFER_FEE: Balance = 100;

    #[derive(
        Debug, PartialEq, Eq, Clone, Copy,
        Encode, Decode, SpreadLayout, PackedLayout
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    enum ApprovalScope {
        All
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: TokenId
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: TokenId
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        approved: bool
    }

    #[ink(storage)]
    #[derive(Default, SpreadAllocate)]
    pub struct Erc721 {
        founder: AccountId,
        burn_account: AccountId,

        token_owners: Mapping<TokenId, AccountId>,
        token_secrets: Mapping<TokenId, TokenSecret>,
        owned_tokens_count: Mapping<AccountId, u32>,

        // actuary dont need to be vec.
        token_approvals: Mapping<TokenId, ApprovedAccounts>,
        operator_approvals: Mapping<ApprovalPair, ApprovalScope>
    }

    impl Erc721 {
        #[ink(constructor)]
        pub fn new() -> Self {
            ink_lang::utils::initialize_contract(|cnt: &mut Self| {
                cnt.init_contract_internal();
            })
        }

        // self construct
        #[inline]
        fn init_contract_internal(&mut self) {
            self.founder = self.env().caller();
            self.burn_account = AccountId::from([0x00; 32]);
        }

        #[ink(message)]
        pub fn collect_funded_all(&mut self) -> Erc721Result {
            let caller = self.env().caller();
            if caller != self.founder {
                return Err(Error::FounderOnly)
            }

            let amount = self.env().balance();
            if self.transfer_funded_internal(&amount).is_err() {
                return Err(Error::NotEnoughBalance)
            }
            Ok(())
        }

        #[inline]
        fn transfer_funded_internal(&self, amount: &Balance
        ) -> ink_env::Result<()> {
            self.env().transfer(self.founder, *amount)
        }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> u32 {
            self.balance_of_or_zero_internal(&owner)
        }

        #[inline]
        fn balance_of_or_zero_internal(&self, owner: &AccountId) -> u32 {
            self.owned_tokens_count.get(owner).unwrap_or(0)
        }

        #[ink(message)]
        pub fn owner_of(&self, gen: TokenGen) -> Option<AccountId> {
            let id = self.string_to_hash_internal(&gen);
            self.owner_of_internal(&id)
        }

        #[inline]
        fn owner_of_internal(&self, id: &TokenId) -> Option<AccountId> {
            self.token_owners.get(id)
        }

        #[ink(message)]
        pub fn get_approved(&self, gen: TokenGen) -> Option<ApprovedAccounts> {
            let id = self.string_to_hash_internal(&gen); 
            self.get_approved_internal(&id)
        }

        #[inline]
        fn get_approved_internal(&self, id: &TokenId) -> Option<ApprovedAccounts> {
            self.token_approvals.get(&id)
        }

        #[ink(message)]
        pub fn is_approved_for_all(
            &self, owner: AccountId, operator: AccountId
        ) -> bool {
            self.is_approved_for_all_internal(&owner, &operator)
        }

        #[inline]
        fn is_approved_for_all_internal(
            &self, owner: &AccountId, operator: &AccountId
        ) -> bool {
            self.operator_approvals.contains((owner, operator))
        }

        fn is_approved_internal(
            &self, owner: &AccountId, id: &TokenId, operator: &AccountId
        ) -> bool {
            if self.is_approved_for_all_internal(owner, operator) {
                return true
            }
                      
            match self.get_approved_internal(id) {
                Some(v) => {
                    return v.contains(operator)
                },
                None => {
                    return false
                }
            }
        }

        #[ink(message)]
        pub fn change_phrase(
            &mut self, gen: TokenGen,
            current_phrase: TokenPhrase, new_phrase: TokenPhrase
        ) -> Erc721Result {
            let caller = self.env().caller();
            let id = self.string_to_hash_internal(&gen);
            let owner = self.token_owners.get(&id).ok_or(Error::TokenNotFound)?;
            if caller != owner {
                return Err(Error::NotTokenOwner)
            }

            let current_secret = self.string_to_hash_internal(&current_phrase);
            let stored_secret = self.token_secrets.get(&id).ok_or(Error::StorageDataInconsistency)?;
            if current_secret != stored_secret {
                return Err(Error::NotTokenOwner)
            }

            self.change_phrase_internal(&id, &new_phrase);
            Ok(())
        }

        #[inline]
        fn change_phrase_internal(
            &mut self, id: &TokenId, new_phrase: &TokenPhrase
        ) {
            let new_secret = self.string_to_hash_internal(&new_phrase);
            self.token_secrets.insert(&id, &new_secret);
        }

        #[ink(message, payable)]
        pub fn mint(&mut self, gen: TokenGen, phrase: TokenPhrase
        ) -> Erc721Result {
            let caller = self.env().caller();
            if caller == self.burn_account {
                return Err(Error::AccountToBurn)
            }

            if self.env().transferred_value() < MINT_FEE {
                return Err(Error::NotEnoughFee)
            } 
            

            let id = self.string_to_hash_internal(&gen);
            let secret = self.string_to_hash_internal(&phrase);
            if self.token_owners.contains(&id) {
                return Err(Error::TokenAlreadyExists)
            }
            if self.token_secrets.contains(&secret) {
                return Err(Error::StorageDataInconsistency)
            }
            
            self.add_new_token_to_internal(&caller, &id)?;
            self.token_secrets.insert(&id, &secret);
            self.env().emit_event(
                Transfer {
                    from: Some(self.burn_account),
                    to: Some(caller),
                    id,
                }
            );
            Ok(())
        }
 
        fn string_to_hash_internal(&self, gen: &TokenGen) -> TokenId {
            let input = gen.as_bytes();
            let mut output 
                = <GenHashing as ink_env::hash::HashOutput>::Type::default();
            ink_env::hash_bytes::<GenHashing>(input, &mut output);
            TokenId::from(output)
        }

        fn add_new_token_to_internal(&mut self, to: &AccountId, id: &TokenId
        ) -> Erc721Result {
            let count = self.owned_tokens_count.get(to).unwrap_or(0);
            let new_count = count.checked_add(1).ok_or(Error::ReachingMax)?;
            
            self.owned_tokens_count.insert(to, &new_count);
            self.token_owners.insert(id, to);
            Ok(())
        }

        #[ink(message, payable)]
        pub fn burn(&mut self, gen: TokenGen, phrase: TokenPhrase
        ) -> Erc721Result {
            let caller = self.env().caller();
            let id = self.string_to_hash_internal(&gen);
            let secret = self.string_to_hash_internal(&phrase);
            let owner = self.token_owners.get(&id).ok_or(Error::TokenNotFound)?;
            if owner != caller {
                return Err(Error::NotTokenOwner)
            }
            let stored_secret = 
                self.token_secrets.get(&id).ok_or(Error::StorageDataInconsistency)?;
            if secret != stored_secret {
                return Err(Error::NotTokenOwner)
            }

            if self.env().transferred_value() < BURN_FEE {
                return Err(Error::NotEnoughFee)
            }

            self.remove_token_internal(&owner, &id)?;
            // !!!!!!!!!!!!!!!!!!!!!!!
            // secret is removed here.
            self.token_secrets.remove(&id);
            self.env().emit_event(
                Transfer {
                    from: Some(caller),
                    to: Some(self.burn_account),
                    id
                }
            );
            Ok(())
        }

        fn remove_token_internal(&mut self, owner: &AccountId, id: &TokenId
        ) -> Erc721Result {
            // !!!!!!!!!!!!!!!!!!!!!!!!!
            // approval is removed here.
            if self.token_approvals.contains(&id) {
                self.token_approvals.remove(&id);
            }

            let count = self.owned_tokens_count
                .get(owner)
                .ok_or(Error::StorageDataInconsistency)?;
            let new_count = count.checked_sub(1).ok_or(Error::ReachingMin)?;
            self.owned_tokens_count.insert(owner, &new_count);
            self.token_owners.remove(&id);
            Ok(())
        }

        #[ink(message)]
        pub fn approve(
            &mut self, to: AccountId, gen: TokenGen, phrase: TokenPhrase
        ) -> Erc721Result {
            let id = self.string_to_hash_internal(&gen);
            let secret = self.string_to_hash_internal(&phrase);
            let owner = self.owner_of_internal(&id).ok_or(Error::TokenNotFound)?;
            if self.env().caller() != owner {
                return Err(Error::NotTokenOwner)
            } // then caller is owner
            let stored_secret 
                = self.token_secrets.get(&id).ok_or(Error::StorageDataInconsistency)?;
            if secret != stored_secret {
                return Err(Error::NotTokenOwner)
            }

            if to == self.burn_account {
                return Err(Error::AccountToBurn)
            }

            if to == owner {
                return Err(Error::ApproveForSelf)
            } 

            if self.approve_for_token_internal(&to, &id).is_ok() {
                self.env().emit_event(
                    Approval {
                        from: owner,
                        to,
                        id
                    }
                );
            }
            Ok(())
        }

        fn approve_for_token_internal(&mut self, to: &AccountId, id: &TokenId
        ) -> InternalResult {
            match self.token_approvals.get(id) {
                Some(mut v) => {
                    if !v.contains(to) {
                        v.push(*to);
                        self.token_approvals.insert(id, &v);
                    } else {
                        return Err(())
                    }
                },
                None => {
                    self.token_approvals.insert(id, &ink_prelude::vec![*to]);
                }                
            }
            Ok(())
        }

        #[ink(message)]
        pub fn set_approval_for_all(
            &mut self, to: AccountId, approved: bool, pairs: TokenGenPhrasePairs
        ) -> Erc721Result {
            let caller = self.env().caller();
            if caller == to {
                return Err(Error::ApproveForSelf)
            }

            for (gen, ph) in pairs.iter() {
                let id = self.string_to_hash_internal(gen);
                let secret = self.string_to_hash_internal(ph);
                let stored_secret 
                    = self.token_secrets.get(&id).ok_or(Error::StorageDataInconsistency)?;
                if secret != stored_secret {
                    return Err(Error::NotTokenOwner)
                }
            }

            if to == self.burn_account || caller == self.burn_account {
                return Err(Error::AccountToBurn)
            }

            if self.approve_for_all_internal(&caller, &to, approved).is_ok() {
                self.env().emit_event(
                    ApprovalForAll {
                        from: caller,
                        to,
                        approved
                    }
                );
            }
            Ok(())
        }

        fn approve_for_all_internal(
            &mut self, owner: &AccountId, to: &AccountId, approved: bool
        ) -> InternalResult {
            if approved {
                if !self.operator_approvals.contains((owner, to)) {
                    self.operator_approvals.insert((owner, to), &ApprovalScope::All);
                    return Ok(())
                }
            } else {
                if self.operator_approvals.contains((owner, to)) {
                    self.operator_approvals.remove((owner, to));
                    return Ok(())
                }
            }
            Err(())
        }

        #[ink(message, payable)]
        pub fn transfer(
            &mut self, to: AccountId, gen: TokenGen, phrase: TokenPhrase
        ) -> Erc721Result {
            let id = self.string_to_hash_internal(&gen);
            let secret = self.string_to_hash_internal(&phrase);
            let owner = self.owner_of_internal(&id).ok_or(Error::TokenNotFound)?;
            if owner != self.env().caller() {
                return Err(Error::NotTokenOwner)
            } // then caller is owner
            let stored_secret 
                = self.token_secrets.get(&id).ok_or(Error::StorageDataInconsistency)?;
            if secret != stored_secret {
                return Err(Error::NotTokenOwner)
            }

            if owner == to {
                return Err(Error::TrasnferToSelf)
            }

            if to == self.burn_account {
                return Err(Error::AccountToBurn)
            }

            if self.env().transferred_value() < TRANSFER_FEE {
                return Err(Error::NotEnoughFee)
            }

            self.transfer_token_from_internal(&owner, &to, &id)?;
            self.env().emit_event(
                Transfer {
                    from: Some(owner),
                    to: Some(to),
                    id
                }
            );
            Ok(())
        }

        #[ink(message, payable)]
        pub fn transfer_from(&mut self, from: AccountId, to: AccountId, gen: TokenGen
        ) -> Erc721Result {
            let id = self.string_to_hash_internal(&gen);
            let owner = self.owner_of_internal(&id).ok_or(Error::TokenNotFound)?;
            if owner != from {
                return Err(Error::StorageDataInconsistency)
            }
            
            let caller = self.env().caller();
            if owner != caller {
                if !self.is_approved_internal(&owner, &id, &caller) {
                    return Err(Error::NotApproved)
                }
            } // then caller is owner or approved one

            if from == to {
                return Err(Error::TrasnferToSelf)
            }

            if to == self.burn_account {
                return Err(Error::AccountToBurn)
            }

            if self.env().transferred_value() < TRANSFER_FEE {
                return Err(Error::NotEnoughFee)
            }
            
            self.transfer_token_from_internal(&from, &to, &id)?;
            self.env().emit_event(
                Transfer {
                    from: Some(from),
                    to: Some(to),
                    id
                }
            );
            Ok(())
        }

        fn transfer_token_from_internal(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            id: &TokenId
        ) -> Erc721Result {
            self.remove_token_internal(from, id)?;
            self.add_new_token_to_internal(to, id)?;
            Ok(())
        }
    }


    #[cfg(test)]
    mod tests {
        
        //use super::*;

        use ink_lang as ink;

        #[ink::test]
        fn it_works() {
 
        }
    }
}
