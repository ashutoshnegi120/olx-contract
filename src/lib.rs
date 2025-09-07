use std::time::Instant;

//load all the modules that we need to use in our program
use bytemuck::{Pod, Zeroable};
use solana_program::entrypoint;
use solana_program::program::invoke;
use solana_program::{
    account_info::{AccountInfo, next_account_info},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{Sysvar, rent::Rent},
};
use solana_system_interface::instruction;

/*
  struct -
    we have to define the structure of our data before we write the code about that ok ,
    as we making this project in zero copy way so the struct should be use Pod and Zeroable
    and also dont forget to add repr(C) attribute.
    inside the struct we don't use any dynamic data type like Vec or String

*/

/*
    as we start our programing first we thought what we are making we are making a olx contract me on-chain side logic about out olx dapp
    as we know we cant pass mutliple instruction in  single enterypoint so we have to make a enum but wait enum and bytemuck dont go well together
    so we have to go with the struct where we define a u8 variable as a instruction identifier and then we can define the other variable that we want to pass
    inside the struct and then make a impl block for that struct and inside that impl block we can make a function that will convert the u8 variable to enum
*/

#[repr(transparent)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct State(u8);

impl State {
    const INIT: u8 = 0;
    const UPDATE: u8 = 1;
    const DELETE: u8 = 2;
    const BUY: u8 = 3;
    const SELL: u8 = 4;
    const CANCEL: u8 = 5;
    const HOLD_ACCOUNT: u8 = 6;

