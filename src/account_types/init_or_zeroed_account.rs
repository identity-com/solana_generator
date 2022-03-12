//! Accepts both forms of initialize-able account

use std::iter::once;
use std::ops::{Deref, DerefMut};

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

use crate::account_argument::{
    AccountArgument, AccountInfoIterator, FromAccounts, MultiIndexable, SingleIndexable,
    ValidateArgument,
};
use crate::account_list::AccountListItem;
use crate::account_types::discriminant_account::DiscriminantAccount;
use crate::account_types::init_account::{InitAccount, InitArgs};
use crate::account_types::zeroed_account::{CheckAll, ZeroedAccount};
use crate::AllAny;
use crate::{AccountInfo, CruiserResult};
use cruiser_derive::verify_account_arg_impl;

verify_account_arg_impl! {
    mod init_account_check{
        <AL, A> InitOrZeroedAccount<AL, A>
        where
            AL: AccountListItem<A>,
            A: BorshSerialize + BorshDeserialize{
            from: [
                /// The initial value of this account
                A;
            ];
            validate: [
                <'a> InitArgs<'a>;
                <'a> (InitArgs<'a>, CheckAll);
            ];
            multi: [(); AllAny];
            single: [()];
        }
    }
}

/// A combination of [`InitAccount`] and [`ZeroedAccount`] accepting either based on owner.
// TODO: impl Debug for this
#[allow(missing_debug_implementations)]
// TODO: use AccountArgument trait for impl when enums supported
pub enum InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    /// Is an [`InitAccount`]
    Init(InitAccount<AL, A>),
    /// Is a [`ZeroedAccount`]
    Zeroed(ZeroedAccount<AL, A>),
}
impl<AL, A> Deref for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    type Target = DiscriminantAccount<AL, A>;

    fn deref(&self) -> &Self::Target {
        match self {
            InitOrZeroedAccount::Init(init) => init,
            InitOrZeroedAccount::Zeroed(zeroed) => zeroed,
        }
    }
}
impl<AL, A> DerefMut for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            InitOrZeroedAccount::Init(init) => init,
            InitOrZeroedAccount::Zeroed(zeroed) => zeroed,
        }
    }
}
impl<AL, A> AccountArgument for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    fn write_back(self, program_id: &'static Pubkey) -> CruiserResult<()> {
        match self {
            InitOrZeroedAccount::Init(init) => init.write_back(program_id),
            InitOrZeroedAccount::Zeroed(zeroed) => zeroed.write_back(program_id),
        }
    }

    fn add_keys(&self, add: impl FnMut(&'static Pubkey) -> CruiserResult<()>) -> CruiserResult<()> {
        match self {
            InitOrZeroedAccount::Init(init) => init.add_keys(add),
            InitOrZeroedAccount::Zeroed(zeroed) => zeroed.add_keys(add),
        }
    }
}
impl<'a, AL, A> FromAccounts<A> for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    fn from_accounts(
        program_id: &'static Pubkey,
        infos: &mut impl AccountInfoIterator,
        arg: A,
    ) -> CruiserResult<Self> {
        let info = AccountInfo::from_accounts(program_id, infos, ())?;
        if *info.owner.borrow() == program_id {
            Ok(Self::Zeroed(ZeroedAccount::from_accounts(
                program_id,
                &mut once(info),
                arg,
            )?))
        } else {
            Ok(Self::Init(InitAccount::from_accounts(
                program_id,
                &mut once(info),
                arg,
            )?))
        }
    }

    fn accounts_usage_hint(_arg: &A) -> (usize, Option<usize>) {
        AccountInfo::accounts_usage_hint(&())
    }
}
impl<'a, AL, A> ValidateArgument<InitArgs<'a>> for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    fn validate(&mut self, program_id: &'static Pubkey, arg: InitArgs<'a>) -> CruiserResult<()> {
        match self {
            InitOrZeroedAccount::Init(init) => init.validate(program_id, arg),
            InitOrZeroedAccount::Zeroed(zeroed) => zeroed.validate(program_id, ()),
        }
    }
}
impl<'a, AL, A> ValidateArgument<(InitArgs<'a>, CheckAll)> for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
{
    fn validate(
        &mut self,
        program_id: &'static Pubkey,
        arg: (InitArgs<'a>, CheckAll),
    ) -> CruiserResult<()> {
        match self {
            InitOrZeroedAccount::Init(init) => init.validate(program_id, arg.0),
            InitOrZeroedAccount::Zeroed(zeroed) => zeroed.validate(program_id, arg.1),
        }
    }
}
impl<AL, A, T> MultiIndexable<T> for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
    AccountInfo: MultiIndexable<T>,
{
    fn is_signer(&self, indexer: T) -> CruiserResult<bool> {
        self.info.is_signer(indexer)
    }

    fn is_writable(&self, indexer: T) -> CruiserResult<bool> {
        self.info.is_writable(indexer)
    }

    fn is_owner(&self, owner: &Pubkey, indexer: T) -> CruiserResult<bool> {
        self.info.is_owner(owner, indexer)
    }
}
impl<AL, A, T> SingleIndexable<T> for InitOrZeroedAccount<AL, A>
where
    AL: AccountListItem<A>,
    A: BorshSerialize + BorshDeserialize,
    AccountInfo: SingleIndexable<T>,
{
    fn info(&self, indexer: T) -> CruiserResult<&AccountInfo> {
        self.info.info(indexer)
    }
}
