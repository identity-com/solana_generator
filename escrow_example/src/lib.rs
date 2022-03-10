use cruiser::{
    AccountArgument, AccountInfo, AccountList, CloseAccount, entrypoint_list, Find, GeneratorError,
    GeneratorResult, InitArgs, InitOrZeroedAccount, Instruction, InstructionList, InstructionProcessor,
    msg, OnChainSize, PDAGenerator, PDASeed, PDASeeder, ProgramAccount, Pubkey,
    RentExempt, Seeds, Single, SystemProgram,
};
use cruiser::borsh::{BorshDeserialize, BorshSerialize};
use cruiser::spl::token::{Owner, TokenAccount, TokenProgram};

entrypoint_list!(EscrowInstructions, EscrowInstructions);

#[derive(InstructionList, Copy, Clone)]
#[instruction_list(account_list = EscrowAccounts)]
pub enum EscrowInstructions {
    #[instruction(instruction_type = InitEscrow)]
    InitEscrow,
    #[instruction(instruction_type = InitEscrow)]
    Exchange,
}

#[derive(AccountList)]
pub enum EscrowAccounts {
    EscrowAccount(EscrowAccount),
}

#[derive(BorshSerialize, BorshDeserialize, Default)]
pub struct EscrowAccount {
    pub initializer: Pubkey,
    pub temp_token_account: Pubkey,
    pub initializer_token_to_receive: Pubkey,
    pub expected_amount: u64,
}
impl OnChainSize for EscrowAccount {
    fn on_chain_size() -> usize {
        Pubkey::on_chain_size() * 3 + u64::on_chain_size()
    }
}

#[derive(Debug)]
struct EscrowPDASeeder;
impl PDASeeder for EscrowPDASeeder {
    fn seeds<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn PDASeed> + 'a> {
        Box::new([&"escrow" as &dyn PDASeed].into_iter())
    }
}

pub struct InitEscrow;
impl Instruction for InitEscrow {
    type Data = InitEscrowData;
    type FromAccountsData = ();
    type Accounts = InitEscrowAccounts;

    fn data_to_instruction_arg(_data: &mut Self::Data) -> GeneratorResult<Self::FromAccountsData> {
        Ok(())
    }
}
impl InstructionProcessor<InitEscrow> for InitEscrow {
    fn process(
        program_id: &'static Pubkey,
        data: <Self as Instruction>::Data,
        accounts: &mut <Self as Instruction>::Accounts,
    ) -> GeneratorResult<()> {
        let escrow_account = &mut accounts.escrow_account;
        escrow_account.initializer = *accounts.initializer.key;
        escrow_account.temp_token_account = *accounts.temp_token_account.get_info().key;
        escrow_account.initializer_token_to_receive =
            *accounts.initializer_token_account.get_info().key;
        escrow_account.expected_amount = data.amount;

        let (pda, _) = EscrowPDASeeder.find_address(program_id);

        msg!("Calling the token program to transfer token account ownership...");
        accounts.token_program.invoke_set_authority(
            &accounts.temp_token_account,
            &pda,
            &accounts.initializer,
        )?;

        Ok(())
    }
}
#[derive(BorshSerialize, BorshDeserialize)]
pub struct InitEscrowData {
    amount: u64,
}
#[derive(AccountArgument)]
pub struct InitEscrowAccounts {
    #[validate(signer)]
    initializer: AccountInfo,
    #[validate(writable, data = Owner(self.initializer.key))]
    temp_token_account: TokenAccount,
    initializer_token_account: TokenAccount,
    #[from(data = EscrowAccount::default())]
    #[validate(writable, data = (InitArgs{
        funder: &self.initializer,
        funder_seeds: None,
        rent: None,
        space: EscrowAccount::on_chain_size(),
        system_program: &self.system_program,
    },))]
    escrow_account: RentExempt<InitOrZeroedAccount<EscrowAccounts, EscrowAccount>>,
    token_program: TokenProgram,
    system_program: SystemProgram,
}

pub struct Exchange;
impl Instruction for Exchange {
    type Data = ExchangeData;
    type FromAccountsData = ();
    type Accounts = ExchangeAccounts;

    fn data_to_instruction_arg(_data: &mut Self::Data) -> GeneratorResult<Self::FromAccountsData> {
        Ok(())
    }
}
impl InstructionProcessor<Exchange> for Exchange {
    fn process(
        _program_id: &'static Pubkey,
        data: <Self as Instruction>::Data,
        accounts: &mut <Self as Instruction>::Accounts,
    ) -> GeneratorResult<()> {
        if data.amount != accounts.escrow_account.expected_amount {
            return Err(GeneratorError::Custom {
                error: format!(
                    "Amount (`{}`) did not equal expected (`{}`)",
                    data.amount, accounts.escrow_account.expected_amount
                ),
            }
            .into());
        }

        msg!("Calling the token program to transfer tokens to the escrow's initializer...");
        accounts.token_program.invoke_transfer(
            &accounts.taker_send_token_account,
            &accounts.initializer_token_account,
            &accounts.taker,
            accounts.escrow_account.expected_amount,
        )?;

        let seeds = accounts.pda_account.take_seed_set().unwrap();
        msg!("Calling the token program to transfer tokens to the taker...");
        accounts.token_program.invoke_signed_transfer(
            &seeds,
            &accounts.temp_token_account,
            &accounts.taker_receive_token_account,
            accounts.pda_account.get_info(),
            accounts.temp_token_account.amount,
        )?;

        msg!("Calling the token program to close pda's temp account...");
        accounts.token_program.invoke_signed_close_account(
            &seeds,
            &accounts.temp_token_account,
            &accounts.initializer,
            accounts.pda_account.get_info(),
        )?;

        msg!("Closing the escrow account...");
        accounts
            .escrow_account
            .set_fundee(accounts.initializer.clone());
        Ok(())
    }
}
#[derive(BorshSerialize, BorshDeserialize)]
pub struct ExchangeData {
    amount: u64,
}
#[derive(AccountArgument)]
pub struct ExchangeAccounts {
    #[validate(signer)]
    taker: AccountInfo,
    #[validate(writable, data = Owner(self.taker.key))]
    taker_send_token_account: TokenAccount,
    #[validate(writable)]
    taker_receive_token_account: TokenAccount,
    #[validate(writable, key = &self.escrow_account.temp_token_account)]
    temp_token_account: TokenAccount,
    #[validate(writable, key = &self.escrow_account.initializer)]
    initializer: AccountInfo,
    #[validate(writable, key = &self.escrow_account.initializer_token_to_receive)]
    initializer_token_account: TokenAccount,
    #[validate(writable)]
    escrow_account: CloseAccount<ProgramAccount<EscrowAccounts, EscrowAccount>>,
    token_program: TokenProgram,
    #[validate(data = (EscrowPDASeeder, Find))]
    pda_account: Seeds<AccountInfo, EscrowPDASeeder>,
}