    fn from_u8(value: u8) -> Result<Self, ProgramError> {
        match value {
            Self::INIT => Ok(State(Self::INIT)),
            Self::UPDATE => Ok(State(Self::UPDATE)),
            Self::DELETE => Ok(State(Self::DELETE)),
            Self::BUY => Ok(State(Self::BUY)),
            Self::SELL => Ok(State(Self::SELL)),
            Self::CANCEL => Ok(State(Self::CANCEL)),
            Self::HOLD_ACCOUNT => Ok(State(Self::HOLD_ACCOUNT)),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}

// let go one by one first we implement the INIT data structer then we go to the other one

// the data we get from the client side
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InitData {
    uuid: [u8; 16],
    item_id: [u8; 32],
    title: [u8; 128],
    description: [u8; 1024],
    price: u64,
    seed: [u8; 32], // 31 char we can store in the seed one char is reserve for \0
}

// the main data structer for an account data that we store in side the account
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct InitAccountData {
    item_id: [u8; 32],
    title: [u8; 128],
    description: [u8; 1024],
    price: u64,
    payer: [u8; 32],
}

//let make a update logic so this struct all going to be same as the InitAccountData
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct UpdateData {
    title: [u8; 128],
    description: [u8; 1024],
    price: u64,
    seed: [u8; 32], // 31 char we can store in the seed one char is reserve for \0
}

// as we need one this in delete logic that is seed because we need to find the pda  and check the person so make a request is realy the post owner  or not beacuse we use the payer key as a part of seed
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct DeleteData {
    seed: [u8; 32], // 31 char we can store in the seed
}

//let make a struct for buy
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BuyInit {
    item_id: [u8; 32],
    buyer: [u8; 32],
    seed: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BuyData {
    item_id: [u8; 32],
    buyer: [u8; 32],
    seller: [u8; 32],
    price: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CancelBuy {
    item_id: [u8; 32],
    seed: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SellData {
    seed_post: [u8; 32],
    seed_buy: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ImmutableRegistryData {
    item_id: [u8; 32],
    buyer: [u8; 32],
    seller: [u8; 32],
    price: u64,
    title: [u8; 128],
    description: [u8; 1024],
    timestamp: u64,
}

//we make two thing normal sol holder and and account where we store the all the immutable proof that buy happend
//===============================================================================================
#[repr(transparent)]
#[derive(Clone,Copy,Pod,Zeroable)]
struct HoldState(u8);

impl HoldState {
    const MONEY_HOLDER : u8 = 0;
    const TEMP_MONEY_HOLDER : u8 = 1;
    const BUY_INFO_HOLDER: u8 = 2;

    fn from_u8(value : u8)-> Result<Self , ProgramError>{
        match value{
            Self::MONEY_HOLDER => Ok(HoldState(Self::MONEY_HOLDER)),
            Self::TEMP_MONEY_HOLDER => Ok(HoldState(Self::TEMP_MONEY_HOLDER)),
            Self::BUY_INFO_HOLDER => Ok(HoldState(Self::BUY_INFO_HOLDER)),
            _ => Err(ProgramError::InvalidInstructionData)
        }

    }
    
}


#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MoneyHolder{
    title : [u8;32]
}


#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TempMoneyHolder{
    title : [u8;32],
    buyer : [u8;32],
    seller : [u8;32]
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Info{
    title : [u8;32],
}




//===============================================================================================

//call the macro entrypoint to define the entry point of the program
entrypoint!(process_instruction);

// we have to make a entery point fn that will call by macro entrypoint and also handle which instruction to call

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8], // [ {state}]
) -> ProgramResult {
    let instruction = State::from_u8(instruction_data[0])?;
    match instruction.0 {
        State::INIT => {
            msg!("Instruction: INIT");
            if instruction_data[1..].len() != std::mem::size_of::<InitData>() {
                return Err(ProgramError::InvalidInstructionData);
            }
            let data = bytemuck::from_bytes::<InitData>(&instruction_data[1..]);
            process_init(program_id, accounts, data) // pass the rest of the data
        }
        State::UPDATE => {
            msg!("Instruction: UPDATE");
            if instruction_data[1..].len() != std::mem::size_of::<UpdateData>() {
                return Err(ProgramError::InvalidInstructionData);
            }
            let data = bytemuck::from_bytes::<UpdateData>(&instruction_data[1..]);
            update_account_data(program_id, accounts, data) // pass the rest of the data
        }

        State::DELETE => {
            msg!("Instruction: DELETE");
            if instruction_data[1..].len() != std::mem::size_of::<DeleteData>() {
                return Err(ProgramError::InvalidInstructionData);
            }
            let data = bytemuck::from_bytes::<DeleteData>(&instruction_data[1..]);
            delete_account_data(program_id, accounts, &data) // pass the rest of the data
        }

        State::BUY => {
            msg!("Instruction: BUY");
            if instruction_data[1..].len() != std::mem::size_of::<BuyInit>() {
                return Err(ProgramError::InvalidInstructionData);
            }
            let data = bytemuck::from_bytes::<BuyInit>(&instruction_data[1..]);
            buy_item(program_id, accounts, data)
        }
        State::SELL => {
            msg!("Instruction: SELL");
            if instruction_data[1..].len() != std::mem::size_of::<SellData>(){
                return Err(ProgramError::InvalidAccountData);
            }
            let data = bytemuck::from_bytes::<SellData>(&instruction_data[1..]);
            sell_item(program_id, accounts, data)
            
        }
        State::CANCEL => {
            msg!("Instruction: CANCEL");
            if instruction_data[1..].len() != std::mem::size_of::<CancelBuy>() {
                return Err(ProgramError::InvalidInstructionData);
            }
            let data = bytemuck::from_bytes::<CancelBuy>(&instruction_data[1..]);
            cancel_buy(program_id, accounts, data)
        }
        State::HOLD_ACCOUNT => {
            msg!("Instruction: HOLD_ACCOUNT");
            if instruction_data[1..].len() >= std::mem::size_of::<HoldState>(){
                match instruction_data[2]{
                    HoldState::MONEY_HOLDER => {
                        if instruction_data[2..].len() < std::mem::size_of::<MoneyHolder>(){
                            return Err(ProgramError::InvalidArgument)
                        }

                        let data = bytemuck::from_bytes::<MoneyHolder>( &instruction_data[2..]);
                        return money_holder(program_id, accounts, data)
                        
                    }
                    HoldState::TEMP_MONEY_HOLDER => {
                        if instruction_data[2..].len() < std::mem::size_of::<TempMoneyHolder>(){
                            return Err(ProgramError::InvalidArgument)
                        }

                        let data = bytemuck::from_bytes::<TempMoneyHolder>( &instruction_data[2..]);
                        return temp_money_holder(program_id, accounts, data)
                        
                    }
                    HoldState::BUY_INFO_HOLDER => {
                       if instruction_data[2..].len() < std::mem::size_of::<Info>(){
                            return Err(ProgramError::InvalidArgument)
                        }

                        let data = bytemuck::from_bytes::<Info>( &instruction_data[2..]);
                        return info_account(program_id, accounts, data)
                    }
                    _ => {
                        msg!("Error: Invalid HoldState argument");
                    }
                }
            }
            Err(ProgramError::InvalidInstructionData)
            
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn process_init(
    program_id: &Pubkey,
    accounts: &[AccountInfo], //[payer , unsign_account , system_program, lookup_table_account]
    ix_data: &InitData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?; // the account that will pay for the rent
    let unsigned_account = next_account_info(account_info_iter)?; // the account that will be created and store the data
    let system_program = next_account_info(account_info_iter)?; // the system program account

    if !payer.is_signer {
        msg!("Error: Payer account should be a signer");
        return Err(ProgramError::InvalidArgument);
    }

    // You can add this back to process_init if you want extra validation
    if !unsigned_account.is_signer {
        msg!("Error: Unsigned account is not a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // let check the pda seed is correct or not
    let (pda, bump_seed) =
        Pubkey::find_program_address(&[b"INIT", &ix_data.seed, payer.key.as_ref()], program_id);
    if pda != *unsigned_account.key {
        msg!("Error: Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(std::mem::size_of::<InitAccountData>());

    let ix = instruction::create_account(
        payer.key,
        &pda,
        required_lamports,
        std::mem::size_of::<InitAccountData>() as u64,
        program_id,
    );

    invoke_signed(
        &ix,
        &[
            payer.clone(),
            unsigned_account.clone(),
            system_program.clone(),
        ],
        &[&[b"INIT", &ix_data.seed, payer.key.as_ref(), &[bump_seed]]],
    )?;

    let mut binding = &mut unsigned_account.data.borrow_mut();
    let unsigned_account_data = bytemuck::from_bytes_mut::<InitAccountData>(&mut binding);

    unsigned_account_data.item_id = ix_data.item_id;
    unsigned_account_data.title = ix_data.title;
    unsigned_account_data.description = ix_data.description;
    unsigned_account_data.price = ix_data.price;
    unsigned_account_data.payer = payer.key.to_bytes();

    msg!("Account initialized successfully");

    Ok(())
}

fn update_account_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo], //[payer , unsign_account , system_program]
    ix_data: &UpdateData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?; // the account that will pay for the rent
    let pda_account = next_account_info(account_info_iter)?; // the account that will be created and store the data

    if !payer.is_signer {
        msg!("Error: Payer account should be a signer");
        return Err(ProgramError::InvalidArgument);
    }

    if pda_account.owner != program_id {
        msg!("Error: Account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // let check the pda seed is correct or not
    let (pda, _bump_seed) =
        Pubkey::find_program_address(&[b"INIT", &ix_data.seed, payer.key.as_ref()], program_id);
    if pda != *pda_account.key {
        msg!("Error: Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let mut binding = &mut pda_account.data.borrow_mut();
    let signed_account_data = bytemuck::from_bytes_mut::<InitAccountData>(&mut binding);

    signed_account_data.title = ix_data.title;
    signed_account_data.description = ix_data.description;
    signed_account_data.price = ix_data.price;

    msg!("Account updated successfully");
    Ok(())
}

fn delete_account_data(
    program_id: &Pubkey,
    accounts: &[AccountInfo], //[payer , unsign_account , system_program]
    ix_data: &DeleteData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?; // the account that will pay for
    let signed_account = next_account_info(account_info_iter)?; // the account that will be created and store the data

    if !payer.is_signer {
        msg!("Error: Payer account should be a signer");
        return Err(ProgramError::InvalidArgument);
    }

    if signed_account.owner != program_id {
        msg!("Error: Account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // check is the payer is the real payer of this account or not

    let (pda, _) =
        Pubkey::find_program_address(&[b"INIT", &ix_data.seed, payer.key.as_ref()], program_id);

    if signed_account.key != &pda {
        msg!("Error: Invalid seeds or payer is not the real payer of this account");
        return Err(ProgramError::InvalidArgument);
    }

    **payer.try_borrow_mut_lamports()? = payer
        .lamports()
        .checked_add(signed_account.lamports())
        .ok_or(ProgramError::InvalidArgument)?;
    **signed_account.try_borrow_mut_lamports()? = 0;

    let mut data = signed_account.try_borrow_mut_data()?;
    data.fill(0);

    Ok(())
}

//let implemet the buy , sell and cancel logic now

fn buy_item(
    program_id: &Pubkey,
    accounts: &[AccountInfo], //[payer , post_account , unsign_account , holder account, system_program]
    ix_data: &BuyInit,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let buyer = next_account_info(account_info_iter)?; // the account that will pay for
    let post_account = next_account_info(account_info_iter)?; // the account that hold the post data
    let unsigned_account = next_account_info(account_info_iter)?;
    let holder_account = next_account_info(account_info_iter)?; // the account that will hold the money
    let system_program = next_account_info(account_info_iter)?; // the system program account

    if !buyer.is_signer {
        msg!("Error: Buyer account should be a signer");
        return Err(ProgramError::InvalidArgument);
    }

    if unsigned_account.owner != program_id {
        msg!("Error: Account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (pda, bump) =
        Pubkey::find_program_address(&[b"BUY", &ix_data.seed, buyer.key.as_ref()], program_id);
    if pda != *unsigned_account.key {
        msg!("Error: Invalid seeds for PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(std::mem::size_of::<BuyData>());

    let (hold_pda, _hold_bump) =
        Pubkey::find_program_address(&[b"HOLDER", ix_data.item_id.as_ref()], program_id);

    if hold_pda != *holder_account.key {
        msg!("Error: Invalid holder account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    if holder_account.owner != program_id {
        msg!("Error: Holder account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let ix = instruction::create_account(
        buyer.key,
        &pda,
        required_lamports,
        std::mem::size_of::<BuyData>() as u64,
        program_id,
    );

    invoke_signed(
        &ix,
        &[
            buyer.clone(),
            unsigned_account.clone(),
            system_program.clone(),
        ],
        &[&[b"BUY", &ix_data.seed, buyer.key.as_ref(), &[bump]]],
    )?;
    let mut seller_data = &mut post_account.data.borrow_mut();
    let post_data = bytemuck::from_bytes_mut::<InitAccountData>(&mut seller_data);

    let mut binding = &mut unsigned_account.data.borrow_mut();
    let buy_account_data = bytemuck::from_bytes_mut::<BuyData>(&mut binding);

    buy_account_data.item_id = ix_data.item_id;
    buy_account_data.buyer = ix_data.buyer;
    buy_account_data.price = post_data.price;
    buy_account_data.seller = post_data.payer;

    let transfer_ix = instruction::transfer(buyer.key, holder_account.key, post_data.price);

    invoke(
        &transfer_ix,
        &[
            buyer.clone(),
            holder_account.clone(),
            system_program.clone(),
        ],
    )?;

    msg!("Buy account initialized successfully");
    Ok(())
}

fn cancel_buy(
    program_id: &Pubkey,
    accounts: &[AccountInfo], //[buyer ,Buy account ,holder_account ]
    ix_data: &CancelBuy,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let buyer = next_account_info(account_info_iter)?;
    let buy_account = next_account_info(account_info_iter)?;
    let holder_account = next_account_info(account_info_iter)?;

    if !buyer.is_signer {
        msg!("Error: Buyer account should be a signer");
        return Err(ProgramError::InvalidArgument);
    }

    let binding = buy_account.data.borrow();
    let buy_data = bytemuck::try_from_bytes::<BuyData>(&binding)
        .or_else(|_| Err(ProgramError::InvalidAccountData))?;

    if buy_data.buyer != buyer.key.to_bytes() {
        msg!("Error: Buyer account mismatch");
        return Err(ProgramError::InvalidArgument);
    }

    let (hold_pda, _hold_bump) =
        Pubkey::find_program_address(&[b"HOLDER", ix_data.item_id.as_ref()], program_id);

    if hold_pda != *holder_account.key {
        msg!("Error: Invalid holder account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    if holder_account.owner != program_id {
        msg!("Error: Holder account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    **buyer.try_borrow_mut_lamports()? = buyer
        .lamports()
        .checked_add(holder_account.lamports())
        .ok_or(ProgramError::InvalidArgument)?;
    **holder_account.try_borrow_mut_lamports()? = 0;

    **buyer.try_borrow_mut_lamports()? = buyer
        .lamports()
        .checked_add(buy_account.lamports())
        .ok_or(ProgramError::InvalidArgument)?;
    **buy_account.try_borrow_mut_lamports()? = 0;

    let mut data = buy_account.try_borrow_mut_data()?;
    data.fill(0);
    msg!("Buy cancelled successfully");

    Ok(())
}

fn sell_item(
    program_id: &Pubkey,
    accounts: &[AccountInfo], //[seller_account,buyer_account, buy_account, post_account , holder_account , system_program, imutiable_registry_accont, temp_account] we can you lookup table here to reduce the transaction fee.
    ix_data: &SellData,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let seller = next_account_info(account_info_iter)?;
    let buyer = next_account_info(account_info_iter)?;
    let buy_account = next_account_info(account_info_iter)?;
    let post_account = next_account_info(account_info_iter)?;
    let holder_account = next_account_info(account_info_iter)?;
    let system_program = next_account_info(account_info_iter)?;
    let imutiable_registry_account = next_account_info(account_info_iter)?;
    let temp_account = next_account_info(account_info_iter)?;

    if !seller.is_signer && !buyer.is_signer {
        msg!("Error: Seller account should be a signer");
        return Err(ProgramError::InvalidArgument);
    }

    let (pda_post, _bump_post) = Pubkey::find_program_address(
        &[b"INIT", &ix_data.seed_post, seller.key.as_ref()],
        program_id,
    );
    let (pda_buy, _bump_buy) =
        Pubkey::find_program_address(&[b"BUY", &ix_data.seed_buy, buyer.key.as_ref()], program_id);

    if pda_post != *post_account.key {
        msg!("Error: Invalid post account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    if post_account.owner != program_id {
        msg!("Error: Post account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    if pda_buy != *buy_account.key {
        msg!("Error: Invalid buy account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    if post_account.owner != program_id {
        msg!("Error: Buy account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let binding = buy_account.data.borrow();
    let data = bytemuck::from_bytes::<BuyInit>(&binding);
    let (hold_pda, _hold_bump) =
        Pubkey::find_program_address(&[b"HOLDER", data.item_id.as_ref()], program_id);

    if hold_pda != *holder_account.key {
        msg!("Error: Invalid holder account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    if holder_account.owner != program_id {
        msg!("Error: Holder account not owned by this program");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (temp_pda , _temp_bump) = Pubkey::find_program_address(&[b"TEMP", buyer.key.as_ref(), seller.key.as_ref(), data.item_id.as_ref()], program_id);

    if temp_pda != *temp_account.key{
         msg!("Error: Invalid temp account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let (pda_imu, _bump_imu) = Pubkey::find_program_address(
        &[
            b"IMUTABLE",
            data.item_id.as_ref(),
            buyer.key.as_ref(),
            seller.key.as_ref(),
        ],
        program_id,
    );
    if pda_imu != *imutiable_registry_account.key {
        msg!("Error: Invalid imutiable registry account PDA");
        return Err(ProgramError::InvalidArgument);
    }

    let rent = Rent::get()?;
    let required_lamports = rent.minimum_balance(std::mem::size_of::<ImmutableRegistryData>());
    let space = std::mem::size_of::<ImmutableRegistryData>() as u64;
    let split_lamport = required_lamports / 2 + 1;

    let ix_acccount_fee_seller = instruction::transfer(seller.key, temp_account.key, split_lamport);
    let ix_account_fee_buyer = instruction::transfer(buyer.key, temp_account.key, split_lamport);

    invoke(
        &ix_acccount_fee_seller,
        &[seller.clone(), buy_account.clone(),temp_account.clone(),system_program.clone()],
    )?;
    invoke(&ix_account_fee_buyer, &[buyer.clone(), temp_account.clone(),system_program.clone()])?;

    let ix = instruction::create_account(temp_account.key, &pda_imu, required_lamports, space, program_id);

    invoke_signed(
        &ix,
        &[temp_account.clone(),imutiable_registry_account.clone(),system_program.clone()],
        &[&[
            b"IMUTABLE",
            data.item_id.as_ref(),
            buyer.key.as_ref(),
            seller.key.as_ref(),
            &[_bump_imu],
        ]],
    )?;
    let binding = &mut post_account.data.borrow();
    let post_account_data = bytemuck::from_bytes::<InitAccountData>(&binding);

    let mut binding = &mut imutiable_registry_account.data.borrow_mut();
    let data_mut = bytemuck::from_bytes_mut::<ImmutableRegistryData>(&mut binding);

    data_mut.buyer = data.buyer;
    data_mut.item_id = data.item_id;
    data_mut.seller = post_account_data.payer;
    data_mut.description = post_account_data.description;
    data_mut.timestamp = solana_program::sysvar::clock::Clock::get()?.unix_timestamp as u64;
    data_mut.title = post_account_data.title;
    data_mut.price = post_account_data.price;


    **seller.try_borrow_mut_lamports()? = seller
        .lamports()
        .checked_add(holder_account.lamports())
        .ok_or(ProgramError::InvalidArgument)?;
    **holder_account.try_borrow_mut_lamports()? = 0;

    **buyer.try_borrow_mut_lamports()? = buyer
        .lamports()
        .checked_add(buy_account.lamports())
        .ok_or(ProgramError::InvalidArgument)?;
    **buy_account.try_borrow_mut_lamports()? = 0;

    let mut data = buy_account.try_borrow_mut_data()?;
    data.fill(0);
    msg!("Sell completed successfully");

    Ok(())
}

fn temp_money_holder(
    program_id: &Pubkey,
    accounts : &[AccountInfo], //[seller,payer, holder_account , system_program]
    data : &TempMoneyHolder
)-> ProgramResult{
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let holder_account = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    if !payer.is_signer{
        msg!("Error: payer must be a signer");
        return Err(ProgramError::InvalidInstructionData)
    }

    let (pda , bump ) = Pubkey::find_program_address(&[b"TEMP",data.buyer.as_ref(), data.seller.as_ref() , data.title.as_ref()], program_id);

    if *holder_account.key != pda{
        msg!("Err: holder account pda dont match provide correct one ");
        return Err(ProgramError::InvalidInstructionData)
    }
    let rent = Rent::get()?;
    let min_lamp = rent.minimum_balance(0);
    let ix = instruction::create_account(
        payer.key, 
        &pda, 
        min_lamp, 
        0, 
        program_id
    );

    invoke_signed(
        &ix, 
        &[payer.clone(),holder_account.clone(), system_program.clone()], 
        &[&[b"TEMP",data.buyer.as_ref(), data.seller.as_ref() , data.title.as_ref() , &[bump]]])?;

    msg!("succesfully temp_money_holder created!!!!!");

    Ok(())

}


fn money_holder(
    program_id: &Pubkey,
    accounts : &[AccountInfo],
    data : &MoneyHolder
)-> ProgramResult{
    let account_iter = &mut accounts.iter();
    let payer = next_account_info(account_iter)?;
    let holder_account = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;

    if !payer.is_signer{
        msg!("Error: payer must be a signer");
        return Err(ProgramError::InvalidInstructionData)
    }

    let (pda , bump ) = Pubkey::find_program_address(&[b"HOLDER", data.title.as_ref()], program_id);

    if *holder_account.key != pda{
        msg!("Err: holder account pda dont match provide correct one ");
        return Err(ProgramError::InvalidInstructionData)
    }

    let rent = Rent::get()?;
    let min_lamp = rent.minimum_balance(0);
    let ix = instruction::create_account(
        payer.key, 
        &pda, 
        min_lamp, 
        0, 
        program_id
    );

    invoke_signed(
        &ix, 
        &[payer.clone(),holder_account.clone(), system_program.clone()], 
        &[&[b"TEMP", data.title.as_ref() , &[bump]]])?;

    msg!("succesfully money_holder_account created!!!!!");

    Ok(())
}


fn info_account(
    program_id: &Pubkey,
    accounts : &[AccountInfo], //[seller , buyer]
    data : &Info
) -> ProgramResult{
    let account_iter = &mut accounts.iter();
    let seller = next_account_info(account_iter)?;
    let buyer = next_account_info(account_iter)?;
    let mut_account = next_account_info(account_iter)?;
    let system_program = next_account_info(account_iter)?;


    if !seller.is_signer && !buyer.is_signer{
        msg!("Error : Either seller or buyer or even both may be not signer so plz provide valid signer");
        return Err(ProgramError::InvalidInstructionData)
    }

    let (pda , bump) = Pubkey::find_program_address(&[b"IMUTABLE",data.title.as_ref(), buyer.key.as_ref(), seller.key.as_ref()], program_id);

    if *mut_account.key != pda{
        msg!("Error: the pda of an IMUTABLE account is wrong ");
        return Err(ProgramError::InvalidInstructionData)
    }

    let rent = Rent::get()?;
    let size = std::mem::size_of::<ImmutableRegistryData>() as u64;
    let min_lamp = rent.minimum_balance(size as usize);

    let ix = instruction::create_account(
        buyer.key,
         &pda,
          min_lamp, 
          size, 
          program_id
        );

    invoke_signed(
        &ix, 
        &[
            buyer.clone(),
            mut_account.clone(),
            system_program.clone()
        ], 
        &[&[
            b"IMUTABLE",
            data.title.as_ref(),
            seller.key.as_ref(),
            &[bump]
        ]]
    )?;

    Ok(())
}


