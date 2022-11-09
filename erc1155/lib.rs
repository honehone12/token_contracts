#![cfg_attr(not(feature = "std"), no_std)]

use ink_lang as ink;

#[ink::contract]
mod erc1155 {
    use ink_storage::{ traits::{SpreadAllocate, PackedLayout, SpreadLayout}, Mapping };
    
    #[derive(Debug, PartialEq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum Error {
        // user errors
        PermissionDenied,
        NotFound,
        AlreadyExist,
        WrongArgument,
        NotEnoughFee,
        NotAvailable,
        BadAdress,
        SelfOperation,

        // system errors
        Overflow,
        Underflow,
        StorageInconsistency,

        // compatibility errors
        TransferDenied
    }

    #[derive(
        Debug, Eq, PartialEq, scale::Encode, scale::Decode,
        PackedLayout, SpreadLayout, Clone, Copy
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum ApprovalScope {
        Some,
        All
    }

    #[derive(
        Debug, Eq, PartialEq, scale::Encode, scale::Decode,
        PackedLayout, SpreadLayout, Clone, Copy
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum TokenKind {
        Ft,
        Nft
    }

    #[derive(
        Debug, Eq, PartialEq, scale::Encode, scale::Decode,
        PackedLayout, SpreadLayout, Clone, Copy
    )]
    #[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
    pub enum AmountOption {
        None,
        Some(Balance),
        Max,
    }
    
    pub type TokenId = u128;
    pub type TokenIdList = ink_prelude::vec::Vec<TokenId>;
    pub type TokenName = ink_prelude::string::String;
    pub type TokenTypesList
        = ink_prelude::vec::Vec<(TokenId, TokenName, TokenKind)>;
    pub type NftIdentity = Hash;
    pub type NftIdentityList = ink_prelude::vec::Vec<NftIdentity>;
    pub type BalanceList
        = ink_prelude::vec::Vec<Balance>;
    pub type BatchBalanceList
        = ink_prelude::vec::Vec<ink_prelude::vec::Vec<Balance>>;
    pub type NftGen = ink_prelude::string::String;
    pub type NftGenList
        = ink_prelude::vec::Vec<ink_prelude::string::String>;
    pub type NftGenHashing = ink_env::hash::Blake2x256;
    pub type AccountIdList = ink_prelude::vec::Vec<AccountId>;
    pub type BytesVec = ink_prelude::vec::Vec<u8>;
    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct TransferSingle {
        #[ink(topic)]
        operator: Option<AccountId>,
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        token_id: TokenId,
        value: Balance
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        id: TokenId
    }

    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool
    }

    const DEFAULT_CURRENCY_NAME: &str = "SKMiZ";
    const MAX_CURRENCY_AMOUNT: Balance = u128::MAX;
    const CURRENCY_TOKEN_ID: TokenId = 0;
    const MINT_FEE: Balance = 100;
    const BUY_AMOUNT_500: Balance = 500;

    const ON_ERC1155_RECEIVED_SELECTOR: [u8; 4] = [0xf2, 0x3a, 0x6e, 0x61];
    const ON_ERC1155_BATCH_RECEIVED_SELECTOR: [u8; 4] = [0xbc, 0x19, 0x7c, 0x81];

    #[ink(storage)]
    #[derive(Default, SpreadAllocate)]
    pub struct Erc1155Contract {
        founder: AccountId,
        variety_of_tokens: u128,
        token_infomations: Mapping<TokenId, (TokenName, TokenKind)>,

        balances: Mapping<(AccountId, TokenId), Balance>,
        nfts: Mapping<(AccountId, TokenId), NftIdentityList>,
        
        approvals: Mapping<(AccountId, AccountId), (ApprovalScope, TokenIdList)>,
        nft_approvals: Mapping<(AccountId, AccountId), NftIdentityList>
    }

    impl Erc1155Contract {
        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // debug section
        #[ink(message)]
        pub fn debug_print_balance(&self, owners: AccountIdList) {
            let mut token_ids = ink_prelude::vec![];
            for i in 0..self.variety_of_tokens {
                token_ids.push(i);
            }

            let balance_list = self.balance_of_batch_impl(&owners, &token_ids);
            ink_env::debug_println!(
                "\n\
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n\
[debug print balance]\n
{:?}\n
!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!",
                balance_list
            );
        }

        #[ink(message)]
        pub fn debug_receive_and_return_balance(&self, balance: u128) -> u128 {
            ink_env::debug_println!("\n\n\n\n\n value is {:?} \n\n\n\n\n", balance);
            balance
        }


        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // constructor section
        #[ink(constructor)]
        pub fn new(
            initial_amount: AmountOption,
            currency_name_override: TokenName
        ) -> Self {
            ink_lang::utils::initialize_contract(|contract: &mut Self| {
                let mut name = TokenName::from(DEFAULT_CURRENCY_NAME);
                if !currency_name_override.is_empty() {
                    name = currency_name_override;
                }
                
                contract.initialize_contract_impl(name, &initial_amount);
            })
        }

        fn initialize_contract_impl(
            &mut self, currency_name: TokenName, initial_amount: &AmountOption
        ) {
            self.founder = self.env().caller();
            self.variety_of_tokens = 0;
            
            if self.create_token_type_impl(
                currency_name, &TokenKind::Ft, initial_amount
            ).is_err() {
                // this contract should not run anymore.
                loop {
                    ink_env::debug_println!(
                        "!!!!! Something should not happen is underway !!!!!"
                    );
                }
            }
        }

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // balance and info section
        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId, token_id: TokenId) -> Balance {
            self.balance_of_single_impl(&owner, &token_id)
        }

        #[ink(message)]
        pub fn balance_of_batch(
            &self, owners: AccountIdList, token_ids: TokenIdList
        ) -> BatchBalanceList {
            self.balance_of_batch_impl(&owners, &token_ids)
        }

        fn balance_of_batch_impl(
            &self, owners: &AccountIdList, token_ids: &TokenIdList
        ) -> BatchBalanceList {
            let mut list_for_all = ink_prelude::vec![];
            for aid in owners.iter() {
                let mut list_for_individual = ink_prelude::vec![];
                for tid in token_ids.iter() {
                    let balance = self.balance_of_single_impl(aid, tid);
                    list_for_individual.push(balance);
                }
                list_for_all.push(list_for_individual);
            }
            list_for_all
        }

        #[inline]
        fn balance_of_single_impl(
            &self, owner: &AccountId, token_id: &TokenId
        ) -> Balance {
            self.balances.get((owner, token_id)).unwrap_or(0)
        }

        #[ink(message)]
        pub fn get_nft_owned_list(&self, token_id: TokenId
        ) -> Option<NftIdentityList> {
            let caller = self.env().caller();
            self.get_nft_owned_list_impl(&caller, &token_id)
        }

        #[inline]
        fn get_nft_owned_list_impl(&self, owner: &AccountId, token_id: &TokenId
        ) -> Option<NftIdentityList> {
            self.nfts.get((owner, token_id))
        }

        #[ink(message)]
        pub fn get_token_types_list(&self) -> Result<TokenTypesList> {
            self.get_token_types_list_impl()
        }
        
        #[inline]
        fn get_token_types_list_impl(&self) -> Result<TokenTypesList> {
            let mut list = ink_prelude::vec![];
            for id in 0..self.variety_of_tokens {
                let (name, kind) = self.get_token_type_single_impl(&id)?;
                list.push((id, name, kind));
            }
            Ok(list)
        }

        #[inline]
        fn get_token_type_single_impl(&self, token_id: &TokenId
        ) -> Result<(TokenName, TokenKind)> {
            let (name, kind) = self.token_infomations
                .get(token_id)
                .ok_or(Error::NotFound)?;
            Ok((name, kind))
        }

        #[ink(message)]
        pub fn remained_currency_pool(&self) -> Balance {
            self.balance_of_single_impl(
                &self.env().account_id(), &CURRENCY_TOKEN_ID
            )
        }

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // creation section
        #[ink(message)]
        pub fn create_token_type(
            &mut self,
            name: TokenName,
            token_kind: TokenKind,
            initial_amount: AmountOption
        ) -> Result<u128> {
            let caller = self.env().caller();
            if caller != self.founder {
                return Err(Error::PermissionDenied)
            }

            if name.is_empty() {
                return Err(Error::WrongArgument)
            }

            self.create_token_type_impl(
                name, &token_kind, &initial_amount
            )
        }

        fn create_token_type_impl(
            &mut self,
            name: TokenName,
            token_kind: &TokenKind,
            initial_amount: &AmountOption
        ) -> Result<u128> {
            let next_idx = self.variety_of_tokens;
            let next_num_tokens = self.variety_of_tokens
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            self.variety_of_tokens = next_num_tokens;
            self.token_infomations.insert(
                &next_idx,
                &(name, token_kind.clone())
            );
            
            if *token_kind == TokenKind::Ft {
                let initial_ft_amount = match *initial_amount {
                    AmountOption::None => 0,
                    AmountOption::Some(n) => n,
                    AmountOption::Max => MAX_CURRENCY_AMOUNT
                };

                let contract_address = self.env().account_id();
                self.balances.insert(
                    &(contract_address, next_idx),
                    &initial_ft_amount
                );
                self.env().emit_event(
                    TransferSingle {
                        operator: Some(self.founder),
                        from: None,
                        to: Some(contract_address),
                        token_id: next_idx,
                        value: initial_ft_amount
                    }
                );
            }
            Ok(next_num_tokens)
        }

        #[ink(message)]
        pub fn mint(&mut self, token_id: TokenId, gen: NftGen
        ) -> Result<NftIdentity> {
            if token_id >= self.variety_of_tokens {
                return Err(Error::NotFound)
            }

            if gen.is_empty() {
                return Err(Error::WrongArgument)
            }

            let (_, k) = self.get_token_type_single_impl(&token_id)?;
            if k != TokenKind::Nft {
                return Err(Error::NotFound)
            }

            if self.pay(MINT_FEE).is_err() {
                return Err(Error::NotEnoughFee)
            }

            self.create_nft_impl(&token_id, gen)
        }

        fn create_nft_impl(
            &mut self, token_id: &TokenId, gen: NftGen
        ) -> Result<NftIdentity> {
            let caller = self.env().caller();
            let identity = self.gen_to_identity_impl(&gen);
            let nft_pair = (&caller, token_id);
            
            match self.nfts.get(nft_pair) {
                Some(mut v) => {
                    if let Some(_) = v.iter().find(|&&i| i == identity) {
                        return Err(Error::AlreadyExist)
                    }

                    v.push(identity);
                    self.nfts.insert(nft_pair, &v);
                },
                None => {
                    let mut v = ink_prelude::vec![];
                    v.push(identity);
                    self.nfts.insert(nft_pair, &v);
                }
            }
        
            let nft_owned = self.balance_of_single_impl(&caller, token_id)
                .checked_add(1)
                .ok_or(Error::Overflow)?;
            self.balances.insert((caller, token_id), &nft_owned);

            self.env().emit_event(
                TransferSingle {
                    operator: Some(caller),
                    from: None,
                    to: Some(caller),
                    token_id: *token_id,
                    value: nft_owned,
                }
            );
            Ok(identity)
        }

        #[inline]
        fn gen_to_identity_impl(&self, gen: &NftGen) -> NftIdentity {
            let input = gen.as_bytes();
            let mut output
                = <NftGenHashing as ink_env::hash::HashOutput>::Type::default();
            ink_env::hash_bytes::<NftGenHashing>(input, &mut output);
            NftIdentity::from(output)
        }

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // approval section
        #[ink(message)]
        pub fn is_approved(
            &self,
            owner: AccountId,
            operator: AccountId,
            token_id: Option<TokenId>,
            nft_gen: Option<NftGen>
        ) -> Result<bool> {
            self.is_approved_impl(&owner, &operator, token_id, nft_gen)
        }

        #[ink(message)]
        pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            // recieving error means only approved for some tokens
            self.is_approved_impl(&owner, &operator, None, None)
                .unwrap_or(false)
        }

        fn is_approved_impl(
            &self,
            owner: &AccountId,
            operator: &AccountId,
            op_token_id: Option<TokenId>,
            op_nft_gen: Option<NftGen>
        ) -> Result<bool> {
            let pair = (*owner, *operator);
            match self.approvals.get(&pair) {
                Some((s, v)) => {
                    match s {
                        ApprovalScope::Some => {
                            match op_token_id {
                                Some(token_id) => {
                                    let (_, k) = self.get_token_type_single_impl(&token_id)?;
                                    match k {
                                        TokenKind::Ft => {
                                            if let None = v.iter().find(|id| **id == token_id) {
                                                return Ok(false)
                                            }
                                            Ok(true)
                                        },
                                        TokenKind::Nft => {
                                            match op_nft_gen {
                                                Some(gen) => {
                                                    if let None = v.iter().find(|id| **id == token_id) {
                                                        return Ok(false)
                                                    }

                                                    if !self.is_approved_single_nft_impl(&pair, gen) {
                                                        return Ok(false)
                                                    }

                                                    Ok(true)
                                                },
                                                None => {
                                                    return Err(Error::WrongArgument)
                                                }
                                            }
                                        }
                                    }
                                },
                                None => {
                                    return Err(Error::WrongArgument)
                                }
                            } 
                        },
                        ApprovalScope::All => {
                            return Ok(true)
                        }
                    }
                },
                None => {
                    return Ok(false)
                }
            }
        }

        #[inline]
        fn is_approved_single_nft_impl(
            &self, pair: &(AccountId, AccountId), gen: NftGen
        ) -> bool {
            match self.nft_approvals.get(pair) {
                Some(v) => {
                    let hash = self.gen_to_identity_impl(&gen);
                    if let Some(_) = v.iter().find(|h| **h == hash) {
                        return true
                    }
                    false
                },
                None => {
                    false
                }
            }
        }

        #[ink(message)]
        pub fn set_approval_for_all(
            &mut self, operator: AccountId, approved: bool
        ) -> Result<()> {
            if operator == AccountId::default() {
                return Err(Error::BadAdress)
            }

            let caller = self.env().caller();
            if operator == caller {
                return Err(Error::SelfOperation)
            }

            self.set_approval_for_all_impl(&caller, &operator, approved);
            Ok(())
        }

        fn set_approval_for_all_impl(
            &mut self, owner: &AccountId, operator: &AccountId, approved: bool
        ) {
            let approval_pair = (owner, operator);
            match self.approvals.get(approval_pair) {
                Some((_, v)) => {
                    self.approvals.insert(
                        approval_pair,
                        &(
                            if approved {ApprovalScope::All} else {ApprovalScope::Some},
                            v
                        )
                    );
                },
                None => {
                    if approved {
                        let v: TokenIdList = ink_prelude::vec![];
                        self.approvals.insert(
                            approval_pair,
                            &(ApprovalScope::All, v)
                        );
                    }
                }
            }
            self.env().emit_event(ApprovalForAll {
                owner: *owner,
                operator: *operator,
                approved
            });
        }

        #[ink(message)]
        pub fn set_approval(
            &mut self,
            operator: AccountId,
            approved_tokens: ink_prelude::vec::Vec<(TokenId, Option<NftGenList>)>,
        ) -> Result<()> {
            if operator == AccountId::default() {
                return Err(Error::BadAdress)
            }
            
            let caller = self.env().caller();
            if caller == operator {
                return Err(Error::SelfOperation)
            }

            for (id, opv) in approved_tokens.iter() {
                let (_, kind) = self.get_token_type_single_impl(id)?;
                match kind {
                    TokenKind::Ft => {
                        self.set_approval_impl(&caller, &operator, id);
                    },
                    TokenKind::Nft => {
                        match opv {
                            Some(v) => {
                                let owned = self.nfts
                                    .get((&caller, id))
                                    .ok_or(Error::NotFound)?;
                                for gen in v.iter()
                                {
                                    let hash = self.gen_to_identity_impl(&gen);
                                    if let None = owned.iter().find(|&&i| i == hash) {
                                        return Err(Error::PermissionDenied)
                                    }

                                    self.set_approval_nft_inpl(&caller, &operator, id, &hash);
                                }  
                            },
                            None => {
                                return Err(Error::WrongArgument)
                            }
                        }
                    }
                }
            }
            Ok(())
        }

        fn set_approval_nft_inpl(
            &mut self,
            owner: &AccountId,
            operator: &AccountId,
            token_id: &TokenId,
            nft_identity: &NftIdentity
        ) {
            let nft_pair = (owner, operator);
            match self.nft_approvals.get(nft_pair) {
                Some(mut v) => {
                    if !v.contains(nft_identity) {
                        v.push(*nft_identity);
                        self.nft_approvals.insert(nft_pair, &v);
                    }
                },
                None => {
                    let mut new_list = ink_prelude::vec![];
                    new_list.push(*nft_identity);
                    self.nft_approvals.insert(nft_pair, &new_list);
                }
            }
            self.set_approval_impl(owner, operator, token_id);
        }

        fn set_approval_impl(
            &mut self, owner: &AccountId, operator: &AccountId, token_id: &TokenId
        ) {
            let approval_pair = (owner, operator);
            match self.approvals.get(approval_pair) {
                Some((s, mut v)) => {
                    if !v.contains(token_id) {
                        v.push(*token_id);
                        self.approvals.insert(approval_pair, &(s, v));
                    }
                },
                None => {
                    let mut new_list = ink_prelude::vec![];
                    new_list.push(*token_id);
                    self.approvals.insert(
                        approval_pair, &(ApprovalScope::Some, new_list)
                    ); 
                }
            }
            self.env().emit_event(Approval {
                owner: *owner,
                operator: *operator,
                id: *token_id
            });
        }

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // payment section
        #[ink(message, payable)]
        pub fn buy_currency_500(&mut self) -> Result<Balance> {
            if self.env().transferred_value() < BUY_AMOUNT_500 {
                return Err(Error::NotEnoughFee)
            }
            
            let contract_address = self.env().account_id();
            let caller = self.env().caller();
            self.transfer_token_impl(
                &caller, &CURRENCY_TOKEN_ID, &contract_address, &caller, &BUY_AMOUNT_500
            )
        }

        #[ink(message, payable)]
        pub fn buy_currency(&mut self, how_much: Balance) -> Result<Balance> {
            if self.env().transferred_value() < how_much {
                return Err(Error::NotEnoughFee)
            }
            
            let contract_address = self.env().account_id();
            let caller = self.env().caller();
            self.transfer_token_impl(
                &caller, &CURRENCY_TOKEN_ID, &contract_address, &caller, &how_much
            )
        }
        
        fn pay(&mut self, amount: Balance) -> Result<Balance> {
            let caller = self.env().caller();
            let contract_address = self.env().account_id();

            self.transfer_token_impl(
                &caller, &CURRENCY_TOKEN_ID, &caller, &contract_address, &amount
            )
        }

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // transfer section
        #[ink(message)]
        pub fn transfer_acceptance_check(
            &self,
            operator: AccountId,
            from: AccountId,
            to: AccountId,
            use_batch: bool,
            token_ids: TokenIdList,
            values: BalanceList,
            data: BytesVec
        ) -> Result<()> {
            self.transfer_acceptance_check_impl(
                &operator,
                &from,
                &to,
                &token_ids,
                &values,
                &data,
                use_batch
            )
        }

        fn transfer_acceptance_check_impl(
            &self,
            operator: &AccountId,
            from: &AccountId,
            to: &AccountId,
            token_ids: &TokenIdList,
            values: &BalanceList,
            data: &BytesVec,
            use_batch: bool
        ) -> Result<()> {
            use ink_env::call;

            let ret = if use_batch
            {
                call::build_call::<Environment>()
                    .call_type(call::Call::new().callee(*to))
                    .exec_input(
                        call::ExecutionInput::new(
                            call::Selector::new(
                                ON_ERC1155_BATCH_RECEIVED_SELECTOR
                            )
                        )
                        .push_arg(*operator)
                        .push_arg(from)
                        .push_arg(token_ids)
                        .push_arg(values)
                        .push_arg(data)
                    )
                    .call_flags(ink_env::CallFlags::default().set_allow_reentry(true))
                    .returns::<BytesVec>()
                    .fire()
            } else {
                call::build_call::<Environment>()
                    .call_type(call::Call::new().callee(*to))
                    .exec_input(
                        call::ExecutionInput::new(
                            call::Selector::new(
                                ON_ERC1155_RECEIVED_SELECTOR
                            )
                        )
                        .push_arg(*operator)
                        .push_arg(from)
                        .push_arg(token_ids[0])
                        .push_arg(values[0])
                        .push_arg(data)
                    )
                    .call_flags(ink_env::CallFlags::default().set_allow_reentry(true))
                    .returns::<BytesVec>()
                    .fire()
            };
            match ret {
                Ok(v) => {
                    for (i, n) in v.iter().enumerate() {
                        if *n != ON_ERC1155_RECEIVED_SELECTOR[i] {
                            return Err(Error::TransferDenied)
                        }
                    }
                    Ok(())
                },
                Err(e) => {
                    match e {
                        ink_env::Error::CodeNotFound | ink_env::Error::NotCallable => {
                            return  Ok(())
                        },
                        _ => {
                            Err(Error::TransferDenied)
                        }
                    }
                }
            }
        }
        
        #[ink(message)]
        pub fn safe_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_id: TokenId,
            amount: Balance,
            gen: NftGen,
        ) -> Result<()> {
            ink_env::debug_println!(
                "\n\n\n\n\n received {:?} {:?} {:?} {:?} {:?} \n\n\n\n\n",
                token_id,
                from,
                to,
                amount,
                gen
            );
            let caller = self.env().caller();
            if caller != from {
                let is_approved = self.is_approved_impl(
                    &from,
                    &caller,
                    Some(token_id),
                    if !gen.is_empty() { Some(gen.clone()) } else { None }
                )?;
                if !is_approved {
                    return Err(Error::PermissionDenied)
                }
            }
            
            if to == AccountId::default() {
                return Err(Error::BadAdress)
            }

            if to == from {
                return Err(Error::SelfOperation)
            }

            self.transfer_acceptance_check_impl(
                &caller,
                &from,
                &to,
                &ink_prelude::vec![token_id],
                &ink_prelude::vec![amount],
                &BytesVec::from(gen.clone()),
                false
            )?;

            self.transfer_token_impl(&caller, &token_id, &from, &to, &amount)?;
            self.approvals.remove(&(caller, from));
            let (_, k) = self.get_token_type_single_impl(&token_id)?;
            if k == TokenKind::Nft {
                self.transfer_nft_impl(&token_id, &from, &to, gen)?;
                self.nft_approvals.remove((&caller, from));
            }
            Ok(())
        }

        #[ink(message)]
        pub fn safe_batch_transfer_from(
            &mut self,
            from: AccountId,
            to: AccountId,
            token_ids: TokenIdList,
            values: BalanceList,
            gens: NftGenList
        ) -> Result<()> {
            let caller = self.env().caller();
            if caller != from {
                let is_approved = self.is_approved_impl(&from, &caller, None, None)
                    .map_err(|_| Error::PermissionDenied)?;
                if !is_approved{
                    return Err(Error::PermissionDenied)
                }
            }

            if to == AccountId::default() {
                return Err(Error::BadAdress)
            }

            if to == from {
                return Err(Error::SelfOperation)
            }

            if token_ids.is_empty() {
                return Err(Error::WrongArgument)
            }

            let iterations = token_ids.len();

            if iterations != values.len() {
                return Err(Error::WrongArgument)
            }

            let mut joined = NftGen::new();
            for s in &gens {
                joined.insert_str(joined.len() - 1, &s.clone());
                joined.insert_str(joined.len() - 1, "|");
            }
            let data = BytesVec::from(joined);

            self.transfer_acceptance_check_impl(
                &caller, &from, &to, &token_ids, &values, &data, true
            )?;

            for i in 0..iterations {
                self.transfer_token_impl(
                    &caller, &token_ids[i], &from, &to, &values[i]
                )?;
                self.approvals.remove(&(caller, from));
                let (_, k) = self.get_token_type_single_impl(&token_ids[i])?;
                if k == TokenKind::Nft {
                    self.transfer_nft_impl(
                        &token_ids[i], &from, &to, gens[i].clone()
                    )?;
                    self.nft_approvals.remove((&caller, from));
                }
            }
            Ok(())
        }

        fn transfer_token_impl(
            &mut self,
            operator: &AccountId,
            token_id: &TokenId,
            from: &AccountId,
            to: &AccountId,
            amount: &Balance
        ) -> Result<Balance> {
            let current_src_balance = self.balance_of_single_impl(from, token_id);
            if current_src_balance < *amount {
                return Err(Error::NotAvailable)
            }

            let new_src_balance = current_src_balance
                .checked_sub(*amount)
                .ok_or(Error::NotAvailable)?;
            let current_dest_balance = self.balance_of_single_impl(to, token_id);
            let new_dest_balance = current_dest_balance
                .checked_add(*amount)
                .ok_or(Error::Overflow)?;
            self.balances.insert((from, token_id), &new_src_balance);
            self.balances.insert((to, token_id), &new_dest_balance);
            self.env().emit_event(
                TransferSingle {
                    operator: Some(*operator),
                    from: Some(*from),
                    to: Some(*to),
                    token_id: *token_id,
                    value: *amount
                }
            );
            Ok(*amount)
        }

        fn transfer_nft_impl(
            &mut self,
            token_id: &TokenId,
            from: &AccountId,
            to: &AccountId,
            gen: NftGen
        ) -> Result<()> {
            ink_env::debug_println!(
                "\n\n\n\n\n nft impl received {:?} {:?} {:?} {:?} \n\n\n\n\n",
                *token_id,
                *from,
                *to,
                gen
            );
            let mut nft_list_src = self.get_nft_owned_list_impl(from, token_id)
                .ok_or(Error::NotFound)?;
                let identity = self.gen_to_identity_impl(&gen);
                match nft_list_src.iter().position(|&i| i == identity) {
                Some(idx) => {
                    let target = nft_list_src.swap_remove(idx);
                    let nft_list_dest = self.get_nft_owned_list_impl(to, token_id);
                    match nft_list_dest {
                        Some(mut v) => {
                            v.push(target);
                            self.nfts.insert((to, token_id), &v);
                        },
                        None => {
                            let v = ink_prelude::vec![target];
                            self.nfts.insert((to, token_id), &v);
                        }
                    }
                },
                None => {
                    return Err(Error::NotFound)
                }
            }
            Ok(())
        }

        // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
        // receiver section
        #[ink(message, selector = 0xF23A6E61)]
        pub fn on_received(
            &mut self,
            _operator: AccountId,
            _from: AccountId,
            _token_id: TokenId,
            _value: Balance,
            _data: BytesVec
        ) -> BytesVec {
            // accept all transfer
            ink_prelude::vec![0xf2, 0x3a, 0x6e, 0x61]
        }

        #[ink(message, selector = 0xBC197C81)]
        pub fn on_batch_received(
            &mut self,
            _operator: AccountId,
            _from: AccountId,
            _token_id: TokenIdList,
            _value: BalanceList,
            _data: BytesVec
        ) -> BytesVec {
            // accept all transfer
            ink_prelude::vec![0xbc, 0x19, 0x7c, 0x81]
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
